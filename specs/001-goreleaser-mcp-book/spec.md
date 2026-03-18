# Feature Specification: GoReleaser MCP Server Binaries & Book Publishing Fix

**Feature Branch**: `001-goreleaser-mcp-book`
**Created**: 2026-03-17
**Status**: Draft
**Input**: User description: "goreleaser with crosscompiled builds of the mcp server and fix the book publishing so it doesn't 404 anymore"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Download Pre-built MCP Server Binary (Priority: P1)

A developer wants to use the Synwire MCP server without installing a Rust toolchain. They go to the GitHub Releases page, download a pre-built binary for their operating system and CPU architecture, and immediately run it without any build step.

**Why this priority**: The MCP server is the primary integration point for tool consumers. Requiring users to compile from source is a significant barrier to adoption. Pre-built binaries are the standard distribution mechanism for CLI tools and MCP servers.

**Independent Test**: Can be tested by visiting the GitHub Releases page after a tagged release and confirming downloadable archives exist for at least Linux amd64, Linux arm64, macOS amd64, and macOS arm64.

**Acceptance Scenarios**:

1. **Given** a new version tag is pushed, **When** the release pipeline completes, **Then** the GitHub Release contains downloadable archives for all supported platforms containing the `synwire-mcp-server` binary.
2. **Given** a user downloads the Linux amd64 archive, **When** they extract and run the binary on a Linux amd64 machine, **Then** the binary executes without requiring any additional runtime dependencies.
3. **Given** a user downloads the macOS arm64 archive, **When** they extract and run the binary on an Apple Silicon Mac, **Then** the binary executes correctly.
4. **Given** a release is published, **When** a user visits the GitHub Release page, **Then** each platform archive is labelled with the OS, architecture, and version in the filename.

---

### User Story 2 - Documentation Site Loads Successfully (Priority: P2)

A developer navigates to the Synwire documentation site (hosted on GitHub Pages) and the homepage loads. Currently, the homepage returns a 404 because the deployment workflow fails at the first step and nothing is ever deployed.

**Why this priority**: A documentation site that returns 404 on the homepage is completely inaccessible. The entire site is unreachable, not individual pages. This is a correctness fix for a broken deployment workflow.

**Independent Test**: Can be tested by pushing to `main` and confirming the workflow completes successfully, then visiting the homepage and receiving a 200 response.

**Acceptance Scenarios**:

1. **Given** a push to `main`, **When** the Pages deployment workflow runs, **Then** it completes without error and produces a deployment.
2. **Given** the documentation site is deployed, **When** a user visits the homepage URL, **Then** the page loads with status 200.
3. **Given** the documentation site is deployed, **When** a user clicks links in the sidebar or body text, **Then** pages load correctly.
4. **Given** a new version is pushed to `main`, **When** the Pages deployment workflow completes, **Then** the deployed site has no 404 responses on any page referenced in the table of contents.

---

### Edge Cases

- What happens when a release tag is pushed but the binary fails to compile for one target? The release should fail visibly rather than publishing a release with missing platform binaries.
- What happens if the GitHub Pages deployment fails mid-way? The previous deployment should remain live; no partial deployment should leave the site broken.
- What if the mdBook version used in CI differs from the version used locally? The build output and link resolution must be identical regardless of where the build runs.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The release pipeline MUST produce a cross-compiled `synwire-mcp-server` binary for at minimum: Linux amd64, Linux arm64, macOS amd64 (x86_64), macOS arm64 (Apple Silicon).
- **FR-002**: Each platform binary MUST be packaged in a compressed archive with a consistent naming convention that includes the project name, version, target OS, and CPU architecture.
- **FR-003**: All platform archives MUST be attached as downloadable assets to the GitHub Release created when a version tag is pushed.
- **FR-004**: The release configuration MUST be version-controlled in the repository so that any contributor can understand and reproduce the release process.
- **FR-004a**: The cross-compilation release tooling MUST replace the existing `github-release` job in the release workflow — a single release per tag, containing binary assets, changelog, and a crate table.
- **FR-005**: The Pages deployment workflow MUST upload the directory containing the root `index.html` so that GitHub Pages serves the homepage correctly.
- **FR-006**: The documentation site MUST be deployed to GitHub Pages such that the homepage and all pages referenced in the table of contents return HTTP 200.
- **FR-007**: The documentation build MUST use a pinned, reproducible tool version so builds are consistent across local and CI environments.
- **FR-009**: The MCP server binary MUST be statically linked (no dynamic library dependencies on the target platform) so it runs without installing system libraries.
- **FR-010**: Release binaries for macOS targets SHOULD be produced for both Intel and Apple Silicon architectures.
- **FR-011**: macOS release binaries MUST be ad-hoc signed so that Gatekeeper does not block execution after download, without requiring an Apple Developer account.
- **FR-013**: The release pipeline MUST publish a Homebrew formula to `randomvariable/homebrew-tap` so that macOS users can install via `brew install randomvariable/tap/synwire-mcp-server` (Homebrew re-signs on install, resolving the quarantine issue for tap users).
- **FR-012**: A SHA-256 checksum file MUST be published as a release asset alongside the binary archives so users can verify download integrity.

### Key Entities

- **Release Artifact**: A versioned, compressed archive containing the `synwire-mcp-server` binary for a specific OS and CPU architecture. Has: name (includes version + platform), binary. Accompanied by a SHA-256 checksum file.
- **Platform Target**: A combination of operating system and CPU architecture for which a binary is produced. Examples: Linux/amd64, Linux/arm64, macOS/amd64, macOS/arm64.
- **Documentation Site**: The mdBook-generated static site deployed to GitHub Pages. Has: a base URL path determined by the repository name, all pages from the table of contents.
- **Release Pipeline**: The automated workflow triggered by a version tag that produces and publishes all release artifacts.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: After any tagged release, downloadable binary archives exist on the GitHub Release page for at least 4 platform targets (Linux amd64, Linux arm64, macOS amd64, macOS arm64) within the pipeline completion time.
- **SC-002**: A developer with no Rust toolchain installed can download, extract, and run the `synwire-mcp-server` binary on a supported platform within 2 minutes.
- **SC-003**: 100% of pages listed in the documentation table of contents return HTTP 200 when browsed on the deployed GitHub Pages site.
- **SC-004**: Zero broken internal navigation links exist in the deployed documentation (sidebar, next/previous chapter, in-body cross-references).
- **SC-005**: The release pipeline configuration can be understood and reproduced by any contributor by reading a single configuration file in the repository.

## Clarifications

### Session 2026-03-17

- Q: Does GoReleaser replace the existing `github-release` job or supplement it? → A: GoReleaser replaces the `github-release` job entirely — one release, one config.
- Q: Should macOS binaries be signed? → A: Ad-hoc signing — removes Gatekeeper block, no Apple Developer account required.
- Q: Should SHA-256 checksums be published with release artifacts? → A: Yes — publish a checksum file alongside each release.

## Assumptions

- The root cause of the homepage 404 is that the Pages deployment workflow uploads `docs/book` but mdBook writes HTML output to `docs/book/html`. The artifact root contains an `html/` subdirectory with no `index.html` at the root level, so GitHub Pages returns 404 for every URL including the homepage. The fix is uploading `docs/book/html` instead.
- The `synwire-mcp-server` binary is the only binary that requires cross-compiled distribution at this time. Library crates continue to be distributed via crates.io only.
- Windows is not a required platform target for the initial implementation, but the release configuration should not prevent adding it later.
- The cross-compilation approach must work within standard GitHub Actions runners without requiring custom hardware or external build services.
- A SHA-256 checksum file MUST be published alongside release archives.
