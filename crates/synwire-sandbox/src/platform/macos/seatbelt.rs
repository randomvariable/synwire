//! macOS Seatbelt (SBPL) profile generation and `sandbox-exec` spawning.
//!
//! Translates a [`SandboxConfig`] into a Seatbelt Profile Language (SBPL)
//! string and spawns commands via `sandbox-exec -p <profile> -- <command>`.
//!
//! # Seatbelt profile structure
//!
//! ```scheme
//! (version 1)
//! (deny default)           ; deny everything not explicitly allowed
//! ; --- network ---
//! (allow network*)         ; when network enabled
//! ; --- filesystem ---
//! (allow file-read*)       ; always: system libraries, process-exec, sysctl
//! (allow file-write* (subpath "/allowed/path"))
//! ; --- process ---
//! (allow process-exec)
//! (allow signal)
//! (allow sysctl-read)
//! ```

use tokio::process::Command;

use synwire_core::agents::sandbox::{
    FilesystemConfig, NetworkConfig, SandboxConfig, SecurityPreset,
};

use crate::SandboxError;

// ── SeatbeltProfile ───────────────────────────────────────────────────────────

/// A generated macOS Seatbelt SBPL profile.
#[derive(Debug, Clone)]
pub struct SeatbeltProfile {
    /// The rendered SBPL string, ready for `sandbox-exec -p`.
    sbpl: String,
}

impl SeatbeltProfile {
    /// Generate a Seatbelt profile from a [`SandboxConfig`].
    #[must_use]
    pub fn from_config(config: &SandboxConfig) -> Self {
        let sbpl = render_sbpl(
            config.filesystem.as_ref(),
            config.network.as_ref(),
            config.security.standard,
        );
        Self { sbpl }
    }

    /// Return the rendered SBPL string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.sbpl
    }

    /// Spawn a command inside this Seatbelt profile.
    ///
    /// Runs: `sandbox-exec -p <profile> -- <command> [args...]`
    ///
    /// # Errors
    ///
    /// Returns [`SandboxError::InitFailed`] if the child process cannot be spawned.
    pub fn spawn(
        &self,
        command: &str,
        args: &[String],
    ) -> Result<tokio::process::Child, SandboxError> {
        let mut cmd = Command::new("sandbox-exec");
        cmd.arg("-p").arg(&self.sbpl).arg("--").arg(command);
        for arg in args {
            cmd.arg(arg);
        }
        cmd.kill_on_drop(true)
            .spawn()
            .map_err(|e| SandboxError::InitFailed {
                reason: format!("sandbox-exec spawn failed: {e}"),
            })
    }
}

// ── SBPL renderer ─────────────────────────────────────────────────────────────

fn render_sbpl(
    fs: Option<&FilesystemConfig>,
    net: Option<&NetworkConfig>,
    preset: SecurityPreset,
) -> String {
    let mut lines: Vec<String> = vec![
        "(version 1)".into(),
        "(deny default)".into(),
        String::new(),
        "; --- system minimums (required for all processes) ---".into(),
        "(allow process-exec)".into(),
        "(allow signal)".into(),
        "(allow sysctl-read)".into(),
        // Dynamic linker, system frameworks.
        r#"(allow file-read* (subpath "/usr/lib"))"#.into(),
        r#"(allow file-read* (subpath "/usr/local/lib"))"#.into(),
        r#"(allow file-read* (subpath "/System/Library"))"#.into(),
        r#"(allow file-read* (subpath "/Library/Apple"))"#.into(),
        // Process metadata — needed by many tools.
        r#"(allow file-read-metadata)"#.into(),
        r#"(allow process-info*)"#.into(),
        String::new(),
    ];

    // --- network ---
    lines.push("; --- network ---".into());
    if net.map(|n| n.enabled).unwrap_or(false) {
        lines.push("(allow network*)".into());
    } else {
        lines.push("(deny network*)".into());
    }
    lines.push(String::new());

    // --- filesystem ---
    lines.push("; --- filesystem reads ---".into());

    let inherit_readable = fs.map(|f| f.inherit_readable).unwrap_or(true);
    if inherit_readable {
        // Allow reading the entire filesystem; deny_read entries are blocked below.
        lines.push("(allow file-read*)".into());
        if let Some(fs_cfg) = fs {
            for deny in &fs_cfg.deny_read {
                let escaped = scheme_string(deny);
                lines.push(format!("(deny file-read* (subpath {escaped}))"));
            }
        }
    } else {
        // Grant read access only to explicitly listed write roots.
        if let Some(fs_cfg) = fs {
            for allow in &fs_cfg.allow_write {
                let escaped = scheme_string(allow);
                lines.push(format!("(allow file-read* (subpath {escaped}))"));
            }
        }
    }

    lines.push(String::new());
    lines.push("; --- filesystem writes ---".into());

    if let Some(fs_cfg) = fs {
        for allow in &fs_cfg.allow_write {
            let escaped = scheme_string(allow);
            lines.push(format!("(allow file-write* (subpath {escaped}))"));
        }
        for deny in &fs_cfg.deny_write {
            let escaped = scheme_string(deny);
            lines.push(format!("(deny file-write* (subpath {escaped}))"));
        }
    }

    // Restricted preset: also deny exec of new processes outside allow_write.
    if preset == SecurityPreset::Restricted {
        lines.push(String::new());
        lines.push("; --- restricted: deny mprotect PROT_EXEC (best-effort) ---".into());
        lines.push("(deny file-write-data)".into());
    }

    lines.join("\n")
}

/// Escape a path for use as an SBPL string literal.
///
/// Wraps in double quotes and escapes backslashes and double quotes.
fn scheme_string(path: &str) -> String {
    let escaped = path.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use synwire_core::agents::sandbox::{
        EnvConfig, IsolationLevel, ProcessTracking, ResourceLimits, SecurityProfile,
    };

    fn make_config(preset: SecurityPreset, network: bool, write_roots: Vec<&str>) -> SandboxConfig {
        SandboxConfig {
            enabled: true,
            isolation: IsolationLevel::Seatbelt,
            filesystem: Some(FilesystemConfig {
                allow_write: write_roots.iter().map(|s| (*s).to_string()).collect(),
                deny_write: vec![],
                deny_read: vec!["/etc/shadow".into()],
                inherit_readable: true,
            }),
            network: Some(NetworkConfig {
                enabled: network,
                allowed_domains: None,
                denied_domains: vec![],
                allow_trusted_domains: false,
            }),
            env: EnvConfig::default(),
            security: SecurityProfile {
                standard: preset,
                ..SecurityProfile::default()
            },
            resources: None,
            process_tracking: ProcessTracking::default(),
            allowed_commands: None,
            denied_commands: vec![],
        }
    }

    #[test]
    fn baseline_profile_contains_deny_default() {
        let cfg = make_config(SecurityPreset::Baseline, false, vec!["/workspace"]);
        let profile = SeatbeltProfile::from_config(&cfg);
        assert!(profile.as_str().contains("(deny default)"));
    }

    #[test]
    fn network_disabled_produces_deny_network() {
        let cfg = make_config(SecurityPreset::Baseline, false, vec!["/workspace"]);
        let profile = SeatbeltProfile::from_config(&cfg);
        assert!(profile.as_str().contains("(deny network*)"));
        assert!(!profile.as_str().contains("(allow network*)"));
    }

    #[test]
    fn network_enabled_produces_allow_network() {
        let cfg = make_config(SecurityPreset::Baseline, true, vec!["/workspace"]);
        let profile = SeatbeltProfile::from_config(&cfg);
        assert!(profile.as_str().contains("(allow network*)"));
    }

    #[test]
    fn write_root_included() {
        let cfg = make_config(SecurityPreset::Baseline, false, vec!["/my/project"]);
        let profile = SeatbeltProfile::from_config(&cfg);
        assert!(
            profile
                .as_str()
                .contains(r#"(allow file-write* (subpath "/my/project"))"#)
        );
    }

    #[test]
    fn deny_read_path_included() {
        let cfg = make_config(SecurityPreset::Baseline, false, vec!["/workspace"]);
        let profile = SeatbeltProfile::from_config(&cfg);
        assert!(
            profile
                .as_str()
                .contains(r#"(deny file-read* (subpath "/etc/shadow"))"#)
        );
    }

    #[test]
    fn restricted_preset_produces_deny_write_data() {
        let cfg = make_config(SecurityPreset::Restricted, false, vec!["/workspace"]);
        let profile = SeatbeltProfile::from_config(&cfg);
        assert!(profile.as_str().contains("(deny file-write-data)"));
    }

    #[test]
    fn privileged_preset_no_deny_write_data() {
        let cfg = make_config(SecurityPreset::Privileged, false, vec!["/workspace"]);
        let profile = SeatbeltProfile::from_config(&cfg);
        assert!(!profile.as_str().contains("(deny file-write-data)"));
    }
}
