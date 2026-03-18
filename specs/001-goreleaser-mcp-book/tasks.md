# Tasks: GoReleaser MCP Server Binaries & Book Publishing Fix

**Input**: Design documents from `specs/001-goreleaser-mcp-book/`
**Prerequisites**: plan.md ✓, spec.md ✓, research.md ✓, data-model.md ✓ (N/A), contracts/release-artifacts.md ✓

**Tests**: Not requested — no test tasks generated.

**Note**: During implementation, GoReleaser 2.14 was found to have dropped the `prebuilt` builder in favour of a native `builder: rust`. This eliminates the matrix build strategy — GoReleaser calls `cargo zigbuild` directly for all 4 targets from a single `ubuntu-latest` runner. macOS ad-hoc signing uses `rcodesign` (Rust port of codesign, works on Linux).

---

## Phase 1: Setup (Shared Infrastructure)

- [ ] T001 Add `HOMEBREW_TAP_GITHUB_TOKEN` secret (PAT with `contents: write` on `randomvariable/homebrew-tap`) to repository secrets at `github.com/randomvariable/synwire/settings/secrets/actions`
- [ ] T002 Verify `randomvariable/homebrew-tap` GitHub repo exists and is accessible with the PAT from T001

**Checkpoint**: Secrets and external repo ready

---

## Phase 3: User Story 1 — Download Pre-built MCP Server Binary (Priority: P1) 🎯 MVP

**Goal**: Tagged releases produce cross-compiled `synwire-mcp-server` archives for Linux amd64/arm64 (musl) and macOS amd64/arm64, with SHA-256 checksums and a Homebrew formula.

**Independent Test**: Push a version tag; verify the `goreleaser` job succeeds and the GitHub Release contains 4 `.tar.gz` archives, `checksums.txt`, and `randomvariable/homebrew-tap` receives an updated `Formula/synwire-mcp-server.rb`.

### Implementation for User Story 1

- [x] T003 [US1] Create `.goreleaser.yaml` at repository root — `builder: rust`, `tool: cargo`, `command: zigbuild`, 4 targets, `brews` → `randomvariable/homebrew-tap`, rcodesign post-hook for macOS ad-hoc signing, `formats: [tar.gz]`, SHA-256 checksums, conventional commit changelog, crate table footer
- [x] T004 [US1] Replace `github-release` job in `.github/workflows/release.yml` with `goreleaser` job — single `ubuntu-latest` runner, `needs: publish`, installs zig + cargo-zigbuild + rcodesign + all 4 rustup targets, runs `goreleaser/goreleaser-action@v6`
- [x] T005–T009 [US1] Superseded by T003/T004 — matrix strategy not needed with native `builder: rust`

**Checkpoint**: US1 complete — tag a release and verify 4 platform archives + checksums + Homebrew formula on `randomvariable/homebrew-tap`

---

## Phase 4: User Story 2 — Documentation Site Loads Successfully (Priority: P2)

**Goal**: Fix GitHub Pages deployment so the homepage and all linked pages return HTTP 200.

**Independent Test**: Push to `main`; visit `https://randomvariable.github.io/synwire/` and confirm the homepage loads.

### Implementation for User Story 2

- [x] T010 [US2] In `.github/workflows/pages.yml`, change `path: docs/book` to `path: docs/book/html`

**Checkpoint**: US2 complete — trigger Pages workflow, confirm homepage loads

---

## Phase 5: Polish & Cross-Cutting Concerns

- [x] T011 [P] Add macOS quarantine/installation note to `README.md`
- [x] T012 [P] Validate `.goreleaser.yaml` with `goreleaser check` — passes (experimental rust builder warning + brews deprecation warning only)

---

## Notes

- `builder: prebuilt` was removed in GoReleaser 2.14; replaced by `builder: rust` which calls cargo directly
- `archives.format` deprecated; use `archives.formats` (array)
- `brews` deprecated in favour of `homebrew_casks`; still functional, noted with TODO comment in config
- IDE schema validation shows stale errors (line 7 `builder: rust`, line 16 `prebuilt`) — these are from the IDE using an older schema; `goreleaser check` confirms the config is valid
- GoReleaser requires `fetch-depth: 0` on checkout for changelog generation
- rcodesign (`apple-codesign` crate) provides `codesign --ad-hoc` equivalent on Linux
