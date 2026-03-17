# Contract: Release Artifact Naming & Structure

## Binary Name

`synwire-mcp-server` (matches the `[[bin]]` name in `crates/synwire-mcp-server/Cargo.toml`)

## Archive Naming Convention

```
synwire-mcp-server_<version>_<os>_<arch>.tar.gz
```

Examples:
```
synwire-mcp-server_v0.1.0_linux_amd64.tar.gz
synwire-mcp-server_v0.1.0_linux_arm64.tar.gz
synwire-mcp-server_v0.1.0_darwin_amd64.tar.gz
synwire-mcp-server_v0.1.0_darwin_arm64.tar.gz
```

## Checksum File

Single file attached to each release:
```
checksums.txt
```

Format (SHA-256):
```
<sha256hex>  synwire-mcp-server_v0.1.0_linux_amd64.tar.gz
<sha256hex>  synwire-mcp-server_v0.1.0_linux_arm64.tar.gz
<sha256hex>  synwire-mcp-server_v0.1.0_darwin_amd64.tar.gz
<sha256hex>  synwire-mcp-server_v0.1.0_darwin_arm64.tar.gz
```

## Platform Targets

| GoReleaser os | GoReleaser arch | Rust target | Runner | Linker |
|---|---|---|---|---|
| `linux` | `amd64` | `x86_64-unknown-linux-musl` | `ubuntu-latest` | cargo-zigbuild (native) |
| `linux` | `arm64` | `aarch64-unknown-linux-musl` | `ubuntu-latest` | cargo-zigbuild (cross) |
| `darwin` | `amd64` | `x86_64-apple-darwin` | `macos-latest` | native |
| `darwin` | `arm64` | `aarch64-apple-darwin` | `macos-latest` | native |

## GoReleaser Staging Path

GoReleaser prebuilt builder expects binaries at:
```
artifacts/<os>_<arch>[_v1]/<binary>
```

Exact layout populated by matrix build jobs:
```
artifacts/
├── linux_amd64_v1/synwire-mcp-server
├── linux_arm64/synwire-mcp-server
├── darwin_amd64_v1/synwire-mcp-server
└── darwin_arm64/synwire-mcp-server
```

The `_v1` suffix is the GoReleaser `goamd64` variant; it is present for `amd64` targets and absent for `arm64`.

## Archive Contents

Each `.tar.gz` contains:
```
synwire-mcp-server   # the binary (executable)
```

## Release Assets on GitHub

After a successful release, the GitHub Release page will contain:
- 4 `.tar.gz` archives (one per platform)
- 1 `checksums.txt`
- Auto-generated source code archives (GitHub default, cannot be disabled)
