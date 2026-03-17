# Feature Specification: Supply Chain Security Tooling

**Feature Branch**: `004-supply-chain-security`
**Created**: 2026-03-17
**Status**: Draft
**Input**: User description: "formalise codeql in github actions, add osv-scanner and updatecli"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Automated Code Security Scanning on Every PR (Priority: P1)

A contributor opens a pull request. Without any manual action, GitHub runs CodeQL static analysis and surfaces any security findings directly on the PR. Reviewers can see security issues before approving the change. The repository security dashboard shows a continuous record of findings over time.

**Why this priority**: Every PR is a potential introduction of security vulnerabilities. Catching issues at review time costs far less than fixing them post-merge. This is the highest-impact change in the feature set.

**Independent Test**: Can be tested by opening a PR with a known-bad code pattern and confirming GitHub surfaces a security alert on the PR without any manual trigger.

**Acceptance Scenarios**:

1. **Given** a pull request is opened against `main`, **When** the CI pipeline runs, **Then** CodeQL results appear in the GitHub Security tab and any findings are annotated on the PR diff within the CI cycle.
2. **Given** a PR contains no security issues, **When** CodeQL completes, **Then** the scan passes with no findings and the PR is unblocked by this check.
3. **Given** a PR introduces a high-severity security vulnerability, **When** CodeQL completes, **Then** the finding is visible to reviewers as a PR annotation or check failure before merge.
4. **Given** a push is made directly to `main`, **When** CI runs, **Then** CodeQL still executes and uploads results to the repository security dashboard.

---

### User Story 2 - Continuous Dependency Vulnerability Detection (Priority: P2)

A maintainer is alerted when any Synwire dependency has a known security vulnerability disclosed in the OSV database. The alert arrives quickly after disclosure and includes enough information (affected package, advisory identifier, severity) to triage and act. Known-accepted vulnerabilities awaiting upstream fixes can be suppressed with a tracked justification so that routine CI is not permanently blocked.

**Why this priority**: Dependencies represent the largest attack surface for most projects. The project already runs `cargo audit` nightly against the RustSec database, but OSV-Scanner broadens coverage to multiple advisory databases and ecosystems including GitHub Actions workflows. Formalising this as a CI check ensures continuous coverage without manual effort.

**Independent Test**: Can be tested by temporarily adding a dependency with a known OSV advisory to `Cargo.toml`, running the scan, and confirming detection and reporting.

**Acceptance Scenarios**:

1. **Given** a dependency has a known vulnerability in the OSV database, **When** the scheduled scan runs, **Then** the vulnerability is reported with its advisory identifier, the affected package name, and the installed version.
2. **Given** a PR introduces a dependency with a known vulnerability, **When** CI runs, **Then** the vulnerability is flagged before merge.
3. **Given** no known vulnerabilities exist in current dependencies, **When** the scan runs, **Then** it completes cleanly with no findings.
4. **Given** a vulnerability is detected, **When** the finding is surfaced, **Then** it includes the advisory URL so maintainers can assess severity and remediation options.
5. **Given** a vulnerability has been explicitly suppressed with a tracked justification, **When** the scan runs, **Then** the suppressed finding does not block CI but remains recorded.

---

### User Story 3 - Automated Dependency Update Proposals (Priority: P3)

A maintainer sees pull requests automatically raised by updatecli proposing updates to Synwire's dependencies across all ecosystems (Cargo, GitHub Actions). updatecli runs on a schedule as a GitHub Actions workflow, requiring no external app installation. Each update PR passes CI before a maintainer reviews it, and includes context about what changed.

**Why this priority**: Keeping dependencies current is both a security and reliability concern. Manual dependency management is error-prone and easily neglected. updatecli automates this as a native GitHub Actions workflow, leaving merge decisions to maintainers without requiring any third-party app installation.

**Independent Test**: Can be tested by triggering the updatecli workflow manually and confirming it raises a PR for any available dependency update with appropriate context.

**Acceptance Scenarios**:

1. **Given** a new version of any Cargo or GitHub Actions dependency is published, **When** the updatecli workflow runs, **Then** a PR is raised proposing the update.
2. **Given** an updatecli update PR is raised, **When** a maintainer reviews it, **Then** the PR description identifies the updated package, the old version, and the new version.
3. **Given** an updatecli update PR is raised, **When** CI runs on it, **Then** the full CI pipeline executes so maintainers can merge with confidence.
4. **Given** the updatecli workflow is triggered, **When** no updates are available, **Then** no PRs are opened and the workflow completes cleanly.

---

### User Story 4 - Continuous Supply Chain Posture Assessment (Priority: P4)

A maintainer can see an objective, scored assessment of the repository's supply chain security posture — covering areas such as pinned dependencies, branch protection, code review enforcement, and CI integrity. The score updates automatically when the repository configuration changes, and findings are surfaced in the GitHub Security tab alongside other scan results.

**Why this priority**: OSSF Scorecard provides a holistic view of supply chain hygiene that no single scanner covers. It catches configuration-level weaknesses (e.g., unpinned actions, missing branch protection) that are invisible to code and dependency scanners. It's free, language-agnostic, and backed by the Linux Foundation's OpenSSF project.

**Independent Test**: Can be tested by enabling the Scorecard workflow and confirming a score appears in the GitHub Security tab without any manual trigger.

**Acceptance Scenarios**:

1. **Given** the Scorecard workflow is enabled, **When** a scheduled run completes, **Then** a supply chain posture score and per-check results are uploaded to the GitHub Security dashboard.
2. **Given** a CI workflow file is changed to use an unpinned action reference, **When** Scorecard next runs, **Then** the "Pinned Dependencies" check score decreases and the finding is surfaced.
3. **Given** the repository meets all assessed supply chain best practices, **When** Scorecard runs, **Then** all checks pass with no findings in the security dashboard.

---

### User Story 5 - Automated SBOM Generation for Releases (Priority: P5)

For every tagged release, a Software Bill of Materials (SBOM) is automatically generated and attached to the GitHub release as a downloadable artifact. Users and security teams who consume Synwire can use the SBOM to audit what components are included, satisfy compliance requirements, and correlate against future vulnerability disclosures.

**Why this priority**: SBOMs are increasingly required by downstream users, enterprise adopters, and regulatory frameworks. Generating one at release time is low-effort with open source tooling and provides long-term value. Anchore Syft is open source, non-commercial, and produces industry-standard SBOM formats.

**Independent Test**: Can be tested by creating a test release tag and confirming an SBOM artifact is attached to the release with complete dependency coverage.

**Acceptance Scenarios**:

1. **Given** a release tag is pushed, **When** the release workflow runs, **Then** an SBOM covering all direct and transitive Cargo dependencies is generated and attached to the GitHub release.
2. **Given** an SBOM is generated, **When** a user downloads it, **Then** it is in a recognised standard format (SPDX or CycloneDX) and lists all packages with their versions and licences.
3. **Given** a new crate is added to the workspace, **When** the next release SBOM is generated, **Then** the new crate and its transitive dependencies appear in the SBOM.

---

### Edge Cases

- What happens when CodeQL cannot build the project (e.g., missing system dependencies)? The scan must fail with a clear diagnostic rather than silently passing.
- What happens if OSV-Scanner detects a vulnerability that is already tracked and intentionally suppressed (e.g., no fix available upstream)? The suppression must be file-based and version-controlled so it is auditable.
- What happens when updatecli and a human contributor both update the same dependency concurrently? updatecli must detect the existing open PR and update it rather than opening a duplicate.
- What happens if CodeQL runs on a fork PR where `GITHUB_TOKEN` has reduced write permissions? The scan must still complete; result upload failures must not cause the workflow to error.
- What happens if the OSV database is temporarily unreachable? The scan should fail clearly rather than silently pass, to avoid false negatives.
- What happens when Scorecard checks require write access to the repository (e.g., uploading SARIF results) but the token has only read permissions? The workflow must be structured to grant upload permissions narrowly without broadening the attack surface.
- What happens if an SBOM is generated for a partial build (e.g., some crates fail to compile)? The SBOM must reflect only successfully resolved dependencies and must not silently omit crates.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: A code security scan MUST run automatically on every pull request targeting `main` and on every direct push to `main`.
- **FR-002**: Code security scan results MUST be published to the GitHub repository security dashboard so that historical findings are accessible.
- **FR-003**: Code security findings MUST be surfaced as PR annotations or check results visible to reviewers before merge.
- **FR-004**: The code security scan MUST cover the Rust codebase using a ruleset appropriate for the language.
- **FR-005**: A dependency vulnerability scan MUST run on a scheduled basis (at minimum daily) against all declared dependencies.
- **FR-006**: A dependency vulnerability scan MUST also run on pull requests to detect newly introduced vulnerable dependencies before merge.
- **FR-007**: The dependency vulnerability scan MUST cover the Rust/Cargo ecosystem.
- **FR-008**: The dependency vulnerability scan MUST cover GitHub Actions dependencies (third-party actions used in workflow files).
- **FR-009**: The dependency vulnerability scanner MUST use the OSV database as its advisory source.
- **FR-010**: A version-controlled suppression mechanism MUST exist to silence a known, accepted vulnerability finding without losing the record of that decision.
- **FR-011**: Automated dependency update proposals MUST be raised as pull requests for all Cargo dependencies when newer versions are published.
- **FR-012**: Automated dependency update proposals MUST be raised for GitHub Actions dependencies when newer versions are published.
- **FR-013**: Each automated dependency update PR MUST identify the updated package, old version, and new version in the PR description.
- **FR-014**: Automated dependency update PRs MUST trigger the full CI pipeline so maintainers can verify the update does not break the build.
- **FR-015**: The automated dependency update tooling MUST run as a native GitHub Actions workflow requiring no external app installation.
- **FR-016**: The automated dependency update configuration MUST target `main` as the base branch for raised PRs.
- **FR-017**: All new CI workflow definitions MUST grant only the minimum permissions required for each job and pin all third-party action references.
- **FR-018**: A supply chain posture assessment MUST run on a scheduled basis (at minimum weekly) and on every push to `main`.
- **FR-019**: Supply chain posture assessment results MUST be published to the GitHub Security dashboard in SARIF format so findings appear alongside other scan results.
- **FR-020**: An SBOM MUST be generated automatically for every tagged release of the project.
- **FR-021**: The generated SBOM MUST cover all direct and transitive Cargo dependencies resolved at build time.
- **FR-022**: The SBOM MUST be published in a recognised standard format (SPDX or CycloneDX) and attached to the corresponding GitHub release as a downloadable artifact.
- **FR-023**: Each SBOM entry MUST include at minimum the package name, version, and licence identifier.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every pull request receives a completed code security scan result before merge, with no manual trigger required, in 100% of cases.
- **SC-002**: Security findings introduced in a PR are visible to reviewers within the standard CI cycle time (target: under 15 minutes from PR open to result available).
- **SC-003**: Dependency vulnerabilities with OSV advisories are detected and reported within 24 hours of the next scheduled scan following public disclosure.
- **SC-004**: Dependency vulnerabilities introduced by a PR are flagged before merge in 100% of cases where an active OSV advisory exists.
- **SC-005**: Automated dependency update PRs are raised within the updatecli workflow's scheduled run cycle following a new dependency release.
- **SC-006**: Vulnerability suppression decisions are all version-controlled and reviewable in the repository, with zero suppressions applied outside of the designated suppression file.
- **SC-007**: A supply chain posture score is generated and visible in the security dashboard within 24 hours of any change to CI/CD configuration or branch protection settings.
- **SC-008**: An SBOM is attached to 100% of tagged releases with no manual steps required from the release author.
- **SC-009**: Every SBOM lists all direct and transitive Cargo dependencies — zero omissions for packages present in the resolved dependency graph.

## Assumptions

- The repository is public; GitHub Advanced Security (CodeQL) is available at no additional cost.
- updatecli runs as a native GitHub Actions workflow; no external bot installation is required.
- The existing `cargo audit` nightly job remains in place; OSV-Scanner coverage is additive.
- Pinning GitHub Actions to full commit SHAs is the preferred approach over mutable tag references.
- Vulnerability suppression uses the OSV-Scanner ignore file format, stored in version control.
- OSSF Scorecard requires the repository to be public; it reads public GitHub API data and cannot assess private repositories without a token with broader scope.
- The SBOM is generated from the resolved Cargo dependency graph at release time; it covers Rust crates only (not system packages or GitHub Actions tooling).
- SPDX format is the preferred SBOM output; CycloneDX is an acceptable alternative if tooling constraints require it.
