# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| Latest stable release | ✓ Security fixes backported |
| Previous minor release | Best-effort |
| Older releases | Not supported |

## Reporting a Vulnerability

**Please do not open a public GitHub issue for security vulnerabilities.**

Use [GitHub private vulnerability reporting](https://github.com/randomvariable/synwire/security/advisories/new) instead:

1. Navigate to the [Security Advisories](https://github.com/randomvariable/synwire/security/advisories) page.
2. Click **"Report a vulnerability"**.
3. Fill in the description, affected versions, and any reproduction steps.
4. Submit the report.

Reports are reviewed by the maintainers within **5 business days**. You will receive an acknowledgement and a timeline for a fix within that window. A CVE will be requested if applicable once a patch is ready.

## Scope

This policy covers the Synwire Rust library crates published to crates.io. It does not cover third-party dependencies (report those to the upstream project), or issues in development tooling (cargo-make tasks, CI workflows) that are not exploitable in a deployed context.

## Dependency Vulnerability Disclosure

Known vulnerabilities in Synwire's dependencies are tracked by OSV-Scanner (scheduled daily scan). Results appear in the [GitHub Security tab](https://github.com/randomvariable/synwire/security/code-scanning). Suppressed vulnerabilities are recorded with justifications in [`.osv-scanner.toml`](.osv-scanner.toml).
