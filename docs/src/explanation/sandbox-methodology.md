# Sandbox Architecture Methodology

This document explains *why* synwire's process sandbox is designed the way it
is. For setup instructions, see the [Process Sandboxing](../how-to/process-sandbox.md)
how-to guide. For a hands-on walkthrough, see the
[Sandboxed Command Execution](../tutorials/10-sandboxed-agent.md) tutorial.

## Design philosophy

An AI agent that can run shell commands is powerful but dangerous. A misguided
`rm -rf /` or a prompt-injection attack that exfiltrates credentials can cause
real damage. Synwire's sandbox exists to bound the blast radius of agent
actions, but it must do so without becoming an obstacle to agent performance
or developer ergonomics.

Five principles guide the design:

1. **Lightweight and rootless.** The sandbox must work without `sudo`, without
   a system daemon, and without asking the user to reconfigure their machine
   beyond what a standard Linux desktop already provides. An unprivileged user
   account is the only requirement.

2. **Sub-second startup.** Every tool call in an agent loop spawns a container.
   If container creation takes seconds, the agent's end-to-end latency becomes
   dominated by sandbox overhead rather than LLM inference. We target under
   50ms for the common case.

3. **Ephemeral by default.** Containers are created, used for a single command,
   and destroyed. There is no persistent container state, no image registry, no
   layer cache. The host filesystem is bind-mounted directly.

4. **PTY support for human-in-the-loop.** Many real-world CLI tools require
   interactive confirmation: `terraform apply`, `ssh` host key prompts, `gpg`
   passphrase entry. The sandbox must provide a pseudo-terminal so that an
   `expect`-style automation layer can drive these interactions.

5. **Output survives process kills.** When an agent exceeds its resource budget
   and the cgroup kills its processes, the partial output must still be
   recoverable. File-backed capture (rather than in-memory pipe buffers)
   ensures this.

## Why not Docker?

Docker is the most widely-known container runtime, so the question "why not
just use Docker?" deserves a thorough answer. There are six reasons, each
sufficient on its own but collectively decisive.

### Docker requires a daemon

`dockerd` runs as root (or, in rootless mode, as a user daemon with its own
network namespace stack). An agent framework should not depend on a system
service being up and correctly configured. If `dockerd` crashes, restarts, or
is not installed, every agent in the system stops working. Synwire's approach
--- invoking an OCI runtime binary directly --- has no daemon dependency. The
runtime binary (`runc` or `runsc`) is a standalone executable with no
long-running state.

### Docker socket access is root-equivalent

The common pattern of mounting `/var/run/docker.sock` into a container grants
that container full control over the Docker daemon, which runs as root. This
means any container with socket access can create privileged containers, mount
the host filesystem, and effectively become root on the host. This is not a
theoretical concern --- it is a well-documented container escape vector. For
an AI agent sandbox, where the entire purpose is to *constrain* what the agent
can do, granting Docker socket access defeats the goal entirely.

### Docker startup latency is too high

Creating a Docker container involves multiple steps: the client sends a request
to `dockerd`, which communicates with `containerd`, which calls the OCI
runtime. Image layers may need to be pulled, unpacked, or verified. Even with
a warm image cache, container creation typically takes 1--5 seconds. By
contrast, `runc run` with a pre-built OCI bundle on a local filesystem
completes in roughly 50ms. Over a 20-step agent loop, this difference is
between 1 second and nearly 2 minutes of pure sandbox overhead.

### Docker is designed for services, not ephemeral commands

Docker's architecture --- images, layers, registries, build caches, named
volumes, networks --- is optimised for long-running services. An agent sandbox
needs none of this. We bind-mount the host working directory into a minimal
rootfs, run a single command, collect the output, and tear everything down.
The image/layer abstraction adds complexity (and latency) without providing
value in this use case.

### Rootless Docker exists but is fragile

Docker does offer a rootless mode (`dockerd-rootless-setuptool.sh`), but it
introduces its own stack of dependencies: `slirp4netns` or `pasta` for
networking, `fuse-overlayfs` for the storage driver, and a user-space
`dockerd` that must be running. Filesystem permission edge cases are common
(files created inside the container may have unexpected ownership on the
host). The rootless stack is less tested than the standard root-mode stack
and has historically been a source of hard-to-debug issues.

### We need the OCI runtime, not the orchestrator

Docker internally delegates to `containerd`, which delegates to `runc` (or
another OCI-compliant runtime). The Docker daemon and `containerd` provide
orchestration features --- image management, networking, health checks,
restart policies --- that an ephemeral agent sandbox does not need. By
calling the OCI runtime directly, we cut out two layers of indirection and
their associated latency, complexity, and failure modes.

## Linux: runc and gVisor two-tier model

On Linux, synwire supports two OCI runtimes via the `OciRuntime` enum, each
representing a different point on the isolation/performance trade-off curve.

### runc: namespace isolation

runc is the reference OCI runtime, maintained by the Open Container
Initiative. It uses Linux kernel namespaces (PID, mount, UTS, IPC, network,
user) combined with seccomp BPF filters and a minimal capability set to
isolate the container process.

The key characteristic of runc is that container processes share the host
kernel. Isolation relies on the kernel correctly enforcing namespace
boundaries. This is well-understood and battle-tested --- it is the same
mechanism that Docker, Podman, and Kubernetes use --- but it means that a
kernel vulnerability in namespace handling could theoretically allow escape.

For agent workloads, this trade-off is usually acceptable. The threat model
is typically *accidental* damage (the LLM generates a destructive command)
rather than *adversarial* kernel exploitation. runc provides strong boundaries
against accidental damage with minimal overhead: container startup takes
roughly 50ms.

### gVisor: user-space kernel

gVisor (`runsc`) provides a fundamentally different isolation model. Instead
of sharing the host kernel, gVisor interposes a user-space kernel called the
*Sentry* between the container and the host. Every syscall from the
containerised process is intercepted and re-implemented by the Sentry in Go.
The Sentry itself runs with a minimal set of host syscalls, so even if the
containerised process triggers a bug in syscall handling, the blast radius is
confined to the Sentry process rather than the host kernel.

This provides substantially stronger isolation: kernel exploits that would
escape a namespace-based container are blocked because the container never
interacts with the real kernel. The trade-off is higher startup latency
(roughly 200ms) and some syscall compatibility limitations (the Sentry does
not implement every Linux syscall). For untrusted code execution or
multi-tenant scenarios, this trade-off is worthwhile.

### Systrap vs ptrace

gVisor supports two mechanisms for intercepting syscalls from container
processes:

- **Systrap** patches syscall instruction sites in memory at runtime, replacing
  `syscall` instructions with traps that the Sentry handles directly. This is
  faster (roughly 10% overhead vs native) but requires `CAP_SYS_PTRACE` in
  the sandbox's ambient capability set.

- **Ptrace** uses Linux's `PTRACE_SYSEMU` and `CLONE_PTRACE` to intercept
  syscalls. It is slower but universally compatible, requiring no special
  capabilities.

Synwire probes systrap on first use by running a trivial container
(`/bin/true`). If it succeeds, systrap is used for all subsequent containers
in the process. If it fails --- commonly because rootless mode with host
networking does not propagate `CAP_SYS_PTRACE` --- synwire falls back to
ptrace and caches the decision. There are no repeated probes and no user
configuration required.

### Cgroup v2 resource accounting

Independent of which OCI runtime is selected, synwire tracks resource
consumption via cgroup v2. The `CgroupV2Manager` creates per-agent cgroups
as siblings of the synwire process's own cgroup:

```text
user@1000.service/
  app.slice/
    code.scope/          <-- synwire process lives here
    synwire/
      agents/<uuid>/     <-- agent cgroups go here
```

Placing agent cgroups under the process cgroup's *parent* (rather than as
children) avoids the cgroup v2 "no internal processes" constraint: a cgroup
that enables subtree controllers must not have processes of its own. Nesting
under the parent keeps the hierarchy close to the synwire process while
remaining legal under cgroup v2 rules.

Resource limits (CPU, memory, PIDs) are written to the agent cgroup's control
files. When the agent terminates, the `CgroupV2Manager`'s `Drop`
implementation writes `1` to `cgroup.kill` (Linux 5.14+) for immediate
cleanup, falling back to per-PID `SIGKILL` on older kernels.

## macOS: Seatbelt and container runtimes

macOS has no equivalent of Linux namespaces. The kernel does not support PID,
mount, or network namespaces, so the Linux approach of calling an OCI runtime
directly is not possible. Synwire provides two isolation tiers on macOS, each
using platform-native mechanisms.

### Seatbelt: policy enforcement

Apple's Seatbelt framework (`sandbox-exec`) provides a deny-by-default policy
enforcement mechanism. Synwire generates Sandbox Profile Language (SBPL)
profiles from `SandboxConfig` at runtime, then spawns the agent command via
`sandbox-exec -p <profile> -- <command>`.

Seatbelt is lightweight --- effectively zero overhead beyond the policy
check on each syscall --- and requires no additional software. Its limitations
are significant, though: there is no process namespace (the sandboxed process
can see all host processes via `ps`), no network namespace, and no resource
limits. Seatbelt constrains *what* a process can access but not *how much*
CPU or memory it can consume.

Additionally, Apple has deprecated the public `sandbox-exec` interface. It
continues to work on current macOS versions and is widely used by build
systems (Nix, Bazel), but Apple has not provided a public replacement. Synwire
will migrate if one becomes available.

### Strong isolation via container runtimes

For stronger isolation on macOS, a Linux kernel is required --- which means
a virtual machine. Synwire supports four container runtimes, detected in
priority order by `detect_container_runtime()`:

1. **Apple Container** (preferred)
2. **Docker Desktop** (widely installed)
3. **Podman** (fallback)
4. **Colima** (last resort)

#### Apple Container: lightweight Linux VMs via Virtualization.framework

[Apple Container](https://github.com/apple/container) is Apple's first-party
tool for running Linux containers as lightweight virtual machines. It uses
macOS Virtualization.framework directly, avoiding the overhead of a
general-purpose VM manager. Containers start as minimal Linux VMs with
automatic file sharing and port forwarding, similar in spirit to Podman
Machine but with tighter system integration and lower overhead since the
hypervisor layer is built into macOS itself.

Apple Container requires **macOS 26 (Tahoe) or later** and **Apple Silicon**.
When these requirements are met, it is the preferred runtime because:

- It is maintained by Apple and uses the same Virtualization.framework that
  powers macOS's own virtualisation features.
- No third-party daemon or VM manager is needed --- `container` is a single
  standalone binary.
- Startup latency is lower than Podman Machine because Virtualization.framework
  VMs are purpose-built for lightweight workloads.

Synwire translates `SandboxConfig` into `container run` flags (volumes,
resource limits, network policy) in the same way it does for Podman.

#### Docker Desktop: daemon-based containers for macOS

Docker Desktop has the largest install base of any container runtime on macOS.
While synwire does not use Docker on Linux (see
[Why not Docker?](#why-not-docker) above --- the daemon model, root-equivalence
concerns, and startup latency make it unsuitable for direct OCI runtime use),
the macOS situation is different: *every* macOS container runtime already runs
a Linux VM, so the daemon-in-a-VM architecture does not add an extra layer of
indirection the way it does on Linux.

Synwire detects Docker Desktop by running `docker version` (not
`docker --version`). The `--version` flag only checks the CLI binary; `docker
version` queries the daemon and fails if the Docker Desktop VM is not running.
This avoids false positives where the CLI is installed but the backend is
stopped.

Docker Desktop, Podman, and Colima share identical CLI flag semantics
(`docker run` / `podman run`), so synwire translates `SandboxConfig` into the
same set of flags for all three: `--volume`, `--network`, `--memory`, `--cpus`,
`--user`, and `--security-opt no-new-privileges`.

> **Note on Linux:** Docker Desktop is *not* supported on Linux. On Linux,
> synwire calls OCI runtimes (`runc`, `runsc`) directly, bypassing any daemon.
> Docker Desktop is only used on macOS where VM-based isolation is already the
> norm.

#### Podman: OCI containers in a managed Linux VM

Podman (`podman machine`) runs a lightweight Linux VM and manages OCI
containers inside it. Synwire translates `SandboxConfig` into
`podman run --rm` flags (volumes, network, memory, CPU limits).

Podman is the fallback when neither Apple Container nor Docker Desktop is
available --- either because the Mac is running macOS 25 or earlier without
Docker, or because it has an Intel processor without Docker Desktop installed.
Podman supports both Apple Silicon and Intel Macs and has no macOS version
restriction beyond what Homebrew requires.

#### Colima: lightweight Docker-compatible VM

Colima wraps Lima to provide a Docker-compatible environment with minimal
configuration. Unlike bare Lima (which uses `limactl shell` to run commands
inside a VM), Colima exposes a Docker socket so that the standard `docker run`
CLI works transparently.

Synwire detects Colima by running `colima status` to check that the Colima VM
is running, then delegates to `docker run` for container execution. Because
Colima surfaces a Docker-compatible socket, it shares the same CLI flag
semantics as Docker Desktop and Podman.

Colima is the last-resort runtime, used only when Apple Container, Docker
Desktop, and Podman are all unavailable. It provides a functional but less
integrated experience compared to the other options.

## The OCI runtime spec as the unifying abstraction

Synwire does not generate runtime-specific command-line flags. Instead, it
produces an [OCI runtime specification][oci-spec] --- a JSON document
(`config.json`) placed in a bundle directory --- and hands that bundle to
whichever runtime is selected.

[oci-spec]: https://github.com/opencontainers/runtime-spec

This design provides several benefits:

- **Runtime portability.** The same spec works with runc, gVisor (`runsc`),
  crun, youki, and any future OCI-compliant runtime. Switching runtimes is a
  configuration change, not a code change.

- **Compile-time correctness.** The `oci-spec` Rust crate provides typed
  builders (`SpecBuilder`, `ProcessBuilder`, `LinuxBuilder`, etc.) that catch
  field-name mistakes and missing required fields at compile time. A typo in a
  namespace type or a missing mount option is a compilation error, not a
  runtime `EINVAL`.

- **Inspectability.** The spec is a JSON file on disk. When debugging
  container issues, developers can read the generated `config.json` directly,
  modify it, and re-run the container manually with
  `runc run --bundle /tmp/synwire-xxx test-id`. No opaque API calls to
  reverse-engineer.

The internal pipeline is:

```text
SandboxConfig --> ContainerConfig --> oci_spec::runtime::Spec --> config.json
```

`SandboxConfig` is synwire's user-facing configuration type (security presets,
filesystem paths, resource limits). `ContainerConfig` is the platform-specific
intermediate representation. The `build_oci_spec` function in the Linux
namespace module transforms `ContainerConfig` into a typed `Spec` via the
`oci-spec` builders, which is then serialised to JSON in the bundle directory.

## PTY and expect integration

Many CLI tools that agents need to drive require interactive input: `terraform
apply` asks for confirmation, `ssh` prompts for host key verification, `gpg`
requests a passphrase. Piping "yes" to stdin is fragile and tool-specific.
A pseudo-terminal (PTY) with pattern-matching automation is the robust
solution.

The OCI runtime spec supports a `terminal: true` flag that tells the runtime
to allocate a PTY inside the container. The runtime delivers the PTY
controller file descriptor to the caller via a Unix domain socket (the
`--console-socket` mechanism), using `SCM_RIGHTS` ancillary data to pass the
fd across process boundaries.

Synwire wraps the received fd in an [`expectrl`][expectrl] session, which
provides goexpect-equivalent pattern matching: wait for a regex, send a
response, set timeouts. This works cross-platform --- `expectrl` handles the
differences between Linux and macOS PTY allocation internally.

[expectrl]: https://crates.io/crates/expectrl

The file-descriptor handoff is the key design choice. Rather than running
`docker exec -it` (which requires a running container and a daemon), the
`--console-socket` mechanism gives synwire direct ownership of the PTY
controller fd immediately after container creation. There is no intermediary
process, no daemon RPC, and no risk of the PTY being torn down by a
container lifecycle event.

## Comparison

| | Docker (Linux) | runc (synwire) | gVisor (synwire) | macOS Seatbelt | Apple Container | Docker Desktop (macOS) | Podman (macOS) | Colima (macOS) |
|---|---|---|---|---|---|---|---|---|
| Requires daemon | Yes | No | No | No | No | Yes (VM) | Yes (VM) | Yes (VM) |
| Requires root | Yes\* | No | No | No | No | No | No | No |
| Startup latency | 1--5s | ~50ms | ~200ms | ~10ms | ~100ms | ~500ms | ~500ms | ~500ms |
| Kernel isolation | Namespaces | Namespaces | User-space kernel | Policy enforcement | VM (Virtualization.framework) | VM + namespaces | VM + namespaces | VM + namespaces |
| PTY support | `docker exec -it` | `--console-socket` | `--console-socket` | Native | Native | `docker exec -it` | `podman exec -it` | `docker exec -it` |
| Syscall filtering | Seccomp | Seccomp | Sentry kernel | SBPL | Full Linux kernel | Seccomp | Seccomp | Seccomp |
| Resource limits | cgroups | cgroups v2 | cgroups v2 | None | VM-level | cgroups v2 (in VM) | cgroups v2 (in VM) | cgroups v2 (in VM) |
| macOS requirement | N/A | N/A | N/A | Any macOS | macOS 26+, Apple Silicon | Any macOS | Any macOS | Any macOS |

\* Rootless Docker exists but introduces significant complexity; see
[Why not Docker?](#why-not-docker) above.
