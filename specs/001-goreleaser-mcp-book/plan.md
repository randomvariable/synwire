# Implementation Plan: GoReleaser MCP Server Binaries & Book Publishing Fix

**Branch**: `001-goreleaser-mcp-book` | **Date**: 2026-03-17 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `specs/001-goreleaser-mcp-book/spec.md`

## Summary

Add cross-compiled pre-built binaries for `synwire-mcp-server` to GitHub Releases using GoReleaser v2 with a matrix build strategy (4 platform targets: Linux amd64/arm64 musl, macOS amd64/arm64). GoReleaser replaces the existing hand-rolled `github-release` job. Separately, fix the GitHub Pages deployment by correcting the upload path from `docs/book` to `docs/book/html` — mdBook writes HTML to a subdirectory, causing every URL including the homepage to 404.

## Technical Context

**Language/Version**: Rust stable 1.88, edition 2024 (Cargo workspace)
**Primary Dependencies**: GoReleaser v2 (prebuilt builder), cargo-zigbuild (Linux cross-compile), GitHub Actions
**Storage**: N/A
**Testing**: No new Rust code; workflow validation via CI run after merge
**Target Platform**: Linux (musl, amd64 + arm64), macOS (amd64 + arm64)
**Project Type**: CLI binary distribution + CI/CD configuration
**Performance Goals**: Release pipeline completes within 15 minutes of tag push
**Constraints**: No Docker-in-Docker; macOS ad-hoc signing only (no paid Developer ID); binary must be statically linked on Linux
**Scale/Scope**: 4 platform targets; 1 binary (`synwire-mcp-server`)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

This feature is purely CI/CD configuration. No Rust library code is added or modified. Constitution principles I–V (trait design, API parity, safety, async, testing) are **not applicable**.

| Principle | Status | Notes |
|---|---|---|
| I. Trait-Based Abstractions | N/A | No Rust code |
| II. API Conceptual Parity | N/A | No Rust code |
| III. Safety and Correctness | N/A | No Rust code |
| IV. Async-First | N/A | No Rust code |
| V. Comprehensive Testing | N/A | No Rust code |
| VI. Diataxis Documentation | **PASS** | Fixing Pages deployment enables documentation to be accessible; no doc content changes required |
| Commit Standards | **APPLIES** | All commits must follow Conventional Commits with scope |
| Code Quality Gates | N/A | No Rust code changes |

**Gate result: PASS** — no violations, no complexity tracking required.

## Project Structure

### Documentation (this feature)

```text
specs/001-goreleaser-mcp-book/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output (N/A — no data model)
├── contracts/
│   └── release-artifacts.md  # Archive naming, staging layout, platform table
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code (repository root)

```text
.goreleaser.yaml                         # NEW — GoReleaser v2 config (prebuilt builder)

.github/workflows/
├── release.yml                          # MODIFIED — replace github-release job with GoReleaser matrix
└── pages.yml                            # MODIFIED — fix upload path: docs/book → docs/book/html
```

No Rust source changes. No new crates. No Cargo.toml changes.

**Structure Decision**: Two independent changes to existing files plus one new configuration file. The GoReleaser config lives at the repo root (conventional location). The release workflow is self-contained; the pages fix is a one-line change.

## Phase 0: Research

**Status: Complete** — see [research.md](research.md)

Key decisions resolved:

| Unknown | Decision |
|---|---|
| GoReleaser approach for Rust | `builder: prebuilt` with matrix-built binaries staged in `artifacts/` |
| Linux aarch64 cross-compilation | `cargo-zigbuild` on `ubuntu-latest` (no Docker required) |
| Static linking on Linux | `-unknown-linux-musl` targets via cargo-zigbuild |
| macOS signing | `codesign --sign - --options runtime --timestamp` (Sequoia-compatible ad-hoc) |
| Quarantine UX | Document `xattr -d com.apple.quarantine`; recommend Homebrew tap |
| Changelog | GoReleaser `use: github` + conventional commit groups |
| Crate table in release body | GoReleaser `release.footer` template |
| Pages root cause | mdBook outputs to `docs/book/html/`, not `docs/book/` — wrong upload path |

## Phase 1: Design & Contracts

**Status: Complete** — see [contracts/release-artifacts.md](contracts/release-artifacts.md)

### `.goreleaser.yaml`

New file at repository root. Key sections:

```yaml
version: 2

project_name: synwire-mcp-server

builds:
  - id: synwire-mcp-server
    builder: prebuilt
    goos: [linux, darwin]
    goarch: [amd64, arm64]
    goamd64: [v1]
    prebuilt:
      path: >-
        artifacts/{{ .Os }}_{{ .Arch }}{{ with .Amd64 }}_{{ . }}{{ end }}/synwire-mcp-server{{ .Ext }}
    binary: synwire-mcp-server

archives:
  - name_template: "synwire-mcp-server_{{ .Version }}_{{ .Os }}_{{ .Arch }}"
    format: tar.gz

checksum:
  name_template: checksums.txt
  algorithm: sha256

changelog:
  use: github
  sort: asc
  abbrev: 7
  filters:
    exclude:
      - '^chore(\(deps\))?:'
      - '^ci:'
      - 'Merge pull request'
      - 'Merge branch'
  groups:
    - title: "Breaking Changes"
      regexp: '^.*?(\w+)(\([[:word:]]+\))?!:.+$'
      order: 0
    - title: Features
      regexp: '^.*?feat(\([[:word:]]+\))??!?:.+$'
      order: 1
    - title: Bug Fixes
      regexp: '^.*?fix(\([[:word:]]+\))??!?:.+$'
      order: 2
    - title: Documentation
      regexp: '^.*?docs(\([[:word:]]+\))??!?:.+$'
      order: 3
    - title: Other
      order: 999

brews:
  - name: synwire-mcp-server
    repository:
      owner: randomvariable
      name: homebrew-tap
      token: "{{ .Env.HOMEBREW_TAP_GITHUB_TOKEN }}"
    homepage: https://randomvariable.github.io/synwire/
    description: "Synwire MCP server"
    install: bin.install "synwire-mcp-server"
    test: |
      system "#{bin}/synwire-mcp-server", "--version"

release:
  footer: |
    ## Crates

    | Crate | Version |
    |-------|---------|
    | [synwire](https://crates.io/crates/synwire) | {{ .Tag }} |
    | [synwire-core](https://crates.io/crates/synwire-core) | {{ .Tag }} |
    | [synwire-orchestrator](https://crates.io/crates/synwire-orchestrator) | {{ .Tag }} |
    | [synwire-derive](https://crates.io/crates/synwire-derive) | {{ .Tag }} |
    | [synwire-checkpoint](https://crates.io/crates/synwire-checkpoint) | {{ .Tag }} |
    | [synwire-checkpoint-sqlite](https://crates.io/crates/synwire-checkpoint-sqlite) | {{ .Tag }} |
    | [synwire-llm-openai](https://crates.io/crates/synwire-llm-openai) | {{ .Tag }} |
    | [synwire-llm-ollama](https://crates.io/crates/synwire-llm-ollama) | {{ .Tag }} |
```

### `release.yml` — GoReleaser matrix strategy

Replace the `github-release` job with:
1. `build` job — matrix across 4 targets (ubuntu × 2, macos × 2), uploads `artifacts/` tree
2. `goreleaser` job — depends on `build`, downloads all artifacts, runs GoReleaser

The existing `publish` job (crate publishing to crates.io) is **unchanged**.

Matrix entries:

| target | os | goreleaser_os | goreleaser_arch | goreleaser_variant |
|---|---|---|---|---|
| `x86_64-unknown-linux-musl` | `ubuntu-latest` | `linux` | `amd64` | `_v1` |
| `aarch64-unknown-linux-musl` | `ubuntu-latest` | `linux` | `arm64` | `` |
| `x86_64-apple-darwin` | `macos-latest` | `darwin` | `amd64` | `_v1` |
| `aarch64-apple-darwin` | `macos-latest` | `darwin` | `arm64` | `` |

Build steps per matrix entry:
- Linux: `rustup target add`, install cargo-zigbuild + zig, `cargo zigbuild --target ... --release --locked`
- macOS: `rustup target add`, `cargo build --target ... --release --locked`, then `codesign --sign - --options runtime --timestamp`
- Both: stage binary into `artifacts/<os>_<arch>[_variant]/synwire-mcp-server`, upload via `actions/upload-artifact@v4`

GoReleaser job steps:
- `actions/checkout@v6` with `fetch-depth: 0`
- `actions/download-artifact@v4` with `merge-multiple: true` into `artifacts/`
- `goreleaser/goreleaser-action@v6` with `args: release --clean`

### `pages.yml` — one-line fix

```diff
-          path: docs/book
+          path: docs/book/html
```

## Implementation Notes

### Homebrew Tap

GoReleaser will push an updated formula to `randomvariable/homebrew-tap` on each release. Add a `brews:` block to `.goreleaser.yaml`:

```yaml
brews:
  - name: synwire-mcp-server
    repository:
      owner: randomvariable
      name: homebrew-tap
      token: "{{ .Env.HOMEBREW_TAP_GITHUB_TOKEN }}"
    homepage: https://randomvariable.github.io/synwire/
    description: "Synwire MCP server"
    install: bin.install "synwire-mcp-server"
    test: |
      system "#{bin}/synwire-mcp-server", "--version"
```

A `HOMEBREW_TAP_GITHUB_TOKEN` secret (PAT with `contents: write` on `randomvariable/homebrew-tap`) must be added to the repo secrets. The default `GITHUB_TOKEN` cannot push to other repos.

Users install with: `brew install randomvariable/tap/synwire-mcp-server`

### macOS Quarantine Caveat

Ad-hoc signing removes the signature validation block but does not pre-remove the quarantine attribute that browsers set on downloaded files. macOS Sequoia 15.1+ users downloading directly from GitHub Releases may need to run `xattr -d com.apple.quarantine ./synwire-mcp-server`. This must be noted in release notes. Users installing via Homebrew tap are unaffected — Homebrew re-signs on install.

### `needs: publish` dependency

The current `github-release` job has `needs: publish` to ensure crates are published before the release is created. The new GoReleaser job should preserve this dependency: `needs: [publish, build]`.

### GoReleaser `--clean` flag

`goreleaser release --clean` deletes and recreates the `dist/` directory but does **not** touch `artifacts/`. This is the correct flag when using prebuilt binaries staged outside `dist/`.
