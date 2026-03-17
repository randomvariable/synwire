#![allow(
    clippy::similar_names,
    clippy::match_same_arms,
    clippy::option_if_let_else,
    clippy::map_unwrap_or,
    clippy::manual_unwrap_or_default
)]
//! Linux namespace container via an OCI runtime (runc or crun).
//!
//! [`NamespaceContainer`] locates an OCI-compliant container runtime on
//! `$PATH`, generates an [OCI runtime spec][oci] from a [`ContainerConfig`],
//! and spawns the container with `<runtime> run`.
//!
//! [oci]: https://github.com/opencontainers/runtime-spec
//!
//! # Non-interactive mode
//!
//! [`NamespaceContainer::spawn`] runs the container in the foreground.
//! stdout/stderr of the runtime process are the container's output.
//!
//! # Captured output mode
//!
//! [`NamespaceContainer::spawn_captured`] redirects stdout/stderr to files in
//! a temporary directory. Output persists even if the process is killed.
//!
//! # Interactive / PTY mode
//!
//! [`NamespaceContainer::spawn_interactive`] uses the runtime's
//! `--console-socket` mechanism to receive a PTY controller fd from the
//! runtime. Stage 2 of the runtime sets up the controlling terminal inside
//! the container.

use std::os::fd::OwnedFd;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use oci_spec::runtime::{
    Arch, Capability, LinuxBuilder, LinuxCapabilitiesBuilder, LinuxIdMappingBuilder,
    LinuxNamespaceBuilder, LinuxNamespaceType, LinuxSeccompAction, LinuxSeccompBuilder,
    LinuxSyscallBuilder, Mount, MountBuilder, ProcessBuilder, RootBuilder, Spec, SpecBuilder,
    UserBuilder,
};
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tracing::{debug, warn};

use crate::SandboxError;
use crate::output::{CapturedOutput, OutputMode, ProcessCapture};

// ── Config types ──────────────────────────────────────────────────────────────

/// Clone flags requested for the namespace container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum CloneFlag {
    /// New PID namespace (`CLONE_NEWPID`).
    NewPid,
    /// New UTS namespace — isolates hostname/domainname (`CLONE_NEWUTS`).
    NewUts,
    /// New IPC namespace (`CLONE_NEWIPC`).
    NewIpc,
    /// New mount namespace (`CLONE_NEWNS`).
    NewNs,
    /// New cgroup namespace (`CLONE_NEWCGROUP`).
    NewCgroup,
    /// New network namespace (`CLONE_NEWNET`).
    NewNet,
    /// New user namespace (`CLONE_NEWUSER`). Attempted; silently skipped on
    /// kernels or system configs that prohibit unprivileged user namespaces.
    NewUser,
}

/// A single bind mount to set up inside the container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindMount {
    /// Host source path.
    pub source: String,
    /// Container target path.
    pub target: String,
    /// Mount read-only.
    pub read_only: bool,
}

/// Security parameters for the container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerSecurity {
    /// Seccomp profile to apply.
    pub seccomp: ContainerSeccomp,
    /// Capabilities to drop (e.g. `["ALL"]`).
    pub capabilities_drop: Vec<String>,
    /// Capabilities to add after dropping.
    pub capabilities_add: Vec<String>,
    /// Set `PR_SET_NO_NEW_PRIVS` before exec.
    pub no_new_privileges: bool,
    /// Run as this UID (None = inherit).
    pub run_as_user: Option<u32>,
    /// Run as this GID (None = inherit).
    pub run_as_group: Option<u32>,
}

/// Seccomp profile selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ContainerSeccomp {
    /// No seccomp filter.
    Unconfined,
    /// Built-in `RuntimeDefault` profile (deny-list of ~18 dangerous syscalls).
    RuntimeDefault,
    /// Load profile from a JSON file path.
    Localhost {
        /// Path to the OCI-format seccomp profile.
        path: String,
    },
}

/// Container configuration — translated to an OCI runtime spec before launch.
///
/// Use [`NamespaceContainer::build_config`] to derive this from a high-level
/// [`SandboxConfig`](synwire_core::agents::sandbox::SandboxConfig).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerConfig {
    /// Namespace flags to apply.
    pub clone_flags: Vec<CloneFlag>,
    /// Isolate the network namespace.
    pub network_isolation: bool,
    /// Request a user namespace mapping (rootless containers).
    pub user_namespace: bool,
    /// Request a cgroup namespace.
    pub cgroup_namespace: bool,
    /// Bind mounts to create inside the container.
    pub bind_mounts: Vec<BindMount>,
    /// Path to the agent's cgroup (used for resource limits inside the ns).
    pub cgroup_path: Option<String>,
    /// Security parameters.
    pub security: ContainerSecurity,
    /// Command to exec inside the container.
    pub command: String,
    /// Arguments for the command.
    pub args: Vec<String>,
    /// Environment variables (complete set — parent env is not inherited
    /// inside the namespace unless explicitly passed).
    pub env: std::collections::HashMap<String, String>,
}

// ── PtySession ──────────────────────────────────────────────────────────────

/// Handle to an interactive PTY session running inside a namespace container.
///
/// `controller` is the host-side controller end of the PTY. Read from it to
/// receive output; write to it to send input to the contained process. Wrap
/// it in [`tokio::io::unix::AsyncFd`] for non-blocking async I/O.
///
/// `child` is the OCI runtime process. Killing the runtime kills the
/// container.
#[derive(Debug)]
pub struct PtySession {
    /// Controller end of the PTY (host side).
    pub controller: OwnedFd,
    /// The OCI runtime child process.
    pub child: tokio::process::Child,
    /// Bundle directory — kept alive while the container runs.
    _bundle: tempfile::TempDir,
}

// ── ContainerProcess ────────────────────────────────────────────────────────

/// A running non-interactive container process.
///
/// Holds the OCI runtime child process and the bundle directory. The bundle
/// is automatically cleaned up when this handle is dropped.
#[derive(Debug)]
pub struct ContainerProcess {
    /// The OCI runtime child process.
    pub child: tokio::process::Child,
    /// Bundle directory — kept alive while the container runs.
    _bundle: tempfile::TempDir,
}

// ── OCI runtime selection ─────────────────────────────────────────────────

/// Which OCI runtime backend to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum OciRuntime {
    /// Standard runc — Linux namespaces + seccomp + capabilities.
    ///
    /// Processes share the host kernel. Isolation relies on kernel
    /// namespace boundaries.
    Runc,
    /// gVisor (runsc) — user-space kernel sandbox.
    ///
    /// Processes run on a Go-based kernel that intercepts syscalls,
    /// providing a much stronger isolation boundary than namespaces
    /// alone. Requires `runsc` on `$PATH`.
    Gvisor,
}

/// Which gVisor platform to use for syscall interception.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum GvisorPlatform {
    /// Systrap — patches syscall instruction sites. Fastest, but requires
    /// `CAP_SYS_PTRACE` (broken in rootless + host-network mode due to a
    /// gVisor bug in `ConfigureCmdForRootless`).
    Systrap,
    /// Ptrace — uses `PTRACE_SYSEMU` / `CLONE_PTRACE`. Slower but
    /// universally compatible. Same isolation guarantees as systrap.
    Ptrace,
}

/// Process-wide cache: once we discover that systrap fails for gVisor,
/// all subsequent containers skip the probe and go straight to ptrace.
///
/// States: 0 = not probed, 1 = systrap works, 2 = ptrace fallback.
static GVISOR_PLATFORM_CACHE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

const _PLATFORM_NOT_PROBED: u8 = 0;
const PLATFORM_SYSTRAP: u8 = 1;
const PLATFORM_PTRACE: u8 = 2;

// ── NamespaceContainer ─────────────────────────────────────────────────────

/// Spawns processes inside Linux containers via an OCI runtime.
///
/// Supports [`OciRuntime::Runc`] (standard namespace isolation) and
/// [`OciRuntime::Gvisor`] (user-space kernel via `runsc`).
///
/// For gVisor, the constructor probes whether the `systrap` platform works
/// (it requires `CAP_SYS_PTRACE` which is missing in rootless + host-network
/// mode due to a gVisor bug). If systrap fails, it falls back to `ptrace`
/// and caches the result for the lifetime of the process — all subsequent
/// gVisor containers skip the probe.
#[derive(Debug)]
pub struct NamespaceContainer {
    /// Path to the OCI runtime binary.
    runtime_path: PathBuf,
    /// Which runtime backend is in use.
    runtime_kind: OciRuntime,
    /// For gVisor: which platform to use (systrap or ptrace).
    gvisor_platform: GvisorPlatform,
}

impl NamespaceContainer {
    /// Create a container using `runc` from `$PATH`.
    ///
    /// # Errors
    ///
    /// Returns [`SandboxError::RuntimeNotFound`] if `runc` is not on `$PATH`.
    pub fn new() -> Result<Self, SandboxError> {
        Self::with_runtime(OciRuntime::Runc)
    }

    /// Create a container using gVisor (`runsc`) from `$PATH`.
    ///
    /// On first call, probes the `systrap` platform by running a trivial
    /// container. If systrap works, uses it for all future containers
    /// (fastest). If it fails (e.g., missing `CAP_SYS_PTRACE` in rootless
    /// mode), falls back to `ptrace` and logs a warning. The result is
    /// cached process-wide — subsequent calls skip the probe.
    ///
    /// # Errors
    ///
    /// Returns [`SandboxError::RuntimeNotFound`] if `runsc` is not on `$PATH`.
    pub fn with_gvisor() -> Result<Self, SandboxError> {
        Self::with_runtime(OciRuntime::Gvisor)
    }

    /// Create a container using the specified OCI runtime.
    ///
    /// # Errors
    ///
    /// Returns [`SandboxError::RuntimeNotFound`] if the runtime binary is
    /// not on `$PATH`.
    pub fn with_runtime(kind: OciRuntime) -> Result<Self, SandboxError> {
        let name = match kind {
            OciRuntime::Runc => "runc",
            OciRuntime::Gvisor => "runsc",
        };
        let path =
            which_binary(name).map_err(|()| SandboxError::RuntimeNotFound { name: name.into() })?;
        debug!(runtime = name, path = %path.display(), "found OCI runtime");

        let gvisor_platform = if kind == OciRuntime::Gvisor {
            resolve_gvisor_platform(&path)
        } else {
            GvisorPlatform::Systrap // unused for runc
        };

        Ok(Self {
            runtime_path: path,
            runtime_kind: kind,
            gvisor_platform,
        })
    }

    /// Spawn a command inside a namespace container.
    ///
    /// Creates a temporary OCI bundle, generates a runtime spec from `config`,
    /// and runs the container in the foreground. Returns a
    /// [`ContainerProcess`] that holds the runtime child and the bundle dir.
    ///
    /// # Errors
    ///
    /// Returns a [`SandboxError`] if bundle creation or process spawn fails.
    pub fn spawn(&self, config: &ContainerConfig) -> Result<ContainerProcess, SandboxError> {
        let (bundle, container_id) = self.prepare_bundle(config, false)?;

        debug!(runtime = %self.runtime_path.display(), %container_id, "spawning namespace container");

        let child = self
            .build_run_command(&bundle, &container_id, None)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| SandboxError::RuntimeFailed {
                reason: format!("spawn failed: {e}"),
            })?;

        Ok(ContainerProcess {
            child,
            _bundle: bundle,
        })
    }

    /// Spawn a command inside a namespace container with output captured to
    /// files in a temporary directory.
    ///
    /// stdout and stderr are redirected to files rather than pipes. The files
    /// persist even if the process is killed, and the temporary directory is
    /// deleted when the last `Arc<CapturedOutput>` reference is dropped.
    ///
    /// # Errors
    ///
    /// Returns a [`SandboxError`] if directory creation, file opening, or
    /// process spawn fails.
    pub fn spawn_captured(
        &self,
        config: &ContainerConfig,
        mode: OutputMode,
    ) -> Result<ProcessCapture, SandboxError> {
        let output = CapturedOutput::new(mode).map_err(|e| SandboxError::RuntimeFailed {
            reason: format!("create capture directory: {e}"),
        })?;

        let stdout_file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(output.stdout_path())
            .map_err(|e| SandboxError::RuntimeFailed {
                reason: format!("open stdout capture file: {e}"),
            })?;

        let stderr_file = match mode {
            OutputMode::Combined => {
                stdout_file
                    .try_clone()
                    .map_err(|e| SandboxError::RuntimeFailed {
                        reason: format!("clone stdout handle for combined stderr: {e}"),
                    })?
            }
            OutputMode::Separate => {
                let stderr_path =
                    output
                        .stderr_path()
                        .ok_or_else(|| SandboxError::RuntimeFailed {
                            reason: "separate mode missing stderr path".into(),
                        })?;
                std::fs::OpenOptions::new()
                    .create(true)
                    .truncate(true)
                    .write(true)
                    .open(stderr_path)
                    .map_err(|e| SandboxError::RuntimeFailed {
                        reason: format!("open stderr capture file: {e}"),
                    })?
            }
        };

        let (bundle, container_id) = self.prepare_bundle(config, false)?;

        debug!(runtime = %self.runtime_path.display(), %container_id, "spawning captured namespace container");

        let child = self
            .build_run_command(&bundle, &container_id, None)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::from(stdout_file))
            .stderr(std::process::Stdio::from(stderr_file))
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| SandboxError::RuntimeFailed {
                reason: format!("spawn failed: {e}"),
            })?;

        Ok(ProcessCapture {
            output: Arc::new(output),
            child,
            _bundle: Some(bundle),
        })
    }

    /// Spawn a command inside a namespace container with full PTY support
    /// for human-in-the-loop interaction.
    ///
    /// Uses the OCI runtime's `--console-socket` mechanism: the runtime
    /// creates a PTY inside the container and sends the controller fd back
    /// over a Unix socket via `SCM_RIGHTS`. The returned [`PtySession`]
    /// contains the controller fd for host-side I/O.
    ///
    /// # Errors
    ///
    /// Returns a [`SandboxError`] if socket setup, process spawn, or PTY
    /// fd handshake fails.
    pub fn spawn_interactive(&self, config: &ContainerConfig) -> Result<PtySession, SandboxError> {
        let (bundle, container_id) = self.prepare_bundle(config, true)?;
        let socket_path = bundle.path().join("console.sock");

        let listener = std::os::unix::net::UnixListener::bind(&socket_path).map_err(|e| {
            SandboxError::RuntimeFailed {
                reason: format!("bind console socket: {e}"),
            }
        })?;

        debug!(runtime = %self.runtime_path.display(), %container_id, "spawning interactive namespace container");

        let child = self
            .build_run_command(&bundle, &container_id, Some(&socket_path))
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| SandboxError::RuntimeFailed {
                reason: format!("spawn failed: {e}"),
            })?;

        // The runtime connects to our socket and sends the PTY controller
        // fd via SCM_RIGHTS.
        let (stream, _) = listener.accept().map_err(|e| SandboxError::RuntimeFailed {
            reason: format!("accept console socket: {e}"),
        })?;

        let controller = recv_pty_controller(&stream)?;

        Ok(PtySession {
            controller,
            child,
            _bundle: bundle,
        })
    }

    /// Build a [`ContainerConfig`] from synwire-core's `SandboxConfig`.
    ///
    /// Derives namespace flags, bind mounts, and security parameters from the
    /// high-level configuration.
    #[must_use]
    pub fn build_config(
        sandbox: &synwire_core::agents::sandbox::SandboxConfig,
        command: impl Into<String>,
        args: Vec<String>,
    ) -> ContainerConfig {
        use synwire_core::agents::sandbox::SeccompProfile;

        let network_enabled = sandbox.network.as_ref().is_some_and(|n| n.enabled);

        let mut clone_flags = vec![
            CloneFlag::NewUts,
            CloneFlag::NewIpc,
            CloneFlag::NewNs,
            CloneFlag::NewCgroup,
            CloneFlag::NewPid,
        ];
        if !network_enabled {
            clone_flags.push(CloneFlag::NewNet);
        }

        // Build bind mounts from filesystem config.
        let bind_mounts = sandbox
            .filesystem
            .as_ref()
            .map(|fs| {
                let mut mounts: Vec<BindMount> = fs
                    .allow_write
                    .iter()
                    .filter_map(|p| {
                        let abs = to_absolute(p)?;
                        Some(BindMount {
                            source: abs.clone(),
                            target: abs,
                            read_only: false,
                        })
                    })
                    .collect();
                if fs.inherit_readable {
                    mounts.push(BindMount {
                        source: "/".into(),
                        target: "/".into(),
                        read_only: true,
                    });
                }
                mounts
            })
            .unwrap_or_default();

        // Build environment.
        let mut env: std::collections::HashMap<String, String> = if sandbox.env.inherit_parent {
            std::env::vars().collect()
        } else {
            std::collections::HashMap::new()
        };
        for k in &sandbox.env.unset {
            let _ = env.remove(k);
        }
        env.extend(sandbox.env.set.clone());

        // Security.
        let seccomp = match &sandbox.security.seccomp {
            SeccompProfile::Unconfined => ContainerSeccomp::Unconfined,
            SeccompProfile::Localhost { path } => {
                ContainerSeccomp::Localhost { path: path.clone() }
            }
            SeccompProfile::RuntimeDefault | _ => ContainerSeccomp::RuntimeDefault,
        };

        let security = ContainerSecurity {
            seccomp,
            capabilities_drop: sandbox.security.capabilities.drop.clone(),
            capabilities_add: sandbox.security.capabilities.add.clone(),
            no_new_privileges: sandbox.security.no_new_privileges,
            run_as_user: sandbox.security.run_as_user,
            run_as_group: sandbox.security.run_as_group,
        };

        ContainerConfig {
            clone_flags,
            network_isolation: !network_enabled,
            user_namespace: true,
            cgroup_namespace: true,
            bind_mounts,
            cgroup_path: None,
            security,
            command: command.into(),
            args,
            env,
        }
    }

    // ── internal ──────────────────────────────────────────────────────────

    /// Build a `Command` for `<runtime> run` with runtime-specific flags.
    ///
    /// gVisor (`runsc`) needs `--rootless` and `--network=none` (when network
    /// isolation is requested) since it manages namespaces internally.
    fn build_run_command(
        &self,
        bundle: &tempfile::TempDir,
        container_id: &str,
        console_socket: Option<&Path>,
    ) -> Command {
        let mut cmd = Command::new(&self.runtime_path);

        // gVisor-specific global flags (before the subcommand).
        if self.runtime_kind == OciRuntime::Gvisor {
            let platform_flag = match self.gvisor_platform {
                GvisorPlatform::Systrap => "--platform=systrap",
                GvisorPlatform::Ptrace => "--platform=ptrace",
            };
            let _cmd = cmd
                .arg("--rootless")
                .arg("--network=host")
                .arg(platform_flag);
        }

        let _cmd = cmd.arg("run");

        if let Some(sock) = console_socket {
            let _cmd = cmd.arg("--console-socket").arg(sock);
        }

        let _cmd = cmd.arg("--bundle").arg(bundle.path()).arg(container_id);

        cmd
    }

    /// Create a temporary OCI bundle directory with `config.json` and `rootfs/`.
    fn prepare_bundle(
        &self,
        config: &ContainerConfig,
        terminal: bool,
    ) -> Result<(tempfile::TempDir, String), SandboxError> {
        let bundle = tempfile::TempDir::with_prefix("synwire-").map_err(|e| {
            SandboxError::RuntimeFailed {
                reason: format!("create bundle dir: {e}"),
            }
        })?;
        let rootfs = bundle.path().join("rootfs");
        let container_id = uuid::Uuid::new_v4().to_string();

        // Generate /etc/passwd and /etc/group so the current user is
        // resolvable inside the container (whoami, id, ls -la all work).
        let passwd_path = bundle.path().join("passwd");
        let group_path = bundle.path().join("group");
        generate_user_files(&passwd_path, &group_path).map_err(|e| {
            SandboxError::RuntimeFailed {
                reason: format!("generate user files: {e}"),
            }
        })?;

        let spec = build_oci_spec(
            config,
            terminal,
            &passwd_path,
            &group_path,
            self.runtime_kind,
        )
        .map_err(|e| SandboxError::RuntimeFailed {
            reason: format!("build OCI spec: {e}"),
        })?;

        // Create mount-point directories inside rootfs.
        prepare_rootfs(&rootfs, &spec).map_err(|e| SandboxError::RuntimeFailed {
            reason: format!("prepare rootfs: {e}"),
        })?;

        let spec_json = serde_json::to_string_pretty(&spec).map_err(SandboxError::SerdeError)?;
        std::fs::write(bundle.path().join("config.json"), spec_json).map_err(|e| {
            SandboxError::RuntimeFailed {
                reason: format!("write config.json: {e}"),
            }
        })?;

        Ok((bundle, container_id))
    }
}

// ── OCI spec generation ───────────────────────────────────────────────────────

/// Convert a capability name string (e.g. `"KILL"`, `"CAP_KILL"`) to a
/// [`Capability`] enum variant. Returns `None` for unrecognised names.
fn parse_capability(name: &str) -> Option<Capability> {
    let canon = format!("CAP_{}", name.trim_start_matches("CAP_"));
    // Capability implements Deserialize which handles the "CAP_*" format.
    serde_json::from_value(serde_json::Value::String(canon)).ok()
}

/// Build an OCI runtime spec from a [`ContainerConfig`].
#[allow(clippy::too_many_lines)]
fn build_oci_spec(
    config: &ContainerConfig,
    terminal: bool,
    passwd_path: &Path,
    group_path: &Path,
    runtime: OciRuntime,
) -> Result<Spec, oci_spec::OciSpecError> {
    let uid = nix::unistd::getuid().as_raw();
    let gid = nix::unistd::getgid().as_raw();

    // Build process.args: [command, ...args]
    let mut args = vec![config.command.clone()];
    args.extend(config.args.clone());

    // Build process.env: ["KEY=val", ...]
    let env: Vec<String> = config.env.iter().map(|(k, v)| format!("{k}={v}")).collect();

    // Build linux.namespaces
    let mut namespaces = Vec::new();
    for flag in &config.clone_flags {
        let ns_type = match flag {
            CloneFlag::NewPid => LinuxNamespaceType::Pid,
            CloneFlag::NewUts => LinuxNamespaceType::Uts,
            CloneFlag::NewIpc => LinuxNamespaceType::Ipc,
            CloneFlag::NewNs => LinuxNamespaceType::Mount,
            CloneFlag::NewCgroup => LinuxNamespaceType::Cgroup,
            CloneFlag::NewNet => LinuxNamespaceType::Network,
            CloneFlag::NewUser => continue, // handled separately below
        };
        namespaces.push(LinuxNamespaceBuilder::default().typ(ns_type).build()?);
    }
    // gVisor manages its own user namespace internally — don't request one
    // in the OCI spec or it will conflict with runsc's sandbox model.
    if config.user_namespace && runtime != OciRuntime::Gvisor {
        namespaces.push(
            LinuxNamespaceBuilder::default()
                .typ(LinuxNamespaceType::User)
                .build()?,
        );
    }

    // Build mounts
    let mut mounts = essential_mounts()?;
    for bm in &config.bind_mounts {
        let mut opts = vec!["rbind".to_string()];
        if bm.read_only {
            opts.push("ro".to_string());
        }
        mounts.push(
            MountBuilder::default()
                .destination(&bm.target)
                .typ("bind")
                .source(&bm.source)
                .options(opts)
                .build()?,
        );
    }
    // If no explicit mounts but user wants host fs, add key dirs.
    if config.bind_mounts.is_empty() {
        for dir in &[
            "/usr", "/bin", "/sbin", "/lib", "/lib64", "/etc", "/home", "/tmp",
        ] {
            if Path::new(dir).exists() {
                mounts.push(
                    MountBuilder::default()
                        .destination(*dir)
                        .typ("bind")
                        .source(*dir)
                        .options(vec!["rbind".into(), "ro".into()])
                        .build()?,
                );
            }
        }
    }

    // Overlay /etc/passwd and /etc/group with generated files so the
    // current user is resolvable inside the container. These are added
    // AFTER the /etc bind mount so they take precedence.
    mounts.push(
        MountBuilder::default()
            .destination("/etc/passwd")
            .typ("bind")
            .source(passwd_path)
            .options(vec!["bind".into(), "ro".into()])
            .build()?,
    );
    mounts.push(
        MountBuilder::default()
            .destination("/etc/group")
            .typ("bind")
            .source(group_path)
            .options(vec!["bind".into(), "ro".into()])
            .build()?,
    );

    // Build capabilities
    let caps = build_capabilities(&config.security)?;

    // Build seccomp (optional)
    let seccomp = build_seccomp(&config.security.seccomp)?;

    let masked_paths = vec![
        "/proc/acpi".into(),
        "/proc/asound".into(),
        "/proc/kcore".into(),
        "/proc/keys".into(),
        "/proc/latency_stats".into(),
        "/proc/timer_list".into(),
        "/proc/timer_stats".into(),
        "/proc/sched_debug".into(),
        "/proc/scsi".into(),
        "/sys/firmware".into(),
        "/sys/devices/virtual/powercap".into(),
    ];
    let readonly_paths = vec![
        "/proc/bus".into(),
        "/proc/fs".into(),
        "/proc/irq".into(),
        "/proc/sys".into(),
        "/proc/sysrq-trigger".into(),
    ];

    let mut linux_builder = LinuxBuilder::default();
    linux_builder = linux_builder
        .namespaces(namespaces)
        .masked_paths(masked_paths)
        .readonly_paths(readonly_paths);

    // UID/GID mappings only apply when we explicitly create a user namespace
    // (runc). gVisor handles UID mapping internally via its --rootless flag.
    if config.user_namespace && runtime != OciRuntime::Gvisor {
        // Rootless user namespaces only allow a single UID/GID mapping
        // entry (without the setuid `newuidmap` helper). runc's init
        // process requires UID 0, so we map containerID 0 → the host
        // user's real UID. The process runs as UID 0 inside the
        // namespace, which the kernel translates to the real UID for all
        // host-side operations (file ownership in bind mounts, etc.).
        //
        // The generated /etc/passwd maps UID 0 to the real username so
        // `whoami`, `id`, and `ls -la` show the expected user identity.
        linux_builder = linux_builder
            .uid_mappings(vec![
                LinuxIdMappingBuilder::default()
                    .container_id(0u32)
                    .host_id(uid)
                    .size(1u32)
                    .build()?,
            ])
            .gid_mappings(vec![
                LinuxIdMappingBuilder::default()
                    .container_id(0u32)
                    .host_id(gid)
                    .size(1u32)
                    .build()?,
            ]);
    }

    // gVisor provides its own syscall filtering via its sentry kernel —
    // applying an OCI seccomp profile on top is redundant and can cause
    // compatibility issues with runsc's internal syscall handling.
    if runtime != OciRuntime::Gvisor {
        if let Some(sec) = seccomp {
            linux_builder = linux_builder.seccomp(sec);
        }
    }

    let linux = linux_builder.build()?;

    // In a user namespace the process runs as UID 0 (mapped to the host UID).
    // Without a user namespace, run as the real UID directly.
    #[allow(clippy::similar_names)]
    let container_uid = if config.user_namespace { 0 } else { uid };
    #[allow(clippy::similar_names)]
    let container_gid = if config.user_namespace { 0 } else { gid };

    let user = UserBuilder::default()
        .uid(config.security.run_as_user.unwrap_or(container_uid))
        .gid(config.security.run_as_group.unwrap_or(container_gid))
        .build()?;

    let process = ProcessBuilder::default()
        .terminal(terminal)
        .user(user)
        .args(args)
        .env(env)
        .cwd("/")
        .capabilities(caps)
        .no_new_privileges(config.security.no_new_privileges)
        .build()?;

    let root = RootBuilder::default()
        .path("rootfs")
        .readonly(true)
        .build()?;

    SpecBuilder::default()
        .version("1.0.2")
        .process(process)
        .root(root)
        .hostname("synwire")
        .mounts(mounts)
        .linux(linux)
        .build()
}

/// Essential OCI mounts (proc, dev, devpts, sysfs).
fn essential_mounts() -> Result<Vec<Mount>, oci_spec::OciSpecError> {
    Ok(vec![
        MountBuilder::default()
            .destination("/proc")
            .typ("proc")
            .source("proc")
            .options(vec!["nosuid".into(), "noexec".into(), "nodev".into()])
            .build()?,
        MountBuilder::default()
            .destination("/dev")
            .typ("tmpfs")
            .source("tmpfs")
            .options(vec![
                "nosuid".into(),
                "strictatime".into(),
                "mode=755".into(),
                "size=65536k".into(),
            ])
            .build()?,
        MountBuilder::default()
            .destination("/dev/pts")
            .typ("devpts")
            .source("devpts")
            .options(vec![
                "nosuid".into(),
                "noexec".into(),
                "newinstance".into(),
                "ptmxmode=0666".into(),
                "mode=0620".into(),
            ])
            .build()?,
        MountBuilder::default()
            .destination("/dev/shm")
            .typ("tmpfs")
            .source("shm")
            .options(vec![
                "nosuid".into(),
                "noexec".into(),
                "nodev".into(),
                "mode=1777".into(),
                "size=65536k".into(),
            ])
            .build()?,
        MountBuilder::default()
            .destination("/dev/mqueue")
            .typ("mqueue")
            .source("mqueue")
            .options(vec!["nosuid".into(), "noexec".into(), "nodev".into()])
            .build()?,
        MountBuilder::default()
            .destination("/sys")
            .typ("none")
            .source("/sys")
            .options(vec![
                "rbind".into(),
                "nosuid".into(),
                "noexec".into(),
                "nodev".into(),
                "ro".into(),
            ])
            .build()?,
    ])
}

/// Build OCI `process.capabilities` from security config.
fn build_capabilities(
    security: &ContainerSecurity,
) -> Result<oci_spec::runtime::LinuxCapabilities, oci_spec::OciSpecError> {
    let drop_all = security.capabilities_drop.iter().any(|c| c == "ALL");
    let caps: oci_spec::runtime::Capabilities = if drop_all {
        security
            .capabilities_add
            .iter()
            .filter_map(|c| parse_capability(c))
            .collect()
    } else {
        // Minimal capability set for agent sandboxes. Intentionally much
        // tighter than Docker's default — agents run as a single user and
        // don't need DAC_OVERRIDE, CHOWN, FOWNER, SETUID/GID, or
        // SYS_CHROOT (runc handles pivot_root before the process starts).
        //
        // CAP_KILL: signal child processes spawned by the agent.
        // CAP_NET_BIND_SERVICE: bind ports <1024 if networking is enabled.
        // CAP_SETPCAP: drop further capabilities (supports no_new_privileges).
        let mut caps: oci_spec::runtime::Capabilities = [
            Capability::Kill,
            Capability::NetBindService,
            Capability::Setpcap,
        ]
        .into_iter()
        .collect();

        for drop in &security.capabilities_drop {
            if let Some(cap) = parse_capability(drop) {
                let _ = caps.remove(&cap);
            }
        }
        caps
    };

    LinuxCapabilitiesBuilder::default()
        .bounding(caps.clone())
        .effective(caps.clone())
        .inheritable(caps.clone())
        .permitted(caps.clone())
        .ambient(caps)
        .build()
}

/// Build OCI `linux.seccomp` from seccomp config. Returns `None` for `Unconfined`.
fn build_seccomp(
    seccomp: &ContainerSeccomp,
) -> Result<Option<oci_spec::runtime::LinuxSeccomp>, oci_spec::OciSpecError> {
    match seccomp {
        ContainerSeccomp::Unconfined => Ok(None),
        ContainerSeccomp::RuntimeDefault => {
            let syscall = LinuxSyscallBuilder::default()
                .names(vec![
                    "kexec_file_load".into(),
                    "kexec_load".into(),
                    "open_by_handle_at".into(),
                    "perf_event_open".into(),
                    "process_vm_readv".into(),
                    "process_vm_writev".into(),
                    "ptrace".into(),
                    "reboot".into(),
                    "request_key".into(),
                    "set_mempolicy".into(),
                    "swapon".into(),
                    "swapoff".into(),
                    "syslog".into(),
                    "umount2".into(),
                    "unshare".into(),
                    "uselib".into(),
                    "userfaultfd".into(),
                ])
                .action(LinuxSeccompAction::ScmpActErrno)
                .errno_ret(1u32)
                .build()?;

            Ok(Some(
                LinuxSeccompBuilder::default()
                    .default_action(LinuxSeccompAction::ScmpActAllow)
                    .architectures(vec![
                        Arch::ScmpArchX86_64,
                        Arch::ScmpArchX86,
                        Arch::ScmpArchAarch64,
                    ])
                    .syscalls(vec![syscall])
                    .build()?,
            ))
        }
        ContainerSeccomp::Localhost { path } => {
            // Load the profile from file — expected to be in OCI seccomp format.
            Ok(std::fs::read_to_string(path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok()))
        }
    }
}

/// Create mount-point directories inside rootfs for each OCI mount.
fn prepare_rootfs(rootfs: &Path, spec: &Spec) -> std::io::Result<()> {
    std::fs::create_dir_all(rootfs)?;
    if let Some(mounts) = spec.mounts() {
        for mount in mounts {
            let dest = mount.destination();
            let target = rootfs.join(dest.strip_prefix("/").unwrap_or(dest));
            std::fs::create_dir_all(&target)?;
        }
    }
    Ok(())
}

/// Receive the PTY controller fd from the OCI runtime via `SCM_RIGHTS`.
///
/// The runtime sends exactly one fd (the PTY controller) over the console
/// socket after creating the PTY inside the container.
fn recv_pty_controller(stream: &std::os::unix::net::UnixStream) -> Result<OwnedFd, SandboxError> {
    use nix::sys::socket::{ControlMessageOwned, MsgFlags, recvmsg};
    use std::os::fd::{AsRawFd, FromRawFd};

    let mut buf = [0u8; 1];
    let mut iov = [std::io::IoSliceMut::new(&mut buf)];
    let mut cmsg_buf = nix::cmsg_space!(std::os::fd::RawFd);

    let msg = recvmsg::<()>(
        stream.as_raw_fd(),
        &mut iov,
        Some(&mut cmsg_buf),
        MsgFlags::empty(),
    )
    .map_err(|e| SandboxError::RuntimeFailed {
        reason: format!("recvmsg on console socket: {e}"),
    })?;

    let iter = msg.cmsgs().map_err(|e| SandboxError::RuntimeFailed {
        reason: format!("parse control messages: {e}"),
    })?;
    for cmsg in iter {
        if let ControlMessageOwned::ScmRights(fds) = cmsg {
            if let Some(&raw_fd) = fds.first() {
                // SAFETY: The fd was received via SCM_RIGHTS from the OCI
                // runtime's console socket protocol. The runtime guarantees
                // this is a valid, newly-created PTY controller fd that we
                // now exclusively own.
                #[allow(unsafe_code)]
                let owned = unsafe { std::os::fd::OwnedFd::from_raw_fd(raw_fd) };
                return Ok(owned);
            }
        }
    }

    Err(SandboxError::RuntimeFailed {
        reason: "no PTY controller fd received from runtime".into(),
    })
}

/// Determine the best gVisor platform, with process-wide caching.
///
/// First checks the cache. If not yet probed, runs a trivial `runsc` container
/// with `--platform=systrap`. If it succeeds, caches `Systrap`. If it fails
/// (typically `PTRACE_ATTACH EPERM` from the `CAP_SYS_PTRACE` bug in rootless
/// + host-network mode), falls back to `Ptrace`, logs a warning, and caches
/// the result so all future containers skip the probe.
#[allow(clippy::doc_lazy_continuation)]
fn resolve_gvisor_platform(runsc_path: &Path) -> GvisorPlatform {
    use std::sync::atomic::Ordering;

    let cached = GVISOR_PLATFORM_CACHE.load(Ordering::Relaxed);
    if cached == PLATFORM_SYSTRAP {
        return GvisorPlatform::Systrap;
    }
    if cached == PLATFORM_PTRACE {
        return GvisorPlatform::Ptrace;
    }

    // Probe: try systrap with a trivial container.
    debug!("probing gVisor systrap platform");
    if probe_gvisor_platform(runsc_path, "systrap") {
        debug!("gVisor systrap platform works — using for all future containers");
        GVISOR_PLATFORM_CACHE.store(PLATFORM_SYSTRAP, Ordering::Relaxed);
        return GvisorPlatform::Systrap;
    }

    // Systrap failed. Try ptrace to confirm it works at all.
    if probe_gvisor_platform(runsc_path, "ptrace") {
        warn!(
            "gVisor systrap platform failed (likely missing CAP_SYS_PTRACE in \
             rootless+host-network mode — see runsc/sandbox/sandbox.go \
             ConfigureCmdForRootless). Falling back to ptrace platform for all \
             future gVisor containers in this process."
        );
        GVISOR_PLATFORM_CACHE.store(PLATFORM_PTRACE, Ordering::Relaxed);
        return GvisorPlatform::Ptrace;
    }

    // Neither works — default to ptrace and let the actual spawn surface the error.
    warn!("gVisor probe failed for both systrap and ptrace — defaulting to ptrace");
    GVISOR_PLATFORM_CACHE.store(PLATFORM_PTRACE, Ordering::Relaxed);
    GvisorPlatform::Ptrace
}

/// Run a trivial `runsc --platform=<p> run` and check if it exits 0.
fn probe_gvisor_platform(runsc_path: &Path, platform: &str) -> bool {
    let Ok(bundle_dir) = tempfile::TempDir::with_prefix("synwire-") else {
        return false;
    };
    let rootfs = bundle_dir.path().join("rootfs");
    if std::fs::create_dir_all(&rootfs).is_err() {
        return false;
    }

    let Ok(spec) = build_gvisor_probe_spec() else {
        return false;
    };

    // Create mount-point directories inside rootfs.
    if let Err(_e) = prepare_rootfs(&rootfs, &spec) {
        return false;
    }

    let Ok(spec_json) = serde_json::to_string_pretty(&spec) else {
        return false;
    };
    if std::fs::write(bundle_dir.path().join("config.json"), spec_json).is_err() {
        return false;
    }

    let container_id = format!("probe-{}", uuid::Uuid::new_v4());
    let result = std::process::Command::new(runsc_path)
        .arg("--rootless")
        .arg("--network=host")
        .arg(format!("--platform={platform}"))
        .arg("run")
        .arg("--bundle")
        .arg(bundle_dir.path())
        .arg(&container_id)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    match result {
        Ok(status) => status.success(),
        Err(_) => false,
    }
}

/// Build a minimal OCI spec for the gVisor platform probe (runs `/bin/true`).
fn build_gvisor_probe_spec() -> Result<Spec, oci_spec::OciSpecError> {
    let uid = nix::unistd::getuid().as_raw();
    let gid = nix::unistd::getgid().as_raw();

    let empty_caps: oci_spec::runtime::Capabilities = std::collections::HashSet::default();
    let caps = LinuxCapabilitiesBuilder::default()
        .bounding(empty_caps.clone())
        .effective(empty_caps.clone())
        .inheritable(empty_caps.clone())
        .permitted(empty_caps.clone())
        .ambient(empty_caps)
        .build()?;

    let process = ProcessBuilder::default()
        .terminal(false)
        .user(UserBuilder::default().uid(0u32).gid(0u32).build()?)
        .args(vec!["/bin/true".into()])
        .env(vec!["PATH=/usr/bin:/bin".into()])
        .cwd("/")
        .capabilities(caps)
        .no_new_privileges(true)
        .build()?;

    let root = RootBuilder::default()
        .path("rootfs")
        .readonly(true)
        .build()?;

    let namespaces = vec![
        LinuxNamespaceBuilder::default()
            .typ(LinuxNamespaceType::Pid)
            .build()?,
        LinuxNamespaceBuilder::default()
            .typ(LinuxNamespaceType::Mount)
            .build()?,
        LinuxNamespaceBuilder::default()
            .typ(LinuxNamespaceType::Ipc)
            .build()?,
        LinuxNamespaceBuilder::default()
            .typ(LinuxNamespaceType::Uts)
            .build()?,
        LinuxNamespaceBuilder::default()
            .typ(LinuxNamespaceType::Cgroup)
            .build()?,
    ];

    let linux = LinuxBuilder::default()
        .namespaces(namespaces)
        .uid_mappings(vec![
            LinuxIdMappingBuilder::default()
                .container_id(0u32)
                .host_id(uid)
                .size(1u32)
                .build()?,
        ])
        .gid_mappings(vec![
            LinuxIdMappingBuilder::default()
                .container_id(0u32)
                .host_id(gid)
                .size(1u32)
                .build()?,
        ])
        .build()?;

    SpecBuilder::default()
        .version("1.0.2")
        .process(process)
        .root(root)
        .mounts(probe_mounts()?)
        .linux(linux)
        .build()
}

/// Minimal mount list for gVisor probe — just enough to run `/bin/true`.
fn probe_mounts() -> Result<Vec<Mount>, oci_spec::OciSpecError> {
    let mut mounts = vec![
        MountBuilder::default()
            .destination("/proc")
            .typ("proc")
            .source("proc")
            .options(vec!["nosuid".into(), "noexec".into(), "nodev".into()])
            .build()?,
        MountBuilder::default()
            .destination("/dev")
            .typ("tmpfs")
            .source("tmpfs")
            .options(vec![
                "nosuid".into(),
                "strictatime".into(),
                "mode=755".into(),
                "size=65536k".into(),
            ])
            .build()?,
    ];
    for dir in &["/usr", "/bin", "/sbin", "/lib", "/lib64"] {
        if Path::new(dir).exists() {
            mounts.push(
                MountBuilder::default()
                    .destination(*dir)
                    .typ("bind")
                    .source(*dir)
                    .options(vec!["rbind".into(), "ro".into()])
                    .build()?,
            );
        }
    }
    Ok(mounts)
}

/// Generate minimal `/etc/passwd` and `/etc/group` files for the current user.
///
/// Includes a `root` entry (required by many tools) and the real user so
/// that `whoami`, `id`, `ls -la`, `$HOME`, and `~` all resolve correctly
/// inside the container.
fn generate_user_files(passwd_path: &Path, group_path: &Path) -> std::io::Result<()> {
    // Try to get the real username; fall back to "user".
    let username = std::env::var("USER")
        .or_else(|_| std::env::var("LOGNAME"))
        .unwrap_or_else(|_| "user".into());

    let home = std::env::var("HOME").unwrap_or_else(|_| format!("/home/{username}"));

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());

    let gid = nix::unistd::getgid().as_raw();

    // Try to resolve the group name from the host /etc/group.
    let groupname = resolve_group_name(gid).unwrap_or_else(|| username.clone());

    // In a rootless user namespace the process runs as UID 0 inside (which
    // is mapped to the real host UID). We make `whoami` and `id` show the
    // real username by mapping UID 0 to the real user's name, home, and
    // shell. This is the same trick Podman uses for rootless containers.
    //
    // passwd format: name:x:uid:gid:gecos:home:shell
    let passwd = format!(
        "{username}:x:0:0::{home}:{shell}\nnobody:x:65534:65534:nobody:/nonexistent:/sbin/nologin\n"
    );

    // group format: name:x:gid:members
    let group = format!("{groupname}:x:0:{username}\nnobody:x:65534:\n");

    std::fs::write(passwd_path, passwd)?;
    std::fs::write(group_path, group)?;
    Ok(())
}

/// Try to resolve a GID to a group name by scanning `/etc/group`.
fn resolve_group_name(gid: u32) -> Option<String> {
    let content = std::fs::read_to_string("/etc/group").ok()?;
    for line in content.lines() {
        let mut parts = line.splitn(4, ':');
        let name = parts.next()?;
        let _ = parts.next(); // password
        let group_gid: u32 = parts.next()?.parse().ok()?;
        if group_gid == gid {
            return Some(name.to_string());
        }
    }
    None
}

// ── helpers ────────────────────────────────────────────────────────────────

fn which_binary(name: &str) -> Result<PathBuf, ()> {
    which::which(name).map_err(|_| ())
}

fn to_absolute(path: &str) -> Option<String> {
    let p = std::path::Path::new(path);
    if p.is_absolute() {
        return Some(path.to_string());
    }
    std::env::current_dir()
        .ok()
        .map(|cwd| cwd.join(p).display().to_string())
}
