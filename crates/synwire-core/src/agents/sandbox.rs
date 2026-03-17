//! Sandbox configuration for agents.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// ── Isolation level ───────────────────────────────────────────────────────────

/// How strongly to isolate processes spawned by this agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub enum IsolationLevel {
    /// No isolation — plain `tokio::process::Command`. Approval prompts remain active.
    #[default]
    None,
    /// cgroup v2 tracking + optional `AppArmor` (Linux) or Seatbelt (macOS).
    /// Falls back to `None` gracefully when unavailable.
    /// When active, terminal commands are auto-approved.
    CgroupTracking,
    /// Full Linux namespace container via OCI runtime (runc/crun).
    /// Provides PID, UTS, IPC, mount, cgroup, and optional network/user namespaces.
    Namespace,
    /// macOS `sandbox-exec` with a generated Seatbelt SBPL profile.
    Seatbelt,
    /// OCI container via Podman or Lima (macOS strong isolation).
    Container,
}

// ── Filesystem config ─────────────────────────────────────────────────────────

/// Filesystem access rules.
///
/// Designed for coding agent scenarios: the full host filesystem remains
/// readable by default (binaries, dotfiles, shared libraries), while writes
/// are restricted to the working directory. Enforcement mechanism varies by
/// [`IsolationLevel`]:
///
/// - `None` / `CgroupTracking`: `AppArmor` (Linux) or Seatbelt (macOS) profile
/// - `Namespace`: bind-mount policy derived from these rules
/// - `Container`: translated to `podman --volume` flags
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FilesystemConfig {
    /// Paths where writes are explicitly permitted.
    ///
    /// Supports absolute paths and cwd-relative paths (including glob patterns
    /// such as `"./src/**"`). Default: `["."]` — working directory only.
    pub allow_write: Vec<String>,
    /// Paths where writes are blocked, evaluated after `allow_write`.
    ///
    /// Example: `["./secrets/", ".env"]`
    pub deny_write: Vec<String>,
    /// Paths where reads are blocked.
    ///
    /// Example: `["/etc/shadow", "~/.ssh/id_rsa"]`
    pub deny_read: Vec<String>,
    /// Expose the entire host filesystem as readable.
    ///
    /// `true` by default — preserves access to binaries, dotfiles, and shared
    /// libraries. In `Namespace` mode this causes the host root to be
    /// bind-mounted read-only; `deny_read` entries are excluded.
    /// Set to `false` for a stripped environment.
    pub inherit_readable: bool,
}

impl Default for FilesystemConfig {
    fn default() -> Self {
        Self {
            allow_write: vec![".".into()],
            deny_write: vec![],
            deny_read: vec![],
            inherit_readable: true,
        }
    }
}

// ── Network config ────────────────────────────────────────────────────────────

/// Network access rules.
///
/// Disabled by default — coding agents should explicitly opt-in to the domains
/// they require.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct NetworkConfig {
    /// Enable outbound network access. Default: `false`.
    pub enabled: bool,
    /// Domain allowlist. Supports wildcards: `"*.npmjs.org"`.
    ///
    /// `None` means all domains are permitted when `enabled = true`.
    pub allowed_domains: Option<Vec<String>>,
    /// Domains that are always blocked, regardless of `allowed_domains`.
    pub denied_domains: Vec<String>,
    /// Also allow domains from a system or user trusted-domains list
    /// (e.g. a corporate proxy allowlist).
    pub allow_trusted_domains: bool,
}

// ── Environment config ────────────────────────────────────────────────────────

/// Controls environment variable inheritance for sandboxed processes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EnvConfig {
    /// Inherit all environment variables from the parent process.
    ///
    /// `true` by default — preserves `PATH`, `HOME`, dotfile locations, tool
    /// configurations, and editor settings. Set to `false` for a clean
    /// environment.
    pub inherit_parent: bool,
    /// Additional variables to set. Merged after parent env when
    /// `inherit_parent = true`; these values take precedence.
    pub set: HashMap<String, String>,
    /// Variable names to remove from the inherited set.
    ///
    /// Example: `["AWS_SECRET_ACCESS_KEY", "GITHUB_TOKEN"]`
    pub unset: Vec<String>,
}

impl Default for EnvConfig {
    fn default() -> Self {
        Self {
            inherit_parent: true,
            set: HashMap::new(),
            unset: vec![],
        }
    }
}

// ── Resource limits ───────────────────────────────────────────────────────────

/// Resource limits applied via cgroup v2 or container runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
#[derive(Default)]
pub struct ResourceLimits {
    /// Maximum memory in bytes. Maps to cgroup `memory.max`.
    pub memory_bytes: Option<u64>,
    /// CPU quota as a fraction of one core (e.g. `0.5` = 50% of one core).
    /// Maps to cgroup `cpu.max` with a 100 ms period.
    pub cpu_quota: Option<f32>,
    /// Maximum number of PIDs in the cgroup. Maps to `pids.max`.
    pub max_pids: Option<u32>,
    /// Wall-clock timeout for a single `execute()` call, in seconds.
    pub exec_timeout_secs: Option<u64>,
}

impl ResourceLimits {
    /// Create resource limits with all fields set to `None` (no restrictions).
    #[must_use]
    pub fn none() -> Self {
        Self::default()
    }
}

// ── Process tracking ──────────────────────────────────────────────────────────

/// Controls whether spawned processes are tracked and exposed as LLM tools.
///
/// When enabled, the agent gains `list_processes`, `kill_process`, and
/// `process_stats` tools automatically.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ProcessTracking {
    /// Enable process tracking and the associated LLM tools.
    pub enabled: bool,
    /// Maximum number of concurrently tracked processes. New spawns are
    /// rejected with an error once this limit is reached. `None` = unlimited.
    pub max_tracked: Option<usize>,
}

// ── Security profile ──────────────────────────────────────────────────────────

/// High-level security preset, inspired by (but not implementing) Kubernetes PSA.
///
/// The preset expands to concrete `seccomp`, `capabilities`, and
/// `no_new_privileges` values before being passed to the sandbox init binary.
/// Individual fields in [`SecurityProfile`] can override the preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub enum SecurityPreset {
    /// No restrictions. Use only in fully-trusted environments.
    Privileged,
    /// Prevent known privilege escalations. **Default.**
    ///
    /// Enables `PR_SET_NO_NEW_PRIVS`, applies `RuntimeDefault` seccomp, and
    /// drops `NET_RAW`, `SYS_PTRACE`, and `SYS_ADMIN` capabilities.
    #[default]
    Baseline,
    /// Defence-in-depth. All `Baseline` restrictions plus drop-all
    /// capabilities and `RuntimeDefault` seccomp (required).
    Restricted,
}

/// Seccomp filter selection.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SeccompProfile {
    /// No seccomp filter applied.
    Unconfined,
    /// Bundled default profile (~50 blocked syscalls: `ptrace`,
    /// `perf_event_open`, `process_vm_readv`, `kexec_load`, etc.).
    ///
    /// Translated to OCI seccomp format and applied by the container runtime.
    #[default]
    RuntimeDefault,
    /// Load a custom seccomp profile JSON from the given path.
    Localhost {
        /// Absolute path to the seccomp profile JSON file.
        path: String,
    },
}

/// Linux capability adjustments applied before exec.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CapabilityConfig {
    /// Capabilities to drop. `["ALL"]` drops every capability.
    pub drop: Vec<String>,
    /// Capabilities to add back after dropping.
    /// Only meaningful when combined with `drop: ["ALL"]`.
    pub add: Vec<String>,
}

/// Per-process security context applied by the sandbox.
///
/// The `standard` preset provides safe defaults; individual fields can be
/// overridden for fine-grained control.
///
/// Expansion table:
///
/// | Preset       | `no_new_privileges` | seccomp          | capabilities                      |
/// |--------------|---------------------|------------------|-----------------------------------|
/// | `Privileged` | false               | `Unconfined`     | no drops                          |
/// | `Baseline`   | true                | `RuntimeDefault` | drop `NET_RAW`, `SYS_PTRACE`, `SYS_ADMIN` |
/// | `Restricted` | true                | `RuntimeDefault` | drop `ALL`, no adds allowed       |
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SecurityProfile {
    /// High-level policy preset. Overrides other fields to safe defaults.
    pub standard: SecurityPreset,
    /// Seccomp profile to apply before exec.
    pub seccomp: SeccompProfile,
    /// Linux capability adjustments.
    pub capabilities: CapabilityConfig,
    /// Set `PR_SET_NO_NEW_PRIVS` before exec, preventing privilege escalation
    /// via setuid binaries or file capabilities.
    pub no_new_privileges: bool,
    /// Run as this UID. `None` = inherit from calling process.
    pub run_as_user: Option<u32>,
    /// Run as this GID. `None` = inherit from calling process.
    pub run_as_group: Option<u32>,
}

impl Default for SecurityProfile {
    /// Baseline preset: `no_new_privileges`, `RuntimeDefault` seccomp,
    /// drop `NET_RAW` / `SYS_PTRACE` / `SYS_ADMIN`.
    fn default() -> Self {
        Self {
            standard: SecurityPreset::Baseline,
            seccomp: SeccompProfile::RuntimeDefault,
            capabilities: CapabilityConfig {
                drop: vec!["NET_RAW".into(), "SYS_PTRACE".into(), "SYS_ADMIN".into()],
                add: vec![],
            },
            no_new_privileges: true,
            run_as_user: None,
            run_as_group: None,
        }
    }
}

// ── SandboxConfig ─────────────────────────────────────────────────────────────

/// Agent-level sandbox configuration.
///
/// Use [`SandboxConfig::coding_agent`] for a sensible default suitable for
/// coding agents: full environment inherited, writes restricted to the working
/// directory, network off, cgroup tracking enabled.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SandboxConfig {
    /// Enable sandboxing. When `false`, all other fields are ignored and
    /// processes run with no restrictions.
    pub enabled: bool,
    /// Isolation mechanism to use. When `isolation != None` and
    /// `enabled = true`, terminal commands are auto-approved.
    pub isolation: IsolationLevel,
    /// Filesystem access rules.
    pub filesystem: Option<FilesystemConfig>,
    /// Network access rules.
    pub network: Option<NetworkConfig>,
    /// Environment variable inheritance.
    pub env: EnvConfig,
    /// Process-level security context (seccomp, capabilities, NNP).
    pub security: SecurityProfile,
    /// cgroup / container resource limits.
    pub resources: Option<ResourceLimits>,
    /// Process tracking and associated LLM tools.
    pub process_tracking: ProcessTracking,
    /// Command allowlist. `None` = all commands permitted.
    pub allowed_commands: Option<Vec<String>>,
    /// Command blocklist (evaluated after `allowed_commands`).
    pub denied_commands: Vec<String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            isolation: IsolationLevel::None,
            filesystem: None,
            network: None,
            env: EnvConfig::default(),
            security: SecurityProfile::default(),
            resources: None,
            process_tracking: ProcessTracking::default(),
            allowed_commands: None,
            denied_commands: vec![],
        }
    }
}

impl SandboxConfig {
    /// Preset for coding agents.
    ///
    /// - Full host filesystem readable; writes restricted to working directory
    /// - Network disabled by default
    /// - Full parent environment inherited (`PATH`, `HOME`, dotfiles, etc.)
    /// - cgroup v2 tracking on Linux (auto-approved terminal commands)
    /// - Baseline security (NNP + `RuntimeDefault` seccomp)
    /// - Process tracking enabled (`list_processes`, `kill_process`,
    ///   `process_stats` available as LLM tools)
    #[must_use]
    pub fn coding_agent() -> Self {
        Self {
            enabled: true,
            isolation: IsolationLevel::CgroupTracking,
            filesystem: Some(FilesystemConfig::default()),
            network: Some(NetworkConfig::default()),
            env: EnvConfig::default(),
            security: SecurityProfile::default(),
            resources: None,
            process_tracking: ProcessTracking {
                enabled: true,
                max_tracked: Some(64),
            },
            allowed_commands: None,
            denied_commands: vec![],
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn coding_agent_preset_has_expected_defaults() {
        let cfg = SandboxConfig::coding_agent();
        assert!(cfg.enabled);
        assert_eq!(cfg.isolation, IsolationLevel::CgroupTracking);
        assert!(cfg.process_tracking.enabled);
        assert_eq!(cfg.process_tracking.max_tracked, Some(64));
        assert!(cfg.env.inherit_parent);
        let fs = cfg.filesystem.unwrap();
        assert_eq!(fs.allow_write, vec!["."]);
        assert!(fs.inherit_readable);
        let net = cfg.network.unwrap();
        assert!(!net.enabled);
    }

    #[test]
    fn default_security_is_baseline() {
        let sp = SecurityProfile::default();
        assert_eq!(sp.standard, SecurityPreset::Baseline);
        assert!(sp.no_new_privileges);
        assert!(sp.capabilities.drop.contains(&"NET_RAW".to_string()));
        assert!(sp.capabilities.drop.contains(&"SYS_PTRACE".to_string()));
        assert!(sp.capabilities.drop.contains(&"SYS_ADMIN".to_string()));
    }

    #[test]
    fn serde_round_trip() {
        let cfg = SandboxConfig::coding_agent();
        let json = serde_json::to_string(&cfg).unwrap();
        let back: SandboxConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.isolation, cfg.isolation);
        assert_eq!(
            back.security.no_new_privileges,
            cfg.security.no_new_privileges
        );
    }
}
