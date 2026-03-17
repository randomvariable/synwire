# Tasks: Supply Chain Security Tooling

**Input**: Design documents from `specs/004-supply-chain-security/`
**Branch**: `004-supply-chain-security`
**Spec**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md) | **Research**: [research.md](./research.md)

**Organization**: Tasks are grouped by user story. No tests are generated (not requested in spec).

## Format: `[ID] [P?] [Story?] Description with file path`

- **[P]**: Can run in parallel (independent files, no incomplete dependencies)
- **[Story]**: Maps to user story from spec.md (US1–US5)
- Exact file paths are included in every task description

---

## Phase 1: Setup

**Purpose**: Confirm prerequisites and update branch filters before any workflow work begins.

- [x] T001 Verify GitHub Advanced Security (code scanning) is enabled on the repository: Settings → Security → Code scanning → ensure "Set up code scanning" is available (required for SARIF uploads from CodeQL, OSV-Scanner, and Scorecard to appear in the Security tab)
- [x] T002 Add `"004-*"` to the `push.branches` filter in `.github/workflows/ci.yml` so CI runs on this branch during development (current filter is `[main, "001-*"]`)

---

## Phase 2: Foundational — Pin Existing Action References

**Purpose**: Convert all floating action tags in existing workflows to full commit SHA pins. Required before Scorecard (US4) is enabled — without pinned actions, Scorecard's `Pinned-Dependencies` check scores 0/10 on day one.

**⚠️ CRITICAL**: Scorecard (US4) must not be enabled until this phase is complete.

- [x] T003 [P] Resolve and replace all floating action references with SHA pins in `.github/workflows/ci.yml`: `actions/checkout@v6`, `dtolnay/rust-toolchain@stable`, `Swatinem/rust-cache@v2`, `taiki-e/install-action@v2`, `davidB/rust-cargo-make@v1`, `actions/upload-artifact@v6`. Use `gh api repos/<owner>/<repo>/git/ref/tags/<tag> --jq '.object.sha'` to resolve each SHA; add the human-readable tag as an inline comment (e.g., `# v4.2.2`)
- [x] T004 [P] Resolve and replace all floating action references with SHA pins in `.github/workflows/nightly.yml` following the same pattern as T003
- [x] T005 [P] Audit `.github/workflows/coverage.yml` for all action references and pin each to its full commit SHA
- [x] T006 [P] Audit `.github/workflows/release.yml` for all action references and pin each to its full commit SHA
- [x] T007 [P] Audit `.github/workflows/pages.yml` for all action references and pin each to its full commit SHA
- [x] T008 Run `actionlint` on all five updated workflow files (ci.yml, nightly.yml, coverage.yml, release.yml, pages.yml) and resolve any reported errors before proceeding

**Checkpoint**: All existing workflows use full SHA pins — Scorecard's `Pinned-Dependencies` check will now pass for these files.

---

## Phase 3: User Story 1 — Automated Code Security Scanning on Every PR (Priority: P1) 🎯 MVP

**Goal**: Every PR against `main` receives a CodeQL security scan result. Findings are visible as PR annotations and in the GitHub Security tab. No manual trigger required.

**Independent Test**: Open a PR against `main` and confirm the "CodeQL / Analyze (Rust)" check appears and completes. Navigate to Security → Code scanning alerts and confirm results are uploaded.

- [x] T009 [US1] Create `.github/workflows/codeql.yml`: triggers on `push` to `main` and `pull_request` targeting `main` and weekly schedule (`cron: "0 3 * * 1"`); single job `analyze` with `runs-on: ubuntu-latest`; job permissions `security-events: write`, `contents: read`, `actions: read`; steps: `actions/checkout` (SHA-pinned, `persist-credentials: false`) → `dtolnay/rust-toolchain@stable` (SHA-pinned) → `github/codeql-action/init@v4` (SHA-pinned, `languages: rust`, `build-mode: none`, `queries: security-extended`) → `github/codeql-action/analyze@v4` (SHA-pinned, `category: "/language:rust"`); no autobuild step; workflow-level `permissions: {}` with job-level overrides

**Checkpoint**: US1 is complete and independently testable. PRs now receive automated CodeQL results.

---

## Phase 4: User Story 2 — Continuous Dependency Vulnerability Detection (Priority: P2)

**Goal**: PRs that introduce a vulnerable dependency are flagged before merge. Daily scheduled scans detect newly disclosed vulnerabilities in existing dependencies. Known-accepted vulnerabilities can be suppressed with an auditable justification.

**Independent Test**: Temporarily add `time = "0.1.44"` (known RUSTSEC advisory) to any crate's `Cargo.toml`, open a PR, and confirm the OSV-Scanner PR check reports the finding. Revert and confirm the check clears.

- [x] T010 [P] [US2] Create `.github/workflows/osv-scanner-pr.yml`: trigger `on: pull_request` targeting `main` and `merge_group`; workflow-level `permissions: {}` with job permissions `security-events: write`, `contents: read`, `actions: read`; single job `scan-pr` using reusable workflow `google/osv-scanner-action/.github/workflows/osv-scanner-reusable-pr.yml@v2.3.3` with `scan-args: "--lockfile=Cargo.lock"` and `permissions` block nested under `uses:`
- [x] T011 [P] [US2] Create `.github/workflows/osv-scanner-scheduled.yml`: triggers on `push` to `main` and daily schedule (`cron: "0 2 * * *"`); single job `scan-scheduled` using reusable workflow `google/osv-scanner-action/.github/workflows/osv-scanner-reusable.yml@v2.3.3` with `scan-args: "--lockfile=Cargo.lock"` and `fail-on-vuln: true`
- [x] T012 [P] [US2] Create `.osv-scanner.toml` at the repository root: include a commented example `[[IgnoredVulns]]` entry showing the `id`, `reason`, and `ignoreUntil` fields with format documentation; leave the active entries section empty (no suppressions on initial commit)
- [x] T013 [US2] Create `.github/dependabot.yml`: `version: 2`; single entry with `package-ecosystem: github-actions`, `directory: /`, `schedule.interval: weekly`; this satisfies FR-008 (GitHub Actions dependency scanning coverage) and the Scorecard `Dependency-Update-Tool` check — Dependabot handles GitHub Actions vulnerability alerting while updatecli handles Cargo

**Checkpoint**: US2 is complete. OSV-Scanner runs on PRs and on schedule. `.osv-scanner.toml` is in place for future suppressions. Dependabot covers GitHub Actions advisories.

---

## Phase 5: User Story 3 — Automated Dependency Update Proposals (Priority: P3)

**Goal**: A weekly GitHub Actions workflow raises PRs proposing updates to Cargo dependencies and GitHub Actions SHA pins. No external app installation required. Each PR passes CI before review.

**Independent Test**: Navigate to Actions → updatecli → Run workflow (manual trigger). Confirm the workflow completes and, if any updates are available, a PR is opened against `main` with `[updatecli]` in the title.

- [x] T014 [US3] Create the `updatecli/updatecli.d/` directory hierarchy at the repository root; this directory holds all updatecli policy YAML files consumed by the updatecli workflow
- [x] T015 [P] [US3] Create `updatecli/updatecli.d/cargo.yaml`: for each direct dependency in the workspace root `Cargo.toml`, define an updatecli manifest with `sources` (kind: `cratesio`, spec: crate name), `conditions` (kind: `toml`, checks current version in `Cargo.toml`), `targets` (kind: `toml`, updates version field in `Cargo.toml`), and `scms` (kind: `github`, token from `GITHUB_TOKEN`, owner: `randomvariable`, repository: `synwire`, branch: `main`, commit message type: `chore`, scope: `deps`); raise PR per updated crate
- [x] T016 [P] [US3] Create `updatecli/updatecli.d/github-actions.yaml`: for each action reference in `.github/workflows/*.yml`, define an updatecli manifest with `sources` (kind: `githubrelease`, owner and repository of the action), `conditions` (kind: `yaml`, reads current SHA from the workflow file), `targets` (kind: `yaml`, updates the SHA pin in the workflow file); raise PR per updated action SHA
- [x] T017 [US3] Create `.github/workflows/updatecli.yml`: triggers on weekly schedule (`cron: "0 5 * * 1"`) and `workflow_dispatch`; permissions `contents: write`, `pull-requests: write`; single job using `updatecli/updatecli-action@v2` (SHA-pinned) with `apply` command and `working-directory: updatecli/updatecli.d`; passes `GITHUB_TOKEN` as environment variable

**Checkpoint**: US3 is complete. updatecli runs weekly and will raise PRs for any available Cargo or GitHub Actions updates.

---

## Phase 6: User Story 4 — Continuous Supply Chain Posture Assessment (Priority: P4)

**Goal**: A supply chain posture score is generated weekly and on every push to `main`. Results appear in the GitHub Security tab. Scorecard's `Pinned-Dependencies` check passes immediately (Phase 2 prerequisite).

**Dependency**: Requires Phase 2 (foundational SHA pinning) to be complete before enabling, otherwise `Pinned-Dependencies` check will score 0 on the first run.

**Independent Test**: Push any commit to `main` and confirm the "Scorecard / Scorecard analysis" check completes. Navigate to Security → Code scanning alerts and confirm Scorecard alerts appear.

- [x] T018 [US4] Create `.github/workflows/scorecard.yml`: `permissions: read-all` at the top-level workflow scope; triggers on `push` to `main` and weekly schedule (`cron: "30 1 * * 6"` — Saturday 01:30 UTC); single job `analysis` with job-level permissions `security-events: write`, `id-token: write`, `contents: read`, `actions: read`, `checks: read`, `issues: read`, `pull-requests: read`; steps: `actions/checkout` (SHA-pinned, `persist-credentials: false`) → `ossf/scorecard-action@4eaacf0543bb3f2c246792bd56e8cdeffafb205a` (`results_file: results.sarif`, `results_format: sarif`, `publish_results: true`, `repo-token: ${{ secrets.SCORECARD_TOKEN }}`) → `actions/upload-artifact` (SHA-pinned, retains `results.sarif` for 5 days) → `github/codeql-action/upload-sarif` (SHA-pinned, `sarif_file: results.sarif`)

**Checkpoint**: US4 is complete. Scorecard runs weekly and on every merge to `main`, with results in the Security tab.

---

## Phase 7: User Story 5 — Automated SBOM Generation for Releases (Priority: P5)

**Goal**: Every GitHub release has an SBOM (SPDX 2.3 JSON) attached as a downloadable artifact. No manual steps required from the release author.

**Independent Test**: Create a test release tag (`v0.0.0-test`) and publish a draft release. Confirm the "SBOM on Release" workflow triggers, completes, and attaches `synwire-v0.0.0-test-sbom.spdx.json` to the release assets. Delete the test release after confirming.

- [x] T019 [US5] Create `.github/workflows/sbom.yml`: trigger `on: release` with `types: [published]`; permissions `contents: write`, `actions: read`; single job `sbom` with steps: `actions/checkout` (SHA-pinned) → `dtolnay/rust-toolchain@stable` (SHA-pinned; required — Syft calls `cargo metadata` internally and falls back to Cargo.lock-only without a toolchain) → `anchore/sbom-action@v0.23.1` (SHA-pinned, `path: ./`, `format: spdx-json`, `artifact-name: synwire-${{ github.ref_name }}-sbom.spdx.json`, `output-file: synwire.spdx.json`)

**Checkpoint**: US5 is complete. Every published release automatically receives an SBOM attachment.

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Improve Scorecard scores, validate all new workflows, and finalise documentation.

- [x] T020 [P] Create `SECURITY.md` at the repository root: include a "Supported Versions" table (current stable release supported; prior versions best-effort), a "Reporting a Vulnerability" section with instructions to use GitHub private vulnerability reporting (Settings → Security → Advisories), and an expected response timeline; this satisfies Scorecard's `Security-Policy` check
- [x] T021 [P] Run `actionlint` on all five new workflow files (`.github/workflows/codeql.yml`, `osv-scanner-pr.yml`, `osv-scanner-scheduled.yml`, `scorecard.yml`, `updatecli.yml`, `sbom.yml`) and resolve any reported errors; confirm each file has explicit `permissions:` blocks with no unnecessary write access
- [x] T022 [P] Fix the `[tasks.geiger]` entry in `Makefile.toml`: replace the current `command = "cargo"` / `args = ["geiger", "--all", "--all-features", "--all-targets"]` with a `script` that iterates over `crates/*/` and runs `cargo geiger --all-features --all-targets` in each crate directory individually (cargo-geiger does not support workspace-level execution; running at the workspace root silently produces incomplete results). The script should print the crate name before each run and use `|| true` to continue on per-crate errors so the task is non-blocking. The CI job in `.github/workflows/ci.yml` calls `cargo make geiger` and inherits this fix automatically — no CI YAML change is required.
- [x] T023 Update `specs/004-supply-chain-security/quickstart.md`: add a one-time setup step to the Scorecard section describing how to create the `SCORECARD_TOKEN` PAT (fine-grained or classic, `public_repo` scope only, no write permissions) and add it as a repository secret at Settings → Secrets → Actions → New repository secret

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (Setup)**: No dependencies — start immediately
- **Phase 2 (Foundational — SHA pinning)**: Depends on Phase 1; **BLOCKS Phase 6 (US4/Scorecard)**
- **Phase 3 (US1 — CodeQL)**: Depends on Phase 1 only; can run in parallel with Phase 2
- **Phase 4 (US2 — OSV-Scanner)**: Depends on Phase 1 only; can run in parallel with Phases 2 and 3
- **Phase 5 (US3 — updatecli)**: Depends on Phase 1 only; can run in parallel with Phases 2, 3, and 4
- **Phase 6 (US4 — Scorecard)**: Depends on Phase 2 completion (SHA pinning required)
- **Phase 7 (US5 — SBOM)**: Depends on Phase 1 only; fully independent
- **Phase 8 (Polish)**: Depends on all user story phases being complete

### User Story Dependencies

| Story | Depends on | Can run in parallel with |
|---|---|---|
| US1 — CodeQL | Phase 1 | Phase 2, US2, US3, US5 |
| US2 — OSV-Scanner | Phase 1 | Phase 2, US1, US3, US5 |
| US3 — updatecli | Phase 1 | Phase 2, US1, US2, US5 |
| US4 — Scorecard | Phase 2 (must complete first) | US5 |
| US5 — SBOM | Phase 1 | Phase 2, US1, US2, US3 |

### Within Each User Story

- For US3: T014 → (T015 ∥ T016) → T017
- All other user stories: single task or fully parallel tasks

### Parallel Opportunities

```
Phase 1:  T001 ∥ T002
Phase 2:  T003 ∥ T004 ∥ T005 ∥ T006 ∥ T007  →  T008
Phase 3+: T009 (US1) ∥ T010+T011+T012+T013 (US2) ∥ T014→T015+T016→T017 (US3) ∥ T019 (US5)
          T018 (US4) starts after T008 (Phase 2 complete)
Polish:   T020 ∥ T021 ∥ T022  →  T023
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1 (T001–T002)
2. Complete Phase 2 (T003–T008) — run T003–T007 in parallel
3. Complete Phase 3 / US1 (T009) — one file
4. **Stop and validate**: Open a test PR and confirm CodeQL check appears and reports to Security tab
5. Merge Phase 3 changes

### Incremental Delivery

1. Phase 1 + Phase 2 → SHA-pinned, branch filter correct
2. Phase 3 (US1 — CodeQL) → PRs get SAST → MVP
3. Phase 4 (US2 — OSV-Scanner) → dependency vulnerability detection on PRs and schedule
4. Phase 5 (US3 — updatecli) → automated dependency update PRs
5. Phase 6 (US4 — Scorecard) → supply chain posture scoring (requires Phase 2 complete)
6. Phase 7 (US5 — SBOM) → SBOM on every release
7. Phase 8 (Polish) → SECURITY.md, actionlint validation, doc updates

### Single-Developer Suggested Order

`T001 → T002 → T003–T007 (parallel) → T008 → T009 → T010–T012 (parallel) → T013 → T014 → T015–T016 (parallel) → T017 → T018 → T019 → T020–T022 (parallel) → T023`

---

## Summary

| Metric | Value |
|---|---|
| Total tasks | 23 |
| Phase 1 (Setup) | 2 |
| Phase 2 (Foundational) | 6 |
| US1 — CodeQL (P1) | 1 |
| US2 — OSV-Scanner (P2) | 4 |
| US3 — updatecli (P3) | 4 |
| US4 — Scorecard (P4) | 1 |
| US5 — SBOM (P5) | 1 |
| Phase 8 (Polish) | 4 |
| Parallelisable tasks | 15 of 23 |
| New files created | 11 |
| Existing files updated | 7 |

### MVP Scope

Phase 1 + Phase 2 + Phase 3 (US1 — CodeQL): **9 tasks, 1 new workflow file**. Delivers automated Rust SAST on every PR with results in the GitHub Security tab.
