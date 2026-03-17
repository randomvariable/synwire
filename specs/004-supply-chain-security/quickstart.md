# Supply Chain Security — Maintainer How-To Guide

This guide covers the day-to-day actions a maintainer needs to take in response to each security tool in the Synwire CI pipeline.

---

## CodeQL (SAST)

**Where findings appear**: GitHub → Security → Code scanning alerts

### Reviewing a CodeQL finding on a PR

1. Open the PR. The "Code scanning / CodeQL" check must pass before merge.
2. If findings exist, they appear as inline annotations on the diff and as entries under "Code scanning alerts" in the Security tab.
3. Each alert shows: the query rule ID (e.g., `rust/path-injection`), the severity, the affected file and line, and a description of the vulnerability class.
4. Review the alert and the highlighted code path.

### Dismissing a false positive

1. Go to Security → Code scanning alerts.
2. Open the alert.
3. Click "Dismiss alert" → select a reason ("False positive", "Won't fix", or "Used in tests").
4. Add a brief justification. The dismissal is recorded in the audit log.

### Known limitation

CodeQL findings that originate inside proc-macro-generated code (e.g., `serde` derive expansions) are reported at the macro invocation site, not the generated code. This is an upstream limitation with no current fix. Treat such findings at annotation sites with additional scrutiny.

---

## OSV-Scanner (Dependency Vulnerability Detection)

**Where findings appear**: GitHub → Security → Code scanning alerts (under "OSV-Scanner")

### Understanding the two scan modes

- **PR scan**: Runs on every pull request. Reports only vulnerabilities *newly introduced* by the PR (i.e., not present in the base branch). This prevents pre-existing issues from blocking every PR.
- **Scheduled scan**: Runs daily at 02:00 UTC. Full scan of `Cargo.lock` and all GitHub Actions workflow files. Reports all known vulnerabilities in current dependencies.

### Acting on a vulnerability finding

1. Read the advisory URL in the finding to understand severity and affected versions.
2. Check whether a fixed version is available in the upstream crate.
3. Update the dependency in `Cargo.toml` to a patched version, then open a PR. CI will re-run the OSV scan and confirm the fix.

### Suppressing a known-accepted vulnerability

If no fix is available and you have accepted the risk:

1. Open `.osv-scanner.toml` at the repo root.
2. Add an entry:
   ```toml
   [[IgnoredVulns]]
   id = "GHSA-xxxx-xxxx-xxxx"   # or CVE-YYYY-NNNNN or RUSTSEC-YYYY-NNNN
   reason = "No fix available upstream; risk accepted 2026-03-17, revisit by 2026-09-17"
   ```
3. Commit the change. The suppression is version-controlled and reviewable.
4. Set a calendar reminder to revisit the suppression on the stated date.

---

## OSSF Scorecard (Supply Chain Posture)

**Where findings appear**: GitHub → Security → Code scanning alerts (under "Scorecard")

### First-time setup (one-off)

Scorecard needs a GitHub personal access token for higher GitHub API rate limits (optional but recommended — the workflow falls back to `GITHUB_TOKEN` without it):

1. Create a PAT with minimal scope:
   - **Fine-grained PAT**: Repository access → `randomvariable/synwire` → no additional permissions needed beyond the default read access.
   - **Classic PAT**: Check `public_repo` scope only. No write permissions.
2. Add it as a repository secret:
   - Navigate to the repository → **Settings** → **Secrets and variables** → **Actions** → **New repository secret**.
   - Name: `SCORECARD_TOKEN`
   - Value: paste the token.
   - Click **Add secret**.

### Interpreting the score

Each Scorecard check produces a score from 0–10. A failed check appears as a code scanning alert with the check name and a description of what to fix. Common fixes:

| Check | Common fix |
|---|---|
| Pinned-Dependencies | Replace `uses: owner/repo@v1` with `uses: owner/repo@<SHA>` |
| Branch-Protection | Enable required reviews and status checks in branch settings |
| Token-Permissions | Add `permissions: {}` at workflow top level; grant per-job minimums |
| Code-Review | Ensure PRs cannot be merged without at least one approval |

### Pinning an action SHA

Use the GitHub CLI to resolve a tag to its SHA:
```bash
gh api repos/<owner>/<repo>/git/ref/tags/<tag> --jq '.object.sha'
```
Then update the workflow file:
```yaml
uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683  # v4.2.2
```
Add the human-readable tag as a comment so updatecli can match and update it.

---

## updatecli (Automated Dependency Updates)

**Where PRs appear**: GitHub → Pull Requests (labelled `dependencies`)

### How it works

updatecli runs weekly (Monday 05:00 UTC) as a GitHub Actions workflow. It reads the policy files in `updatecli/updatecli.d/`, checks for newer versions of each declared dependency, and opens a PR for each update found. If a PR already exists for that update (e.g., from a previous week's run), it updates the existing PR branch rather than opening a duplicate.

### Merging an update PR

1. Open the PR. Verify CI passes (all existing checks run on the update branch).
2. Review the version diff in `Cargo.toml` or the workflow YAML.
3. Merge when satisfied. No special steps required.

### Triggering updatecli manually

Go to Actions → updatecli → Run workflow. This is useful after adding a new dependency that you want updatecli to track immediately.

### Adding a new dependency to updatecli tracking

Edit `updatecli/updatecli.d/cargo.yaml` and add a new source/condition/target block for the crate. Follow the existing pattern in the file. Commit the change; updatecli will pick it up on the next scheduled run.

---

## Anchore Syft (SBOM — Software Bill of Materials)

**Where the SBOM appears**: GitHub → Releases → attached asset (`synwire-<version>-sbom.spdx.json`)

### Accessing the SBOM for a release

1. Go to Releases.
2. Find the release version.
3. Under "Assets", download `synwire-<version>-sbom.spdx.json`.

### Reading the SBOM

The SBOM is in SPDX 2.3 JSON format. Each entry (`packages` array) contains:
- `name` — crate name
- `versionInfo` — version string
- `licenseConcluded` / `licenseDeclared` — licence from `Cargo.toml`
- `externalRefs` — includes a `purl` (Package URL) for cross-tool compatibility

### Correlating the SBOM against future advisories

Use any OSV-compatible tool to scan the SBOM file after the fact:
```bash
osv-scanner --sbom synwire-<version>-sbom.spdx.json
```
This is useful for auditing a historical release against newly published advisories.
