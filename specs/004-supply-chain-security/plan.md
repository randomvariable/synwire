# Implementation Plan: Supply Chain Security Tooling

**Branch**: `004-supply-chain-security` | **Date**: 2026-03-17 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `specs/004-supply-chain-security/spec.md`

## Summary

Add five complementary, free, open-source security tooling workflows to the Synwire repository: CodeQL (SAST), OSV-Scanner (SCA), OSSF Scorecard (supply-chain posture), Anchore Syft (SBOM generation), and updatecli (automated dependency updates). All new workflows are pinned to specific action SHAs and carry minimal permissions. CodeQL, OSV-Scanner, and Scorecard upload SARIF results to the GitHub Security tab. updatecli runs as a native GitHub Actions workflow with no external app installation required.

## Technical Context

**Language/Version**: Rust stable, edition 2024, MSRV 1.85 (CodeQL supports editions 2021 and 2024)
**Primary Dependencies**: GitHub Actions runners (`ubuntu-latest`), GitHub Advanced Security (free for public repos)
**Storage**: N/A — this feature produces workflow YAML, TOML/YAML config files, and release SBOM artifacts
**Testing**: `actionlint` for static lint of workflow YAML; live workflow runs are the acceptance tests
**Target Platform**: GitHub Actions (linux/ubuntu-latest runners)
**Project Type**: CI/CD configuration (YAML + TOML files)
**Performance Goals**: CodeQL under 15 min; OSV-Scanner and Scorecard under 5 min each; updatecli under 5 min
**Constraints**: All third-party action references MUST be pinned to full commit SHA; all jobs MUST declare minimal explicit permissions; no paid secrets or tokens required for core functionality
**Scale/Scope**: 5 new workflow files, 1 updatecli policy directory (`updatecli/updatecli.d/`), 1 OSV-Scanner ignore config, updates to 5 existing workflow files

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

This feature adds CI/CD configuration only. No Rust source code is modified.

| Principle | Applies? | Status |
|---|---|---|
| I. Trait-Based Abstractions | No — no new Rust traits | PASS (N/A) |
| II. API Conceptual Parity | No — no Rust API surface | PASS (N/A) |
| III. Safety and Correctness | Indirectly — CodeQL enforces this | PASS — scanners strengthen compliance |
| IV. Async-First | No — no Rust I/O code | PASS (N/A) |
| V. Comprehensive Testing | Partial — workflow files are not unit-testable | PASS — `actionlint` provides static validation; live run is the acceptance test |
| VI. Diataxis Documentation | Yes — quickstart.md required | PASS — quickstart.md covers how-to quadrant |

**Code Quality Gates**: Unchanged. New workflows run alongside existing gates without modifying them.

**No violations. Proceed.**

## Project Structure

### Documentation (this feature)

```text
specs/004-supply-chain-security/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── quickstart.md        # Phase 1 output — maintainer how-to guide
├── checklists/
│   └── requirements.md  # Spec quality checklist
└── tasks.md             # Phase 2 output (/speckit.tasks)
```

### Source Code (repository root)

```text
.github/
└── workflows/
    ├── ci.yml                # EXISTING — branch filter + SHA pin updates
    ├── nightly.yml           # EXISTING — SHA pin updates only
    ├── coverage.yml          # EXISTING — SHA pin updates only
    ├── release.yml           # EXISTING — SHA pin updates only
    ├── pages.yml             # EXISTING — SHA pin updates only
    ├── codeql.yml            # NEW — CodeQL SAST (Rust, build-mode: none)
    ├── osv-scanner-pr.yml    # NEW — OSV-Scanner SCA differential PR scan (reusable workflow)
    ├── osv-scanner-scheduled.yml # NEW — OSV-Scanner SCA full scan (reusable workflow, scheduled + push)
    ├── scorecard.yml         # NEW — OSSF Scorecard (scheduled + push to main)
    ├── updatecli.yml         # NEW — updatecli dependency updates (scheduled)
    └── sbom.yml              # NEW — Anchore Syft SBOM (on release published)

updatecli/
└── updatecli.d/
    ├── cargo.yaml            # NEW — updatecli policy: Cargo dependency updates
    └── github-actions.yaml   # NEW — updatecli policy: GitHub Actions SHA updates

.osv-scanner.toml             # NEW — OSV-Scanner suppression config

.github/
└── dependabot.yml            # NEW — github-actions ecosystem only (satisfies Scorecard Dependency-Update-Tool check + FR-008)
```

**Structure Decision**: All workflow files in `.github/workflows/`. updatecli policies follow the updatecli convention of `updatecli/updatecli.d/*.yaml` at the repo root. `.osv-scanner.toml` at repo root per OSV-Scanner convention.

---

## Phase 0: Research

*See [research.md](./research.md) for full findings. Key decisions:*

| Topic | Decision | Rationale |
|---|---|---|
| CodeQL Rust support | GA since CodeQL CLI 2.23.3 (Oct 2025); `build-mode: none` only | Confirmed GA; no autobuild available for Rust; `rust-analyzer`-based extraction |
| CodeQL query suite | `security-extended` | Covers OWASP A01–A10 (except A06 which OSV-Scanner handles) |
| CodeQL action version | `github/codeql-action@v4` (v4.33.0) | v3 deprecated Oct 2025; v2 retired Jan 2025 |
| OSV-Scanner action | `google/osv-scanner-action@v2.3.3` reusable workflows | Official action; Cargo.lock scanning only — does not scan GitHub Actions workflow files |
| OSV-Scanner suppress format | `[[IgnoredVulns]]` table in `.osv-scanner.toml` | TOML at repo root; version-controlled and auditable |
| GitHub Actions vuln coverage (FR-008) | `.github/dependabot.yml` with `github-actions` ecosystem | Dependabot handles GHA alerts; satisfies Scorecard `Dependency-Update-Tool` check |
| OSSF Scorecard action | `ossf/scorecard-action@v2.4.3` (SHA: `4eaacf05...`) | Current stable; requires `id-token: write`; needs `permissions: read-all` at workflow level |
| Scorecard `Dependency-Update-Tool` | Satisfied by `dependabot.yml` (not updatecli) | updatecli is not recognised by Scorecard; dependabot.yml for GHA + updatecli for Cargo is the split |
| Anchore Syft SBOM action | `anchore/sbom-action@v0.23.1` | Current stable; SPDX-JSON output; attaches artifact to GitHub release |
| SBOM format | SPDX 2.3 (JSON) | Widest toolchain compatibility |
| updatecli approach | Native GitHub Actions workflow + YAML policy files in `updatecli/updatecli.d/` | No GitHub App installation required; policies are explicit and version-controlled |
| updatecli scheduling | Weekly on Mondays | Balances update frequency with PR noise |
| Action SHA pinning | All new and existing actions pinned to full commit SHA | Scorecard "Pinned Dependencies" check; updatecli keeps pins current via its own PRs |

---

## Phase 1: Design

### Workflow Designs

#### codeql.yml

- **Triggers**: `push` to `main`, `pull_request` targeting `main`, weekly schedule (Monday 03:00 UTC)
- **Permissions**: `security-events: write`, `contents: read`, `actions: read`
- **Steps**: checkout → install Rust stable (`dtolnay/rust-toolchain@stable`) → `codeql-action/init` (language: rust, build-mode: none, queries: security-extended) → `codeql-action/analyze`
- **No autobuild step** — not supported for Rust; `build-mode: none` uses rust-analyzer
- **Known limitation**: proc macro findings are reported at the macro invocation site (upstream issue, no planned fix)

#### osv-scanner.yml

Two separate workflow files (OSV-Scanner uses reusable workflows, not direct action calls):
1. **PR workflow** — calls `google/osv-scanner-action/.github/workflows/osv-scanner-reusable-pr.yml@v2.3.3`; differential scan reports only newly introduced vulnerabilities
2. **Scheduled workflow** — calls `google/osv-scanner-action/.github/workflows/osv-scanner-reusable.yml@v2.3.3`; full `Cargo.lock` scan on push to `main` and daily at 02:00 UTC

- **Permissions**: `security-events: write`, `contents: read`, `actions: read`
- **Suppression**: `.osv-scanner.toml` with `[[IgnoredVulns]]` entries (id, reason, ignoreUntil fields)
- **GitHub Actions coverage**: handled separately by `.github/dependabot.yml` (not OSV-Scanner)

#### scorecard.yml

- **Triggers**: `push` to `main`, weekly schedule (Saturday 01:30 UTC)
- **Top-level permissions**: `read-all` (required for Scorecard's own `Token-Permissions` check to pass)
- **Job-level permissions**: `security-events: write`, `id-token: write`, `contents: read`, `actions: read`, `checks: read`, `issues: read`, `pull-requests: read`
- **Steps**: checkout (with `persist-credentials: false`) → `ossf/scorecard-action` (SHA-pinned) → `codeql-action/upload-sarif`
- **Token**: `SCORECARD_TOKEN` secret (PAT with `public_repo` scope only) for higher GitHub API rate limits; falls back to `GITHUB_TOKEN`

#### updatecli.yml

- **Triggers**: weekly schedule (Monday 05:00 UTC), `workflow_dispatch`
- **Permissions**: `contents: write`, `pull-requests: write`
- **Steps**: checkout → `updatecli/updatecli-action` with `apply` command pointing at `updatecli/updatecli.d/`
- **Policies**:
  - `cargo.yaml`: source = crates.io latest version; condition = version differs from `Cargo.toml`; target = update `Cargo.toml` and raise PR
  - `github-actions.yaml`: source = GitHub releases for each action; target = update SHA pins in workflow files; raise PR per action

#### sbom.yml

- **Trigger**: `on: release` (types: `[published]`)
- **Permissions**: `contents: write`
- **Steps**: checkout → install Rust stable → `anchore/sbom-action` (format: spdx-json, artifact-name: `synwire-${{ github.ref_name }}-sbom.spdx.json`)
- **Scope**: Cargo workspace root; Syft discovers all member crates automatically

### No Data Model

No application data entities. `data-model.md` is not produced.

### No External Contracts

No public API, CLI interface, or service endpoint is changed. `contracts/` is not produced.

---

## Phase 1 Implementation Phases

### Phase A — Pin all existing action references (prerequisite for Scorecard)

Update existing workflows to use full SHA pins. updatecli will keep these current going forward.

- `.github/workflows/ci.yml` — pin `actions/checkout`, `dtolnay/rust-toolchain`, `Swatinem/rust-cache`, `taiki-e/install-action`, `davidB/rust-cargo-make`, `actions/upload-artifact`; add `004-*` to push branch filter
- `.github/workflows/nightly.yml` — same action set
- `.github/workflows/coverage.yml` — audit and pin all actions
- `.github/workflows/release.yml` — audit and pin all actions
- `.github/workflows/pages.yml` — audit and pin all actions

### Phase B — CodeQL workflow

Create `.github/workflows/codeql.yml`.

### Phase C — OSV-Scanner workflows + suppress config + dependabot.yml

Create two separate workflow files (OSV-Scanner uses reusable workflows, not a single action call):
- `.github/workflows/osv-scanner-pr.yml` — differential PR scan
- `.github/workflows/osv-scanner-scheduled.yml` — full scheduled + push-to-main scan
- `.osv-scanner.toml` — empty suppress list, ready for entries
- `.github/dependabot.yml` — `github-actions` ecosystem only; satisfies FR-008 (GitHub Actions dependency scanning) and Scorecard's `Dependency-Update-Tool` check

### Phase D — OSSF Scorecard workflow

Create `.github/workflows/scorecard.yml`. Document `SCORECARD_TOKEN` secret setup in quickstart.md.

### Phase E — updatecli workflow + policies

Create `.github/workflows/updatecli.yml`, `updatecli/updatecli.d/cargo.yaml`, and `updatecli/updatecli.d/github-actions.yaml`.

### Phase F — SBOM workflow

Create `.github/workflows/sbom.yml`.
