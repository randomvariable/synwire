# Process Sandboxing

Synwire isolates agent-spawned processes using Linux cgroup v2 for resource
accounting and OCI container runtimes for namespace isolation. Two runtimes
are supported:

| Runtime | Binary | Isolation model |
|---------|--------|-----------------|
| **runc** | `runc` | Linux namespaces + seccomp — processes share the host kernel |
| **gVisor** | `runsc` | User-space kernel — syscalls are intercepted by a Go-based sentry, providing a much stronger isolation boundary |

## Prerequisites

| Requirement | Minimum version | Purpose |
|-------------|----------------|---------|
| Linux kernel | 4.15 | cgroup v2 unified hierarchy |
| systemd | 239 | User cgroup delegation |
| runc | 1.1+ | Namespace isolation (standard) |
| runsc (gVisor) | latest | Namespace isolation (hardened) — *optional* |

> **WSL2 note**: cgroup v2 is available but user delegation may not be
> enabled by default — see the [WSL2 section](#wsl2) below.

## Architecture

Synwire uses battle-tested OCI runtimes for namespace isolation instead of a
custom init binary. These runtimes handle all namespace, mount, seccomp, and
capability setup including hardening against known CVEs.

For each container, synwire:

1. Creates a temporary OCI bundle directory
2. Generates an OCI runtime spec (`config.json`) from the `SandboxConfig`
3. Generates `/etc/passwd` and `/etc/group` so the current user is
   resolvable inside the container (`whoami`, `id`, `ls -la` all work)
4. Runs `runc run --bundle <dir> <id>` (or `runsc --rootless run ...` for gVisor)
5. Cleans up the bundle when the container exits

### Runtime selection

```rust,ignore
use synwire_sandbox::platform::linux::namespace::NamespaceContainer;

// Standard runc — finds "runc" on $PATH
let container = NamespaceContainer::new()?;

// gVisor — finds "runsc" on $PATH
let container = NamespaceContainer::with_gvisor()?;

// Explicit selection
use synwire_sandbox::platform::linux::namespace::OciRuntime;
let container = NamespaceContainer::with_runtime(OciRuntime::Gvisor)?;
```

### User namespace and UID mapping

Rootless user namespaces only allow a single UID/GID mapping entry (without
the setuid `newuidmap` helper). runc's init process requires UID 0, so synwire
maps `containerID 0 → hostID <real-uid>`. The process runs as UID 0 inside
the namespace, which the kernel translates to the real UID for all host-side
operations (file ownership in bind mounts, etc.).

A generated `/etc/passwd` maps UID 0 to the real username — the same trick
Podman uses for rootless containers. Inside the container:

```text
$ whoami
naadir
$ id
uid=0(naadir) gid=0(naadir) groups=0(naadir)
$ touch /tmp/test && ls -la /tmp/test
-rw-r--r-- 1 naadir naadir 0 Mar 16 12:00 /tmp/test
```

### Capabilities

The default capability set is intentionally minimal — much tighter than
Docker's default:

| Capability | Purpose |
|------------|---------|
| `CAP_KILL` | Signal child processes spawned by the agent |
| `CAP_NET_BIND_SERVICE` | Bind ports <1024 if networking is enabled |
| `CAP_SETPCAP` | Drop further capabilities (supports `no_new_privileges`) |

Dropped from Docker's default: `CHOWN`, `DAC_OVERRIDE`, `FSETID`, `FOWNER`,
`SETGID`, `SETUID`, `SYS_CHROOT`, `AUDIT_WRITE`. Use `capabilities_add` in
`SandboxConfig` to grant additional capabilities if a specific use case
requires them.

### gVisor differences

When using `OciRuntime::Gvisor`, synwire adjusts its behaviour automatically:

- **No user namespace** in the OCI spec — runsc manages its own user
  namespace via `--rootless`
- **No UID/GID mappings** — handled internally by runsc
- **No seccomp profile** — gVisor's sentry kernel provides stronger syscall
  filtering than BPF-based seccomp; applying both causes compatibility issues
- **Platform auto-detected** — probes systrap first, falls back to ptrace if
  needed (see [Platform auto-detection](#platform-auto-detection) below)

### cgroup hierarchy

Agent cgroups are placed as siblings of the synwire process's own cgroup:

```text
user@1000.service/
  app.slice/
    code.scope/          ← synwire process lives here
    synwire/
      agents/<uuid>/     ← agent cgroups go here
```

When the `CgroupV2Manager` is dropped (agent terminated), it writes `1` to
`cgroup.kill` (Linux 5.14+) or falls back to `SIGKILL` per PID to ensure
immediate cleanup.

## Installing runc

```bash
# Debian/Ubuntu
sudo apt install runc

# Fedora
sudo dnf install runc

# Arch
sudo pacman -S runc
```

Verify:

```bash
runc --version
```

## Installing gVisor (optional)

gVisor provides a stronger isolation boundary by running a user-space kernel
that intercepts syscalls. Install `runsc`:

```bash
# Download and install runsc
ARCH=$(uname -m)
URL="https://storage.googleapis.com/gvisor/releases/release/latest/${ARCH}"
wget "${URL}/runsc" "${URL}/runsc.sha512"
sha512sum -c runsc.sha512
chmod a+rx runsc
sudo mv runsc /usr/local/bin/
```

Or via a package manager (where available):

```bash
# Arch (AUR)
yay -S gvisor-bin
```

Verify:

```bash
runsc --version
```

### Platform auto-detection

gVisor supports two syscall interception platforms:

| Platform | Mechanism | Performance | Compatibility |
|----------|-----------|-------------|---------------|
| **systrap** (default) | Patches syscall instruction sites | Fastest | Requires `CAP_SYS_PTRACE` |
| **ptrace** | `PTRACE_SYSEMU` / `CLONE_PTRACE` | Slower | Universal |

On first use, synwire automatically probes whether systrap works by running
a trivial container (`/bin/true`). If systrap succeeds, it is used for all
future containers in the process. If it fails, synwire falls back to ptrace,
logs a warning, and caches the decision for the lifetime of the process —
no repeated probes.

**Why systrap may fail**: In rootless mode with `--network=host`, gVisor has
a bug where `CAP_SYS_PTRACE` is not included in the sandbox's ambient
capabilities. `ConfigureCmdForRootless()` in `runsc/sandbox/sandbox.go`
overwrites `AmbientCaps` without `CAP_SYS_PTRACE` (line 1059), and the
systrap capability check that would add it back is in the `else` branch
that only runs when host networking is *not* used (line 1143). This causes
systrap's `PTRACE_ATTACH` on stub threads to fail with `EPERM`. The ptrace
platform uses `CLONE_PTRACE` from the child instead, avoiding the issue.

When the gVisor bug is fixed upstream, the probe will succeed automatically
and synwire will use systrap — no code change needed.

## Enabling cgroup v2 delegation

### Verify cgroup v2 is mounted

```bash
# Should show cgroup2 filesystem
mount | grep cgroup2

# Should list available controllers
cat /sys/fs/cgroup/cgroup.controllers
```

If `/sys/fs/cgroup/cgroup.controllers` does not exist, add
`systemd.unified_cgroup_hierarchy=1` to your kernel command line and reboot.

### Verify user delegation

```bash
# Show your process's cgroup
cat /proc/self/cgroup
# Output: 0::/user.slice/user-1000.slice/user@1000.service/app.slice/...

# Check the parent is writable
CGROUP_PATH=$(sed -n 's|0::|/sys/fs/cgroup|p' /proc/self/cgroup)
PARENT=$(dirname "$CGROUP_PATH")
ls -la "$PARENT"/cgroup.subtree_control
```

If the parent cgroup is not writable, ensure systemd user sessions are
enabled:

```bash
systemctl --user status

# If "Failed to connect to bus":
loginctl enable-linger $USER
```

### Enable controller delegation

If controllers (cpu, memory, pids) are not available in the user cgroup:

```bash
sudo mkdir -p /etc/systemd/system/user@.service.d
sudo tee /etc/systemd/system/user@.service.d/delegate.conf <<'EOF'
[Service]
Delegate=cpu cpuset io memory pids
EOF

sudo systemctl daemon-reload
# Log out and back in, or:
sudo systemctl restart user@$(id -u).service
```

Verify:

```bash
cat /sys/fs/cgroup/user.slice/user-$(id -u).slice/user@$(id -u).service/cgroup.subtree_control
# Should show: cpu io memory pids
```

## WSL2

WSL2 runs a custom init by default. Add to `/etc/wsl.conf`:

```ini
[boot]
systemd=true
```

Then restart WSL (`wsl --shutdown` from PowerShell).

## Isolation levels

| Level | Mechanism | Requires |
|-------|-----------|----------|
| `CgroupTracking` | cgroup v2 accounting only | user delegation |
| `Namespace` | OCI container via runc (PID/mount/UTS/IPC/net namespaces) | runc + user namespaces |
| `Gvisor` | OCI container via runsc (user-space kernel sandbox) | runsc + user namespaces |

## macOS sandboxing

macOS lacks Linux namespaces and cgroup v2, so synwire uses platform-native
mechanisms: Apple's Seatbelt sandbox for light isolation, and OCI container
runtimes (Docker Desktop, Podman, or Colima) for strong isolation.

### Seatbelt (light isolation)

Seatbelt uses Apple's `sandbox-exec` tool with Sandbox Profile Language (SBPL)
profiles. Synwire generates an SBPL profile from the `SandboxConfig` at
runtime, applying a **deny-by-default** model — all operations are denied
unless explicitly allowed.

> **Deprecation note**: Apple has deprecated `sandbox-exec` and the public
> SBPL interface. It remains functional on current macOS versions and is
> widely used by build systems (Nix, Bazel). Synwire will migrate to a
> replacement if Apple provides one.

#### How profiles are generated

Synwire translates `SandboxConfig` fields into SBPL rules:

| `SandboxConfig` field | SBPL effect |
|----------------------|-------------|
| `network: true` | `(allow network*)` |
| `network: false` | Network operations remain denied |
| `filesystem.read_paths` | `(allow file-read* (subpath "..."))` per path |
| `filesystem.write_paths` | `(allow file-write* (subpath "..."))` per path |
| `filesystem.deny_paths` | `(deny file-read* file-write* (subpath "..."))` — evaluated first |

#### `SecurityPreset` levels

| Preset | Filesystem | Network | Subprocesses |
|--------|-----------|---------|-------------|
| `Baseline` | Read home, read/write workdir and tmpdir | Allowed | Allowed |
| `Privileged` | Read/write home | Allowed | Allowed |
| `Restricted` | Read/write workdir only | Denied | Denied |

#### Example SBPL profile

A `Restricted` preset with a workdir of `/tmp/agent-work` produces:

```scheme
(version 1)
(deny default)

;; Allow basic process execution
(allow process-exec)
(allow process-fork)
(allow sysctl-read)
(allow mach-lookup)

;; Filesystem: workdir read/write
(allow file-read* file-write*
  (subpath "/tmp/agent-work"))

;; Filesystem: system libraries (read-only)
(allow file-read*
  (subpath "/usr/lib")
  (subpath "/usr/share")
  (subpath "/System")
  (subpath "/Library/Frameworks")
  (subpath "/private/var/db/dyld"))

;; Network: denied (restricted preset)
;; Subprocesses: denied (restricted preset)
(deny process-fork (with send-signal SIGKILL))
```

#### Usage

```rust,ignore
use synwire_sandbox::platform::macos::seatbelt::SeatbeltContainer;

let container = SeatbeltContainer::new(config)?;
container.run(command).await?;
```

### Container runtime (strong isolation)

For stronger isolation on macOS, synwire uses a container runtime that runs
Linux in a lightweight VM. Synwire auto-detects the available runtime via
`detect_container_runtime()`, using a four-tier priority order:

```text
Apple Container  >  Docker Desktop  >  Podman  >  Colima
```

```rust,ignore
use synwire_sandbox::platform::macos::container::detect_container_runtime;

let runtime = detect_container_runtime().await?;
// Returns ContainerRuntime::AppleContainer, ContainerRuntime::DockerDesktop,
// ContainerRuntime::Podman, or ContainerRuntime::Colima
```

#### Apple Container (preferred)

[Apple Container](https://github.com/apple/container) is Apple's first-party
tool for running Linux containers as lightweight VMs using macOS
Virtualization.framework. It is the preferred strong-isolation runtime when
available.

**Requirements:** macOS 26+ (Tahoe), Apple Silicon.

##### Installing Apple Container

```bash
# Via Homebrew
brew install apple/container/container

# Or download from GitHub releases
# https://github.com/apple/container/releases
```

Verify:

```bash
container --version
```

> **Note:** Apple Container is preferred over all other runtimes when available
> because it is a first-party Apple tool with tighter system integration and
> lower overhead. If your Mac does not meet the requirements (macOS 26+ and
> Apple Silicon), synwire falls back to Docker Desktop, then Podman, then
> Colima.

#### Docker Desktop (widely installed)

Docker Desktop has the largest install base of any container runtime on macOS,
making it the second-priority option. Although synwire does not use Docker on
Linux (see the
[sandbox methodology](../explanation/sandbox-methodology.md#why-not-docker)
for details on the daemon model concerns), the macOS situation is different:
every macOS container runtime already runs a Linux VM, so the daemon-in-a-VM
architecture does not add an extra layer of indirection.

Synwire checks for Docker Desktop by running `docker version` (not
`docker --version`). The `--version` flag only checks the CLI binary is
installed; `docker version` queries the daemon and fails if the Docker Desktop
VM is not running. This avoids false positives where the CLI is present but
the backend is stopped.

Docker Desktop, Podman, and Colima share identical `docker run` / `podman run`
CLI flag semantics, so synwire translates `SandboxConfig` into the same set
of flags for all three.

##### Installing Docker Desktop

Download and install from [docker.com](https://www.docker.com/products/docker-desktop/).
Launch Docker Desktop and wait for the engine to start.

Verify:

```bash
docker version
```

#### Podman (fallback)

Podman runs a lightweight Linux VM (`podman machine`) and manages OCI
containers inside it. It is the fallback runtime when neither Apple Container
nor Docker Desktop is available. Synwire invokes Podman with the following
flags:

| Flag | Purpose |
|------|---------|
| `--volume <host>:<container>` | Bind-mount working directory |
| `--network none` | Disable networking (when `SandboxConfig` denies it) |
| `--memory <limit>` | Memory cap from `ResourceLimits` |
| `--cpus <count>` | CPU cap from `ResourceLimits` |
| `--user <uid>:<gid>` | Run as non-root inside the container |
| `--security-opt no-new-privileges` | Prevent privilege escalation |

These same flags apply to Docker Desktop and Colima, which share identical
CLI semantics.

##### Installing Podman

```bash
brew install podman
podman machine init
podman machine start
```

Verify:

```bash
podman info
```

#### Colima (last resort)

Colima wraps Lima to provide a Docker-compatible environment with minimal
configuration. Unlike bare Lima, Colima exposes a Docker socket so that the
standard `docker run` CLI works transparently. Synwire detects Colima by
running `colima status` to check that the Colima VM is running, then delegates
to `docker run` for container execution.

Colima is used only when Apple Container, Docker Desktop, and Podman are all
unavailable.

##### Installing Colima

```bash
brew install colima
colima start
```

Verify:

```bash
colima status
docker version   # Should succeed via Colima's Docker socket
```

### Isolation levels (macOS)

| Level | Mechanism | Requires |
|-------|-----------|----------|
| `Seatbelt` | `sandbox-exec` SBPL profiles | macOS (built-in) |
| `Container` (Apple Container) | Lightweight Linux VM via Virtualization.framework | macOS 26+, Apple Silicon, `container` on `$PATH` |
| `Container` (Docker Desktop) | OCI container in Docker Desktop VM | Docker Desktop running (`docker version` succeeds) |
| `Container` (Podman) | OCI container in Podman Machine VM | `podman` on `$PATH` |
| `Container` (Colima) | OCI container via Colima VM + Docker socket | `colima` on `$PATH`, Colima VM running |

### Check user namespace support

```bash
# Should return 1
cat /proc/sys/kernel/unprivileged_userns_clone 2>/dev/null || \
  sysctl kernel.unprivileged_userns_clone

# If 0:
sudo sysctl -w kernel.unprivileged_userns_clone=1
echo 'kernel.unprivileged_userns_clone=1' | sudo tee /etc/sysctl.d/99-userns.conf
```

## Running the tests

```bash
# Unprivileged tests (always work)
cargo test -p synwire-sandbox --test linux_e2e

# cgroup + runc namespace tests (require delegation + runc)
cargo test -p synwire-sandbox --test linux_e2e -- --ignored

# gVisor tests only (require runsc on $PATH)
cargo test -p synwire-sandbox --test linux_e2e gvisor -- --ignored
```

The cgroup tests gracefully skip if delegation is not available. The
namespace tests skip if runc is not found. The gVisor tests skip if
`runsc` is not found.
