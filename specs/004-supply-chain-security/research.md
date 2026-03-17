# Research: Supply Chain Security Tooling

**Feature**: 004-supply-chain-security | **Date**: 2026-03-17

---

## 1. CodeQL — Rust Support

### Decision
Use CodeQL with `build-mode: none` and the `security-extended` query suite. Pin to `github/codeql-action@v4`.

### Findings

**Status**: GA since CodeQL CLI 2.23.3 (October 2025). Not experimental.

**Supported editions**: Rust 2021 and 2024. Nightly features are explicitly not supported (not relevant for Synwire, which targets stable MSRV 1.88 edition 2024).

**Build modes**: Only `build-mode: none` is available for Rust. `autobuild` and manual build are not supported. Under `none` mode, CodeQL uses `rust-analyzer` to extract the database — `build.rs` scripts and macro code are compiled by rust-analyzer, not via a full `cargo build`. A `Cargo.toml` workspace root is sufficient for CodeQL to discover all member crates.

**Prerequisites on runner**: `rustup` and `cargo` must be on PATH. `ubuntu-latest` GitHub-hosted runners satisfy this. `cargo-make` is not invoked by CodeQL and requires no special setup.

**Query suites available**:
- `security-extended` — recommended; OWASP A01–A10 coverage (except A06, handled by OSV-Scanner)
- `security-and-quality` — adds maintainability queries
- Default suite — smaller subset

**Security queries available (as of CLI 2.23.7/2.23.8, December 2025)**:
- `rust/sql-injection`, `rust/path-injection`, `rust/regex-injection`
- `rust/xss`, `rust/insecure-cookie`, `rust/non-https-url`
- `rust/request-forgery`, `rust/cleartext-storage-database`
- `rust/disabled-certificate-check`
- Cryptographic misuse queries

**Supported crates (built-in taint model)**: actix-web, poem, rocket, warp, tokio, async-std, futures, reqwest, hyper, rustls, rusqlite, sqlx, mysql, postgres, serde. Synwire's key dependencies (tokio, reqwest, rusqlite, serde) are covered.

**Known limitations**:
- Proc macro findings report at the macro invocation site only (upstream issue #20659 — no planned fix). Synwire uses `thiserror` and `serde` derive macros; any taint flows originating in generated code may have coarse location reporting.
- A low percentage of macro calls may have unresolved targets, potentially triggering "low quality scan" warnings (upstream issue #20643).
- Query depth is maturing: approximately a dozen production security queries vs hundreds for Java/C#. New queries added monthly.

**Action version**: `github/codeql-action@v4` (latest point release: v4.33.0 as of 2026-03-16). v3 deprecated October 2025; v2 retired January 2025. `init` and `analyze` steps must use the same version within a job.

**Minimum permissions**:
- `security-events: write` — required (upload SARIF)
- `contents: read` — required
- `actions: read` — required for private repos; implicit for public repos (include explicitly for forward-compatibility)

### Alternatives Considered
- **cargo-geiger**: Already in CI (Tier 2). Detects unsafe usage, not security vulnerabilities. Complementary, not a replacement.
- **`cargo audit`**: Already in CI (nightly). Covers RustSec database. OSV-Scanner is additive with broader database coverage.

---

## 2. OSV-Scanner

### Decision
Use `google/osv-scanner-action@v2.3.3` reusable workflows with two jobs: PR differential scan and scheduled full scan. Suppress via `.osv-scanner.toml`.

### Findings

**Action**: OSV-Scanner uses **reusable workflows**, not a direct `uses:` step. The version is `v2.3.3`.

**Two distinct reusable workflows**:
- `google/osv-scanner-action/.github/workflows/osv-scanner-reusable-pr.yml@v2.3.3` — PR differential scan; reports only vulnerabilities newly introduced by the PR diff vs the base branch
- `google/osv-scanner-action/.github/workflows/osv-scanner-reusable.yml@v2.3.3` — full lockfile scan; used for push-to-main and scheduled runs

**Cargo.lock scanning**: Supported natively with `--lockfile=Cargo.lock` or `--recursive ./`.

**GitHub Actions workflow file scanning**: **Not supported** by OSV-Scanner. The tool scans lockfiles (Cargo.lock, package-lock.json, etc.) against the OSV vulnerability database. It does not inspect `uses:` references in workflow YAML files. Scorecard's `Pinned-Dependencies` check covers action reference pinning; Dependabot with `github-actions` ecosystem covers action version vulnerability alerting.

**Impact on FR-008**: The spec requires scanning GitHub Actions dependencies. OSV-Scanner satisfies Cargo coverage (FR-007) but not GitHub Actions coverage (FR-008). A minimal `.github/dependabot.yml` with `package-ecosystem: github-actions` satisfies both FR-008 and Scorecard's `Dependency-Update-Tool` check. See also the Scorecard section below.

**Suppression format** (`.osv-scanner.toml` at repo root, auto-detected with `--recursive ./`):
```toml
[[IgnoredVulns]]
id = "GHSA-xxxx-xxxx-xxxx"
reason = "No fix available upstream; risk accepted 2026-03-17"
ignoreUntil = 2026-09-17   # ISO date; forces re-evaluation

[[IgnoredVulns]]
id = "RUSTSEC-2023-0071"
reason = "Only affects Windows targets"
```
The `id` field accepts GHSA, CVE, RUSTSEC, or OSV IDs. Aliases of a suppressed ID are automatically suppressed.

**Permissions**: `security-events: write`, `contents: read`, `actions: read`

### Alternatives Considered
- **`cargo audit`** (already in nightly): Uses RustSec advisory database only. OSV covers a superset.
- **Dependabot security alerts**: GitHub-native but less configurable for CI blocking.

---

## 3. OSSF Scorecard

### Decision
Use `ossf/scorecard-action@v2.4.3` (SHA: `4eaacf0543bb3f2c246792bd56e8cdeffafb205a`) on a weekly schedule and on push to `main`. Set `permissions: read-all` at workflow level. Add `.github/dependabot.yml` to satisfy `Dependency-Update-Tool` check.

### Findings

**Action**: `ossf/scorecard-action@v2.4.3` (backed by Scorecard v5.3.0).

**Permissions required** (more expansive than initially expected):
```yaml
# Top-level workflow permissions MUST be read-all for Scorecard's own
# Token-Permissions check to pass — it flags anything more permissive.
permissions: read-all

# Job-level permissions (override top-level to grant write where needed):
permissions:
  security-events: write  # SARIF upload
  id-token: write         # Sigstore keyless signing (mandatory for publish_results: true)
  contents: read
  actions: read
  checks: read            # required for GraphQL queries
  issues: read            # required for GraphQL queries
  pull-requests: read     # required for GraphQL queries
```

`id-token: write` is mandatory when `publish_results: true`. Without it the action errors on result publishing.

**`Dependency-Update-Tool` check — updatecli compatibility issue**: Scorecard's `Dependency-Update-Tool` check looks for `.github/dependabot.yml` or a Renovate config at well-known paths. **updatecli is not recognised**. Without one of these files, this check will score 0/10.

**Resolution**: Add a minimal `.github/dependabot.yml` covering the `github-actions` ecosystem only (Dependabot will handle GitHub Actions vulnerability alerting; updatecli handles Cargo updates). This satisfies FR-008, the Scorecard check, and does not conflict with updatecli's Cargo policies.

```yaml
# .github/dependabot.yml
version: 2
updates:
  - package-ecosystem: github-actions
    directory: /
    schedule:
      interval: weekly
```

**SARIF upload**: Via `codeql-action/upload-sarif`. Results appear in Security → Code scanning. Each failing check is an individual alert.

**Schedule**: Must run on push to `main` for results to be attributed to the default branch. Weekly schedule (Saturday 01:30 UTC) for ongoing posture tracking.

**Fork PR restriction**: Cannot run on fork PRs (no `id-token` permission). Do not trigger on `pull_request`.

### Alternatives Considered
- **StepSecurity Harden-Runner**: Complementary (runtime egress control), not a replacement.

---

## 4. Anchore Syft SBOM

### Decision
Use `anchore/sbom-action@v0.23.1` triggered on release published. Output SPDX 2.3 JSON, attached as a release asset.

### Findings

**Action**: `anchore/sbom-action@v0.23.1` (bundles Syft v1.42.2).

**Rust toolchain required**: Syft calls `cargo metadata` internally to resolve the full dependency graph including dev-dependencies. Without a Rust toolchain on PATH, Syft falls back to parsing `Cargo.lock` only — dev-dependency resolution is incomplete. The `dtolnay/rust-toolchain@stable` step must precede `sbom-action` in the workflow.

**Cargo workspace support**: Syft natively discovers all member crates in a Cargo workspace from the root `Cargo.toml`. No explicit crate listing required.

**Output formats**: SPDX 2.3 (JSON or tag-value), CycloneDX 1.4 (JSON or XML). SPDX-JSON is the widest-compatibility choice.

**Release asset attachment**: When triggered on `on: release`, the action automatically attaches the SBOM file to the GitHub release as a downloadable artifact.

**Permissions**: `contents: write` (to attach to release). No `id-token` required unless signing the SBOM (optional future enhancement).

**Trigger**: `on: release` with `types: [published]` ensures the SBOM is generated only for final releases, not drafts.

**Artifact naming**: configurable via `artifact-name` input. Convention: `synwire-${{ github.ref_name }}-sbom.spdx.json`.

**SBOM contents**: Each entry includes package name, version, PURL (Package URL), declared licence (from `Cargo.toml`), and file paths. Transitive dependencies are included.

### Alternatives Considered
- **`cargo-cyclonedx`**: Rust-specific but produces CycloneDX only. Syft produces both formats and is more actively maintained.
- **`cargo-sbom`**: Smaller project, less ecosystem adoption.

---

## 5. updatecli

### Decision
Use `updatecli/updatecli-action` as a scheduled GitHub Actions workflow with YAML policy files in `updatecli/updatecli.d/`. No GitHub App installation required.

### Findings

**What updatecli is**: A Go CLI tool (updatecli.io) that reads declarative YAML policy files describing what to update, checks sources for new versions, applies changes to targets (files in the repo), and optionally raises a GitHub pull request. It runs entirely within a GitHub Actions workflow using the repository's `GITHUB_TOKEN`.

**Key difference from Renovate**: Renovate uses convention-based auto-detection; updatecli uses explicit YAML policies. More verbose to configure but no external app installation, no webhook registration, and no third-party service dependency.

**Action**: `updatecli/updatecli-action@v2` — runs `updatecli apply` against a policy directory. The action uses `GITHUB_TOKEN` to create PRs.

**Permissions**: `contents: write`, `pull-requests: write` (to create branches and open PRs).

**Policy structure** (`updatecli/updatecli.d/`):

`cargo.yaml` — updates Cargo dependencies:
```yaml
sources:
  crate-name:
    kind: cratesio
    spec:
      crate: crate-name
conditions:
  check-cargo-toml:
    kind: toml
    spec:
      file: Cargo.toml
      key: dependencies.crate-name
      versionfilter:
        kind: semver
        pattern: ">=0"
targets:
  update-cargo-toml:
    kind: toml
    spec:
      file: Cargo.toml
      key: dependencies.crate-name
    scmid: default
scms:
  default:
    kind: github
    spec:
      user: "{{ requiredEnv "GITHUB_ACTOR" }}"
      email: "updatecli@users.noreply.github.com"
      owner: randomvariable
      repository: synwire
      token: "{{ requiredEnv "GITHUB_TOKEN" }}"
      branch: main
      commitmessage:
        type: chore
        scope: deps
```

`github-actions.yaml` — updates GitHub Actions SHA pins:
```yaml
sources:
  action-version:
    kind: githubrelease
    spec:
      owner: <action-owner>
      repository: <action-repo>
      token: "{{ requiredEnv "GITHUB_TOKEN" }}"
targets:
  pin-sha:
    kind: yaml
    spec:
      file: .github/workflows/<workflow>.yml
      key: jobs.<job>.steps[<n>].uses
    scmid: default
```

**Scheduling**: Weekly (Monday 05:00 UTC) via `on: schedule`. `workflow_dispatch` allows manual trigger.

**PR behaviour**: updatecli raises one PR per policy source that has an update available. If a PR already exists for that update, it updates it rather than opening a duplicate.

### Alternatives Considered
- **Renovate**: GitHub App installation required; convention-based magic is convenient but less auditable. Dropped in favour of updatecli per project requirement.
- **Dependabot** (`dependabot.yml`): GitHub-native, zero setup. Would cover Cargo and GitHub Actions automatically. Considered but not selected — updatecli gives explicit, version-controlled policy files aligned with the project's preference for auditability.
