# Security Policy

## Supported versions

Security fixes are provided for the latest release on the default branch. Older
releases are not patched separately — upgrade to the latest version to receive a
fix.

## Reporting a vulnerability

Report suspected vulnerabilities privately. Do not open a public issue, pull
request, or discussion for a vulnerability that has not been fixed yet.

Two private channels are available:

- **GitHub private vulnerability reporting** —
  <https://github.com/sebastian-software/ferrocat/security/advisories/new>, or
  open this repository's **Security** tab and choose **Report a vulnerability**.
- **Email** — security@sebastian-software.de.

Include a concise description, the affected crate (`ferrocat`, `ferrocat-po`,
`ferrocat-icu`, or `ferrocat-cli`) and version or commit, reproduction steps,
the impact you expect, and any suggested fix or known mitigation. Leave out
credentials and data you are not allowed to share.

## Response expectations

Maintainers aim to:

- Acknowledge a private report within 7 days.
- Assess severity and affected versions within 14 days.
- Coordinate a fix and a disclosure timeline with the reporter.
- Credit the reporter when desired and appropriate.

Timing can vary for low-impact reports and for reports that depend on a fix in
an upstream dependency.

## Scope

In scope: anything in this repository that lets someone read, modify, or execute
something they should not — including the published crates, the parsers for
untrusted `.po` and ICU MessageFormat input, the runtime-compiled catalogs, and
the build and release automation.

Usually out of scope: reports without a concrete impact path, vulnerabilities in
third-party dependencies used as documented, and problems that require an
already-compromised machine or a deliberately corrupted local state.
