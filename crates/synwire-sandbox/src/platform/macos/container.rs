//! macOS container runtime detection and spawning.
//!
//! Detection order (first found wins):
//!
//! 1. **Apple Container** (macOS 26+, Apple Silicon) — first-party,
//!    Virtualization.framework-backed Linux VMs.
//! 2. **Docker Desktop** — most common macOS container runtime.
//! 3. **Podman** — rootless alternative to Docker.
//! 4. **Colima** — lightweight Docker-compatible runtime using Lima.
//!
//! Docker Desktop, Podman, and Colima all use the same `docker run` /
//! `podman run` CLI semantics for spawning containers. Apple Container
//! uses its own `container run` syntax.

use tokio::process::Command;
use tracing::{debug, info};

use synwire_core::agents::sandbox::SandboxConfig;

use crate::SandboxError;

// ── ContainerRuntime ──────────────────────────────────────────────────────────

/// Available container runtimes on the current macOS system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ContainerRuntime {
    /// Apple Container (macOS 26+, Apple Silicon).
    ///
    /// Uses Virtualization.framework for lightweight Linux VMs.
    /// First-party Apple support, OCI-compatible images.
    AppleContainer,
    /// Docker Desktop — the most widely installed macOS container runtime.
    ///
    /// Runs a Linux VM via Virtualization.framework (Apple Silicon) or
    /// HyperKit (Intel). Requires the Docker Desktop application to be
    /// running.
    Docker,
    /// Podman — rootless container runtime, no daemon required.
    Podman,
    /// Colima — lightweight Docker-compatible runtime backed by Lima.
    ///
    /// Provides a `docker` CLI that talks to a Lima-managed Linux VM.
    /// No Docker Desktop license required.
    Colima,
}

impl ContainerRuntime {
    /// Human-readable name for logging.
    fn display_name(self) -> &'static str {
        match self {
            Self::AppleContainer => "Apple Container",
            Self::Docker => "Docker Desktop",
            Self::Podman => "Podman",
            Self::Colima => "Colima",
        }
    }

    /// CLI binary to probe for availability.
    fn probe_binary(self) -> &'static str {
        match self {
            Self::AppleContainer => "container",
            Self::Docker => "docker",
            Self::Podman => "podman",
            Self::Colima => "colima",
        }
    }

    /// CLI binary used for `run` commands.
    ///
    /// Colima provides a `docker` CLI, so we use `docker` for both
    /// Docker Desktop and Colima once detected.
    fn run_binary(self) -> &'static str {
        match self {
            Self::AppleContainer => "container",
            Self::Docker | Self::Colima => "docker",
            Self::Podman => "podman",
        }
    }
}

/// Probe `$PATH` for a supported container runtime.
///
/// Returns the first runtime found in preference order:
/// Apple Container > Docker Desktop > Podman > Colima.
///
/// Returns `None` if no runtime is available.
pub async fn detect_container_runtime() -> Option<ContainerRuntime> {
    // Apple Container: first-party, preferred on macOS 26+.
    if binary_responds("container").await {
        info!(
            runtime = "Apple Container",
            "detected macOS container runtime"
        );
        return Some(ContainerRuntime::AppleContainer);
    }

    // Docker Desktop: check that the daemon is actually running, not just
    // that the binary exists (docker CLI is useless without dockerd).
    if docker_daemon_running().await {
        info!(
            runtime = "Docker Desktop",
            "detected macOS container runtime"
        );
        return Some(ContainerRuntime::Docker);
    }

    // Podman: rootless, no daemon needed — just check the binary.
    if binary_responds("podman").await {
        info!(runtime = "Podman", "detected macOS container runtime");
        return Some(ContainerRuntime::Podman);
    }

    // Colima: check that both `colima` and `docker` binaries exist, and
    // that Colima's VM is running (colima status).
    if colima_running().await {
        info!(runtime = "Colima", "detected macOS container runtime");
        return Some(ContainerRuntime::Colima);
    }

    debug!("no macOS container runtime found (tried: container, docker, podman, colima)");
    None
}

// ── Unified spawn ─────────────────────────────────────────────────────────────

/// Spawn a command using the detected container runtime.
///
/// Dispatches to the appropriate runtime-specific spawn function.
/// All runtimes receive the same [`SandboxConfig`] and translate it into
/// their native CLI flags.
///
/// # Errors
///
/// Returns [`SandboxError::InitFailed`] if the runtime cannot be spawned.
pub async fn spawn_with_runtime(
    runtime: ContainerRuntime,
    config: &SandboxConfig,
    image: &str,
    command: &str,
    args: &[String],
) -> Result<tokio::process::Child, SandboxError> {
    match runtime {
        ContainerRuntime::AppleContainer => {
            spawn_apple_container(config, image, command, args).await
        }
        ContainerRuntime::Docker | ContainerRuntime::Colima => {
            spawn_docker_compatible(runtime, config, image, command, args).await
        }
        ContainerRuntime::Podman => {
            spawn_docker_compatible(runtime, config, image, command, args).await
        }
    }
}

// ── Apple Container ───────────────────────────────────────────────────────────

/// Spawn via `container run` (Apple Container).
async fn spawn_apple_container(
    config: &SandboxConfig,
    image: &str,
    command: &str,
    args: &[String],
) -> Result<tokio::process::Child, SandboxError> {
    let mut cmd = Command::new("container");
    let _c = cmd.arg("run").arg("--rm");

    if let Some(fs) = &config.filesystem {
        for path in &fs.allow_write {
            let _c = cmd
                .arg("--mount")
                .arg(format!("type=bind,src={path},dst={path}"));
        }
        for path in &fs.deny_write {
            let _c = cmd
                .arg("--mount")
                .arg(format!("type=bind,src={path},dst={path},readonly"));
        }
    }

    let network_enabled = config.network.as_ref().map(|n| n.enabled).unwrap_or(false);
    if !network_enabled {
        let _c = cmd.arg("--network").arg("none");
    }

    apply_resource_flags(&mut cmd, config);
    apply_security_flags(&mut cmd, config);
    apply_env_flags(&mut cmd, config);

    let _c = cmd.arg(image).arg(command);
    for arg in args {
        let _c = cmd.arg(arg);
    }

    cmd.kill_on_drop(true)
        .spawn()
        .map_err(|e| SandboxError::InitFailed {
            reason: format!("Apple Container spawn failed: {e}"),
        })
}

// ── Docker / Podman / Colima ─────────────────────────────────────────────────

/// Spawn via `docker run` or `podman run` (shared CLI semantics).
///
/// Docker Desktop, Podman, and Colima all accept the same `run` flags:
/// `--volume`, `--network`, `--memory`, `--cpus`, `--user`, `--env`, etc.
async fn spawn_docker_compatible(
    runtime: ContainerRuntime,
    config: &SandboxConfig,
    image: &str,
    command: &str,
    args: &[String],
) -> Result<tokio::process::Child, SandboxError> {
    let binary = runtime.run_binary();
    let mut cmd = Command::new(binary);
    let _c = cmd.arg("run").arg("--rm").arg("--interactive");

    // Filesystem mounts.
    if let Some(fs) = &config.filesystem {
        for path in &fs.allow_write {
            let _c = cmd.arg("--volume").arg(format!("{path}:{path}:rw"));
        }
        for path in &fs.deny_write {
            let _c = cmd.arg("--volume").arg(format!("{path}:{path}:ro"));
        }
    }

    // Network.
    let network_enabled = config.network.as_ref().map(|n| n.enabled).unwrap_or(false);
    if !network_enabled {
        let _c = cmd.arg("--network").arg("none");
    }

    apply_resource_flags(&mut cmd, config);

    // User mapping.
    if let (Some(uid), Some(gid)) = (config.security.run_as_user, config.security.run_as_group) {
        let _c = cmd.arg("--user").arg(format!("{uid}:{gid}"));
    }

    apply_security_flags(&mut cmd, config);
    apply_env_flags(&mut cmd, config);

    let _c = cmd.arg(image).arg(command);
    for arg in args {
        let _c = cmd.arg(arg);
    }

    cmd.kill_on_drop(true)
        .spawn()
        .map_err(|e| SandboxError::InitFailed {
            reason: format!("{} spawn failed: {e}", runtime.display_name()),
        })
}

// ── shared helpers ────────────────────────────────────────────────────────────

/// Apply `--memory` and `--cpus` flags from resource limits.
fn apply_resource_flags(cmd: &mut Command, config: &SandboxConfig) {
    if let Some(limits) = &config.resources {
        if let Some(mem) = limits.memory_bytes {
            let _c = cmd.arg("--memory").arg(mem.to_string());
        }
        if let Some(cpu) = limits.cpu_quota {
            let _c = cmd.arg("--cpus").arg(format!("{cpu:.2}"));
        }
    }
}

/// Apply security flags (`--security-opt no-new-privileges`).
fn apply_security_flags(cmd: &mut Command, config: &SandboxConfig) {
    if config.security.no_new_privileges {
        let _c = cmd.arg("--security-opt").arg("no-new-privileges");
    }
}

/// Apply environment variable flags (`--env KEY=val`).
fn apply_env_flags(cmd: &mut Command, config: &SandboxConfig) {
    if config.env.inherit_parent {
        for (k, v) in std::env::vars() {
            if !config.env.unset.contains(&k) {
                let _c = cmd.arg("--env").arg(format!("{k}={v}"));
            }
        }
    }
    for (k, v) in &config.env.set {
        let _c = cmd.arg("--env").arg(format!("{k}={v}"));
    }
}

/// Return `true` if `binary` is found in `$PATH` and responds to `--version`.
async fn binary_responds(binary: &str) -> bool {
    Command::new(binary)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Check if the Docker daemon is actually running (not just the CLI binary).
///
/// `docker version` fails if the daemon is not reachable, while
/// `docker --version` succeeds as long as the binary exists.
async fn docker_daemon_running() -> bool {
    Command::new("docker")
        .arg("version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Check if Colima is installed and its VM is running.
///
/// `colima status` exits 0 when the VM is running, non-zero otherwise.
async fn colima_running() -> bool {
    Command::new("colima")
        .arg("status")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}
