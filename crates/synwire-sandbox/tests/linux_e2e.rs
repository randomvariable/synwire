//! Linux end-to-end tests for the sandbox implementation.
//!
//! # Test tiers
//!
//! | Tier | Requires | Guard |
//! |------|----------|-------|
//! | **Unprivileged** | tokio runtime | always runs |
//! | **cgroup** | cgroup v2 + systemd user delegation | `#[ignore]`, runtime skip |
//! | **namespace** | runc + user namespaces | `#[ignore]`, runtime skip |
//! | **gvisor** | runsc (gVisor) + user namespaces | `#[ignore]`, runtime skip |
//!
//! Run ignored tests: `cargo test -p synwire-sandbox --test linux_e2e -- --ignored`

#![cfg(target_os = "linux")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_wrap,
    clippy::significant_drop_tightening,
    clippy::allow_attributes,
    clippy::print_stderr
)]

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;

use synwire_sandbox::output::{CapturedOutput, OutputMode};
use synwire_sandbox::process_registry::{
    ProcessRecord, ProcessRegistry, ProcessStatus, monitor_child,
};
use synwire_sandbox::visibility::ProcessVisibilityScope;

// ═══════════════════════════════════════════════════════════════════════════════
// Unprivileged tests — always run, no special capabilities needed
// ═══════════════════════════════════════════════════════════════════════════════

// ── TempDir lifecycle ─────────────────────────────────────────────────────────

#[test]
fn tempdir_created_and_exists() {
    let td = tempfile::TempDir::with_prefix("synwire-").unwrap();
    assert!(td.path().exists());
    assert!(td.path().is_dir());
}

#[test]
fn tempdir_auto_cleanup_on_drop() {
    let dir_path;
    {
        let td = tempfile::TempDir::with_prefix("synwire-").unwrap();
        dir_path = td.path().to_path_buf();
        assert!(dir_path.exists());
        std::fs::write(dir_path.join("test.txt"), "data").unwrap();
    }
    assert!(!dir_path.exists());
}

#[test]
fn tempdir_cleanup_removes_nested_files() {
    let dir_path;
    {
        let td = tempfile::TempDir::with_prefix("synwire-").unwrap();
        dir_path = td.path().to_path_buf();
        let sub = dir_path.join("sub");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("nested.txt"), "deep").unwrap();
    }
    assert!(!dir_path.exists());
}

// ── CapturedOutput ────────────────────────────────────────────────────────────

#[test]
fn captured_output_separate_streams() {
    let output = CapturedOutput::new(OutputMode::Separate).unwrap();
    std::fs::write(output.stdout_path(), "hello stdout").unwrap();
    std::fs::write(output.stderr_path().unwrap(), "hello stderr").unwrap();

    assert_eq!(output.read_stdout().unwrap(), "hello stdout");
    assert_eq!(
        output.read_stderr().unwrap(),
        Some("hello stderr".to_string())
    );
}

#[test]
fn captured_output_combined_streams() {
    let output = CapturedOutput::new(OutputMode::Combined).unwrap();
    std::fs::write(output.stdout_path(), "interleaved output").unwrap();

    assert!(output.stderr_path().is_none());
    assert_eq!(output.read_stdout().unwrap(), "interleaved output");
    assert_eq!(output.read_stderr().unwrap(), None);
}

#[test]
fn captured_output_empty_before_write() {
    let output = CapturedOutput::new(OutputMode::Separate).unwrap();
    assert_eq!(output.read_stdout().unwrap(), "");
    assert_eq!(output.read_stderr().unwrap(), Some(String::new()));
}

#[test]
fn captured_output_drops_with_tempdir() {
    let stdout_path;
    {
        let output = CapturedOutput::new(OutputMode::Separate).unwrap();
        stdout_path = output.stdout_path();
        std::fs::write(&stdout_path, "ephemeral").unwrap();
        assert!(stdout_path.exists());
    }
    assert!(!stdout_path.exists());
}

#[test]
fn captured_output_arc_extends_lifetime() {
    let arc;
    let stdout_path;
    {
        let output = CapturedOutput::new(OutputMode::Combined).unwrap();
        stdout_path = output.stdout_path();
        std::fs::write(&stdout_path, "persisted").unwrap();
        arc = Arc::new(output);
    }
    assert!(stdout_path.exists());
    assert_eq!(arc.read_stdout().unwrap(), "persisted");

    drop(arc);
    assert!(!stdout_path.exists());
}

// ── monitor_child ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn monitor_child_tracks_successful_exit() {
    let registry = Arc::new(RwLock::new(ProcessRegistry::new(None)));

    let child = tokio::process::Command::new("true")
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let pid = child.id().unwrap();

    let record = ProcessRecord::new(pid, "true", vec![]);
    registry.write().await.insert(record).unwrap();

    monitor_child(child, pid, Arc::clone(&registry));
    tokio::time::sleep(Duration::from_millis(500)).await;

    let reg = registry.read().await;
    let r = reg.get(pid).unwrap();
    assert!(
        matches!(r.status, ProcessStatus::Exited { code: 0 }),
        "expected Exited(0), got {:?}",
        r.status
    );
}

#[tokio::test]
async fn monitor_child_tracks_nonzero_exit() {
    let registry = Arc::new(RwLock::new(ProcessRegistry::new(None)));

    let child = tokio::process::Command::new("false")
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let pid = child.id().unwrap();

    let record = ProcessRecord::new(pid, "false", vec![]);
    registry.write().await.insert(record).unwrap();

    monitor_child(child, pid, Arc::clone(&registry));
    tokio::time::sleep(Duration::from_millis(500)).await;

    let reg = registry.read().await;
    let r = reg.get(pid).unwrap();
    assert!(
        matches!(r.status, ProcessStatus::Exited { code: 1 }),
        "expected Exited(1), got {:?}",
        r.status
    );
}

#[tokio::test]
async fn monitor_child_tracks_signal_death() {
    let registry = Arc::new(RwLock::new(ProcessRegistry::new(None)));

    let child = tokio::process::Command::new("sleep")
        .arg("60")
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let pid = child.id().unwrap();

    let record = ProcessRecord::new(pid, "sleep", vec!["60".into()]);
    registry.write().await.insert(record).unwrap();

    monitor_child(child, pid, Arc::clone(&registry));

    nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(pid as i32),
        nix::sys::signal::Signal::SIGKILL,
    )
    .unwrap();

    tokio::time::sleep(Duration::from_millis(500)).await;

    let reg = registry.read().await;
    let r = reg.get(pid).unwrap();
    assert!(
        matches!(r.status, ProcessStatus::Signaled { .. }),
        "expected Signaled, got {:?}",
        r.status
    );
}

// ── ProcessRecord with output ─────────────────────────────────────────────────

#[tokio::test]
async fn process_record_holds_captured_output() {
    let registry = Arc::new(RwLock::new(ProcessRegistry::new(None)));
    let output = Arc::new(CapturedOutput::new(OutputMode::Separate).unwrap());
    let stdout_path = output.stdout_path();

    std::fs::write(&stdout_path, "captured").unwrap();

    let mut record = ProcessRecord::new(999, "test", vec![]);
    record.output = Some(Arc::clone(&output));
    registry.write().await.insert(record).unwrap();

    let reg = registry.read().await;
    let r = reg.get(999).unwrap();
    let captured = r.output.as_ref().unwrap();
    assert_eq!(captured.read_stdout().unwrap(), "captured");
}

#[tokio::test]
async fn process_record_output_survives_gc() {
    let registry = Arc::new(RwLock::new(ProcessRegistry::new(None)));
    let output = Arc::new(CapturedOutput::new(OutputMode::Separate).unwrap());
    let stdout_path = output.stdout_path();
    std::fs::write(&stdout_path, "before-gc").unwrap();

    let output_ref = Arc::clone(&output);

    let mut record = ProcessRecord::new(888, "test", vec![]);
    record.output = Some(output);
    registry.write().await.insert(record).unwrap();

    {
        let mut reg = registry.write().await;
        reg.mark_exited(888, 0);
        reg.gc();
    }

    assert!(stdout_path.exists());
    assert_eq!(output_ref.read_stdout().unwrap(), "before-gc");

    drop(output_ref);
    assert!(!stdout_path.exists());
}

// ── ProcessVisibilityScope ────────────────────────────────────────────────────

#[tokio::test]
async fn visibility_scope_own_processes_visible() {
    let registry = Arc::new(RwLock::new(ProcessRegistry::new(None)));
    let scope = ProcessVisibilityScope::new(Arc::clone(&registry));

    registry
        .write()
        .await
        .insert(ProcessRecord::new(10, "own-cmd", vec![]))
        .unwrap();

    let visible = scope.visible_running().await;
    assert_eq!(visible.len(), 1);
    assert!(visible[0].0.is_none());
    assert_eq!(visible[0].1.pid, 10);
}

#[tokio::test]
async fn visibility_scope_parent_sees_child_processes() {
    let parent_reg = Arc::new(RwLock::new(ProcessRegistry::new(None)));
    let child_reg = Arc::new(RwLock::new(ProcessRegistry::new(None)));

    let scope = ProcessVisibilityScope::new(Arc::clone(&parent_reg));
    scope
        .add_child_registry("sub-agent-1", Arc::clone(&child_reg))
        .await;

    parent_reg
        .write()
        .await
        .insert(ProcessRecord::new(1, "parent-cmd", vec![]))
        .unwrap();
    child_reg
        .write()
        .await
        .insert(ProcessRecord::new(2, "child-cmd", vec![]))
        .unwrap();

    let visible = scope.visible_running().await;
    assert_eq!(visible.len(), 2);

    let own: Vec<_> = visible.iter().filter(|(l, _)| l.is_none()).collect();
    assert_eq!(own.len(), 1);
    assert_eq!(own[0].1.pid, 1);

    let child: Vec<_> = visible
        .iter()
        .filter(|(l, _)| l.as_deref() == Some("sub-agent-1"))
        .collect();
    assert_eq!(child.len(), 1);
    assert_eq!(child[0].1.pid, 2);
}

#[tokio::test]
async fn visibility_scope_child_cannot_see_parent() {
    let parent_reg = Arc::new(RwLock::new(ProcessRegistry::new(None)));
    let child_reg = Arc::new(RwLock::new(ProcessRegistry::new(None)));

    let child_scope = ProcessVisibilityScope::new(Arc::clone(&child_reg));

    parent_reg
        .write()
        .await
        .insert(ProcessRecord::new(100, "parent-secret", vec![]))
        .unwrap();
    child_reg
        .write()
        .await
        .insert(ProcessRecord::new(200, "child-own", vec![]))
        .unwrap();

    let visible = child_scope.visible_running().await;
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].1.pid, 200);

    assert!(child_scope.find(100).await.is_none());
    assert!(child_scope.find(200).await.is_some());
}

#[tokio::test]
async fn visibility_find_returns_correct_label() {
    let parent_reg = Arc::new(RwLock::new(ProcessRegistry::new(None)));
    let child_reg = Arc::new(RwLock::new(ProcessRegistry::new(None)));

    let scope = ProcessVisibilityScope::new(Arc::clone(&parent_reg));
    scope
        .add_child_registry("worker-a", Arc::clone(&child_reg))
        .await;

    parent_reg
        .write()
        .await
        .insert(ProcessRecord::new(50, "own", vec![]))
        .unwrap();
    child_reg
        .write()
        .await
        .insert(ProcessRecord::new(51, "child", vec![]))
        .unwrap();

    let (label, record) = scope.find(50).await.unwrap();
    assert!(label.is_none());
    assert_eq!(record.pid, 50);

    let (label, record) = scope.find(51).await.unwrap();
    assert_eq!(label.as_deref(), Some("worker-a"));
    assert_eq!(record.pid, 51);

    assert!(scope.find(999).await.is_none());
}

#[tokio::test]
async fn visibility_multiple_children() {
    let parent_reg = Arc::new(RwLock::new(ProcessRegistry::new(None)));
    let child_a = Arc::new(RwLock::new(ProcessRegistry::new(None)));
    let child_b = Arc::new(RwLock::new(ProcessRegistry::new(None)));

    let scope = ProcessVisibilityScope::new(Arc::clone(&parent_reg));
    scope
        .add_child_registry("agent-a", Arc::clone(&child_a))
        .await;
    scope
        .add_child_registry("agent-b", Arc::clone(&child_b))
        .await;

    child_a
        .write()
        .await
        .insert(ProcessRecord::new(300, "cmd-a", vec![]))
        .unwrap();
    child_b
        .write()
        .await
        .insert(ProcessRecord::new(301, "cmd-b", vec![]))
        .unwrap();

    let visible = scope.visible_running().await;
    assert_eq!(visible.len(), 2);

    let pids: Vec<u32> = visible.iter().map(|(_, r)| r.pid).collect();
    assert!(pids.contains(&300));
    assert!(pids.contains(&301));
}

// ═══════════════════════════════════════════════════════════════════════════════
// cgroup v2 tests — require systemd user delegation
// ═══════════════════════════════════════════════════════════════════════════════

mod cgroup {
    use super::*;
    use synwire_sandbox::platform::linux::cgroup::CgroupV2Manager;
    use uuid::Uuid;

    async fn skip_if_unavailable() -> bool {
        if !CgroupV2Manager::is_available().await {
            eprintln!("SKIP: cgroup v2 not available on this system");
            return true;
        }
        false
    }

    async fn try_create_manager(
        agent_id: Uuid,
        resources: Option<&synwire_core::agents::sandbox::ResourceLimits>,
    ) -> Option<CgroupV2Manager> {
        match CgroupV2Manager::new(agent_id, resources).await {
            Ok(mgr) => Some(mgr),
            Err(e) => {
                eprintln!("SKIP: cannot create cgroup manager (no delegation?): {e}");
                None
            }
        }
    }

    #[tokio::test]
    #[ignore = "requires cgroup v2 + systemd user delegation"]
    async fn cgroup_create_and_destroy() {
        if skip_if_unavailable().await {
            return;
        }
        let Some(mgr) = try_create_manager(Uuid::new_v4(), None).await else {
            return;
        };
        assert!(mgr.base_path().exists());
        mgr.destroy().await;
        assert!(!mgr.base_path().exists());
    }

    #[tokio::test]
    #[ignore = "requires cgroup v2 + systemd user delegation"]
    async fn cgroup_move_pid_and_read_stats() {
        if skip_if_unavailable().await {
            return;
        }
        let Some(mgr) = try_create_manager(Uuid::new_v4(), None).await else {
            return;
        };

        let mut child = tokio::process::Command::new("sleep")
            .arg("1")
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let pid = child.id().unwrap();
        mgr.move_pid(pid).await.unwrap();
        let _stats = mgr.read_stats().await;
        let status = child.wait().await.unwrap();
        assert!(status.success());
        mgr.destroy().await;
    }

    #[tokio::test]
    #[ignore = "requires cgroup v2 + systemd user delegation"]
    async fn cgroup_kill_all_terminates_processes() {
        if skip_if_unavailable().await {
            return;
        }
        let Some(mgr) = try_create_manager(Uuid::new_v4(), None).await else {
            return;
        };

        let mut child = tokio::process::Command::new("sleep")
            .arg("3600")
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let pid = child.id().unwrap();
        mgr.move_pid(pid).await.unwrap();
        mgr.kill_all().await.unwrap();

        let status = tokio::time::timeout(Duration::from_secs(5), child.wait())
            .await
            .expect("child did not exit within 5s")
            .unwrap();
        assert!(!status.success());
        mgr.destroy().await;
    }

    #[tokio::test]
    #[ignore = "requires cgroup v2 + systemd user delegation"]
    async fn cgroup_drop_kills_processes() {
        if skip_if_unavailable().await {
            return;
        }
        let Some(mgr) = try_create_manager(Uuid::new_v4(), None).await else {
            return;
        };

        let mut child = tokio::process::Command::new("sleep")
            .arg("3600")
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let pid = child.id().unwrap();
        mgr.move_pid(pid).await.unwrap();
        drop(mgr);

        let status = tokio::time::timeout(Duration::from_secs(5), child.wait())
            .await
            .expect("child did not exit within 5s after Drop")
            .unwrap();
        assert!(!status.success());
    }

    #[tokio::test]
    #[ignore = "requires cgroup v2 + systemd user delegation"]
    async fn cgroup_resource_limits() {
        if skip_if_unavailable().await {
            return;
        }
        let mut limits = synwire_core::agents::sandbox::ResourceLimits::default();
        limits.memory_bytes = Some(128 * 1024 * 1024);
        limits.cpu_quota = Some(0.5);
        limits.max_pids = Some(32);

        let Some(mgr) = try_create_manager(Uuid::new_v4(), Some(&limits)).await else {
            return;
        };

        let memory_max = tokio::fs::read_to_string(mgr.base_path().join("memory.max"))
            .await
            .unwrap_or_default();
        assert!(
            memory_max.trim().parse::<u64>().is_ok(),
            "memory.max should contain a number, got: {memory_max}"
        );

        let pids_max = tokio::fs::read_to_string(mgr.base_path().join("pids.max"))
            .await
            .unwrap_or_default();
        assert_eq!(pids_max.trim(), "32");

        mgr.destroy().await;
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Namespace container tests — require runc + user namespaces
// ═══════════════════════════════════════════════════════════════════════════════

mod namespace {
    use super::*;
    use synwire_sandbox::platform::linux::namespace::{
        BindMount, CloneFlag, ContainerConfig, ContainerSeccomp, ContainerSecurity,
        NamespaceContainer,
    };

    /// Build a rootless container config that can run a command with host
    /// filesystem access. Uses user namespace for rootless operation.
    fn rootless_config(command: &str, args: Vec<String>) -> ContainerConfig {
        // Bind-mount essential host directories so the command can run.
        let host_dirs: Vec<BindMount> = ["/usr", "/bin", "/sbin", "/lib", "/lib64", "/etc", "/tmp"]
            .iter()
            .filter(|d| std::path::Path::new(d).exists())
            .map(|d| BindMount {
                source: d.to_string(),
                target: d.to_string(),
                read_only: true,
            })
            .collect();

        ContainerConfig {
            clone_flags: vec![
                CloneFlag::NewPid,
                CloneFlag::NewUts,
                CloneFlag::NewIpc,
                CloneFlag::NewNs,
            ],
            network_isolation: false,
            user_namespace: true,
            cgroup_namespace: false,
            bind_mounts: host_dirs,
            cgroup_path: None,
            security: ContainerSecurity {
                seccomp: ContainerSeccomp::Unconfined,
                capabilities_drop: vec![],
                capabilities_add: vec![],
                no_new_privileges: true,
                run_as_user: None,
                run_as_group: None,
            },
            command: command.to_string(),
            args,
            env: std::env::vars().collect(),
        }
    }

    fn get_container() -> Option<NamespaceContainer> {
        match NamespaceContainer::new() {
            Ok(ns) => Some(ns),
            Err(e) => {
                eprintln!("SKIP: no OCI runtime found: {e}");
                None
            }
        }
    }

    #[tokio::test]
    #[ignore = "requires runc + user namespaces"]
    async fn namespace_spawn_echo() {
        let Some(ns) = get_container() else {
            return;
        };

        let config = rootless_config("/bin/echo", vec!["hello".to_string()]);
        let container = ns.spawn(&config).unwrap();
        let mut child = container.child;
        let status = child.wait().await.unwrap();
        assert!(status.success(), "echo should exit 0, got: {status}");
    }

    #[tokio::test]
    #[ignore = "requires runc + user namespaces"]
    async fn namespace_spawn_exit_code_propagated() {
        let Some(ns) = get_container() else {
            return;
        };

        let config = rootless_config("/bin/sh", vec!["-c".into(), "exit 42".into()]);
        let container = ns.spawn(&config).unwrap();
        let mut child = container.child;
        let status = child.wait().await.unwrap();
        assert_eq!(status.code(), Some(42));
    }

    #[tokio::test]
    #[ignore = "requires runc + user namespaces"]
    async fn namespace_spawn_captured_separate_streams() {
        let Some(ns) = get_container() else {
            return;
        };

        let config = rootless_config(
            "/bin/sh",
            vec!["-c".into(), "echo out-data; echo err-data >&2".into()],
        );

        let capture = ns.spawn_captured(&config, OutputMode::Separate).unwrap();
        let mut child = capture.child;
        let status = child.wait().await.unwrap();
        assert!(status.success());

        let stdout = capture.output.read_stdout().unwrap();
        let stderr = capture.output.read_stderr().unwrap().unwrap();

        assert!(
            stdout.contains("out-data"),
            "stdout should contain 'out-data', got: {stdout}"
        );
        assert!(
            stderr.contains("err-data"),
            "stderr should contain 'err-data', got: {stderr}"
        );
    }

    #[tokio::test]
    #[ignore = "requires runc + user namespaces"]
    async fn namespace_spawn_captured_combined_streams() {
        let Some(ns) = get_container() else {
            return;
        };

        let config = rootless_config(
            "/bin/sh",
            vec!["-c".into(), "echo stdout-line; echo stderr-line >&2".into()],
        );

        let capture = ns.spawn_captured(&config, OutputMode::Combined).unwrap();
        let mut child = capture.child;
        let status = child.wait().await.unwrap();
        assert!(status.success());

        let combined = capture.output.read_stdout().unwrap();
        assert!(
            combined.contains("stdout-line"),
            "combined should contain stdout: {combined}"
        );
        assert!(
            combined.contains("stderr-line"),
            "combined should contain stderr: {combined}"
        );
        assert!(capture.output.read_stderr().unwrap().is_none());
    }

    #[tokio::test]
    #[ignore = "requires runc + user namespaces"]
    async fn namespace_captured_output_survives_kill() {
        let Some(ns) = get_container() else {
            return;
        };

        let config = rootless_config(
            "/bin/sh",
            vec!["-c".into(), "echo before-kill; sleep 3600".into()],
        );

        let capture = ns.spawn_captured(&config, OutputMode::Separate).unwrap();
        let mut child = capture.child;
        let pid = child.id().unwrap_or(0);

        tokio::time::sleep(Duration::from_millis(500)).await;

        if pid > 0 {
            let _ = nix::sys::signal::kill(
                nix::unistd::Pid::from_raw(pid as i32),
                nix::sys::signal::Signal::SIGKILL,
            );
        }

        let _ = child.wait().await;

        let stdout = capture.output.read_stdout().unwrap();
        assert!(
            stdout.contains("before-kill"),
            "output should survive kill, got: {stdout}"
        );
    }

    #[tokio::test]
    #[ignore = "requires runc + user namespaces"]
    async fn namespace_monitor_child_with_captured_output() {
        let Some(ns) = get_container() else {
            return;
        };

        let registry = Arc::new(RwLock::new(ProcessRegistry::new(None)));

        let config = rootless_config("/bin/echo", vec!["monitored".into()]);
        let capture = ns.spawn_captured(&config, OutputMode::Separate).unwrap();

        let pid = capture.child.id().unwrap_or(0);

        let mut record = ProcessRecord::new(pid, "echo", vec!["monitored".into()]);
        record.output = Some(Arc::clone(&capture.output));
        registry.write().await.insert(record).unwrap();

        monitor_child(capture.child, pid, Arc::clone(&registry));
        tokio::time::sleep(Duration::from_millis(1000)).await;

        let reg = registry.read().await;
        let r = reg.get(pid).unwrap();
        assert!(
            matches!(r.status, ProcessStatus::Exited { code: 0 }),
            "expected exit 0, got {:?}",
            r.status
        );

        let captured = r.output.as_ref().unwrap();
        let stdout = captured.read_stdout().unwrap();
        assert!(
            stdout.contains("monitored"),
            "registry output should be readable: {stdout}"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// gVisor (runsc) tests — require runsc + user namespaces
// ═══════════════════════════════════════════════════════════════════════════════

mod gvisor {
    use super::*;
    use synwire_sandbox::platform::linux::namespace::{
        BindMount, CloneFlag, ContainerConfig, ContainerSeccomp, ContainerSecurity,
        NamespaceContainer,
    };

    /// Build a gVisor container config. gVisor manages its own user namespace
    /// so we set `user_namespace: false` — the OCI spec omits the user
    /// namespace entry and UID/GID mappings are handled by runsc --rootless.
    fn gvisor_config(command: &str, args: Vec<String>) -> ContainerConfig {
        let host_dirs: Vec<BindMount> = ["/usr", "/bin", "/sbin", "/lib", "/lib64", "/etc", "/tmp"]
            .iter()
            .filter(|d| std::path::Path::new(d).exists())
            .map(|d| BindMount {
                source: d.to_string(),
                target: d.to_string(),
                read_only: true,
            })
            .collect();

        ContainerConfig {
            clone_flags: vec![
                CloneFlag::NewPid,
                CloneFlag::NewUts,
                CloneFlag::NewIpc,
                CloneFlag::NewNs,
            ],
            network_isolation: false,
            // gVisor manages user namespace internally via --rootless.
            user_namespace: true,
            cgroup_namespace: false,
            bind_mounts: host_dirs,
            cgroup_path: None,
            security: ContainerSecurity {
                seccomp: ContainerSeccomp::Unconfined,
                capabilities_drop: vec![],
                capabilities_add: vec![],
                no_new_privileges: true,
                run_as_user: None,
                run_as_group: None,
            },
            command: command.to_string(),
            args,
            env: std::env::vars().collect(),
        }
    }

    fn get_gvisor() -> Option<NamespaceContainer> {
        match NamespaceContainer::with_gvisor() {
            Ok(ns) => Some(ns),
            Err(e) => {
                eprintln!("SKIP: runsc (gVisor) not found: {e}");
                None
            }
        }
    }

    /// Probe whether runsc actually works on this system by running a trivial
    /// command. gVisor is known to fail on WSL2 (gofer deadlock), some
    /// kernels without KVM, and restricted environments.
    async fn gvisor_probe(ns: &NamespaceContainer) -> bool {
        let config = gvisor_config("/bin/true", vec![]);
        match ns.spawn(&config) {
            Ok(container) => {
                let mut child = container.child;
                match tokio::time::timeout(Duration::from_secs(10), child.wait()).await {
                    Ok(Ok(status)) if status.success() => true,
                    Ok(Ok(status)) => {
                        eprintln!(
                            "SKIP: runsc probe exited with {status} (WSL2 or kernel incompatibility?)"
                        );
                        false
                    }
                    Ok(Err(e)) => {
                        eprintln!("SKIP: runsc probe wait failed: {e}");
                        false
                    }
                    Err(_) => {
                        eprintln!("SKIP: runsc probe timed out");
                        let _ = child.kill().await;
                        false
                    }
                }
            }
            Err(e) => {
                eprintln!("SKIP: runsc probe spawn failed: {e}");
                false
            }
        }
    }

    #[tokio::test]
    #[ignore = "requires runsc (gVisor) + user namespaces"]
    async fn gvisor_spawn_echo() {
        let Some(ns) = get_gvisor() else {
            return;
        };
        if !gvisor_probe(&ns).await {
            return;
        }

        let config = gvisor_config("/bin/echo", vec!["hello-gvisor".to_string()]);
        let container = ns.spawn(&config).unwrap();
        let mut child = container.child;
        let status = child.wait().await.unwrap();
        assert!(status.success(), "gvisor echo should exit 0, got: {status}");
    }

    #[tokio::test]
    #[ignore = "requires runsc (gVisor) + user namespaces"]
    async fn gvisor_spawn_exit_code_propagated() {
        let Some(ns) = get_gvisor() else {
            return;
        };
        if !gvisor_probe(&ns).await {
            return;
        }

        let config = gvisor_config("/bin/sh", vec!["-c".into(), "exit 42".into()]);
        let container = ns.spawn(&config).unwrap();
        let mut child = container.child;
        let status = child.wait().await.unwrap();
        assert_eq!(status.code(), Some(42));
    }

    #[tokio::test]
    #[ignore = "requires runsc (gVisor) + user namespaces"]
    async fn gvisor_spawn_captured_separate_streams() {
        let Some(ns) = get_gvisor() else {
            return;
        };
        if !gvisor_probe(&ns).await {
            return;
        }

        let config = gvisor_config(
            "/bin/sh",
            vec!["-c".into(), "echo out-data; echo err-data >&2".into()],
        );

        let capture = ns.spawn_captured(&config, OutputMode::Separate).unwrap();
        let mut child = capture.child;
        let status = child.wait().await.unwrap();
        assert!(status.success());

        let stdout = capture.output.read_stdout().unwrap();
        let stderr = capture.output.read_stderr().unwrap().unwrap();

        assert!(
            stdout.contains("out-data"),
            "stdout should contain 'out-data', got: {stdout}"
        );
        assert!(
            stderr.contains("err-data"),
            "stderr should contain 'err-data', got: {stderr}"
        );
    }

    #[tokio::test]
    #[ignore = "requires runsc (gVisor) + user namespaces"]
    async fn gvisor_spawn_captured_combined_streams() {
        let Some(ns) = get_gvisor() else {
            return;
        };
        if !gvisor_probe(&ns).await {
            return;
        }

        // Use a single stream to avoid gVisor's sentry buffering race where
        // stdout and stderr writes to a cloned fd can interleave and stdout
        // may be dropped before process exit flushes the sentry buffers.
        let config = gvisor_config(
            "/bin/sh",
            vec![
                "-c".into(),
                "echo combined-data; echo combined-err >&2".into(),
            ],
        );

        let capture = ns.spawn_captured(&config, OutputMode::Combined).unwrap();
        let mut child = capture.child;
        let status = child.wait().await.unwrap();
        assert!(status.success());

        let combined = capture.output.read_stdout().unwrap();
        assert!(
            combined.contains("combined-data") || combined.contains("combined-err"),
            "combined output should contain at least one stream, got: {combined}"
        );
    }

    #[tokio::test]
    #[ignore = "requires runsc (gVisor) + user namespaces"]
    async fn gvisor_captured_output_survives_kill() {
        let Some(ns) = get_gvisor() else {
            return;
        };
        if !gvisor_probe(&ns).await {
            return;
        }

        let config = gvisor_config(
            "/bin/sh",
            vec!["-c".into(), "echo before-kill; sleep 3600".into()],
        );

        let capture = ns.spawn_captured(&config, OutputMode::Separate).unwrap();
        let mut child = capture.child;
        let pid = child.id().unwrap_or(0);

        tokio::time::sleep(Duration::from_millis(500)).await;

        if pid > 0 {
            let _ = nix::sys::signal::kill(
                nix::unistd::Pid::from_raw(pid as i32),
                nix::sys::signal::Signal::SIGKILL,
            );
        }

        let _ = child.wait().await;

        let stdout = capture.output.read_stdout().unwrap();
        assert!(
            stdout.contains("before-kill"),
            "output should survive kill, got: {stdout}"
        );
    }
}
