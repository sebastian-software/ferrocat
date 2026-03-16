# Test Coverage

`ferrocat` now exposes workspace-local Cargo aliases for coverage reporting through `cargo-llvm-cov`.

The coverage commands intentionally exclude:

- `ferrocat-bench`, which is a benchmark harness rather than shipped library code
- `ferrocat-conformance`, which is fixture data and expectations for the upstream snapshot

That keeps the report focused on the public and internal library crates that matter most in day-to-day changes:

- `ferrocat`
- `ferrocat-po`
- `ferrocat-icu`

## Local Setup

Install the required tooling once:

```bash
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov
```

## Local Commands

Print a terminal summary:

```bash
cargo coverage-summary
```

Build an HTML report:

```bash
cargo coverage
```

The HTML output is written to:

```text
target/coverage/html/index.html
```

Generate an LCOV file for external tooling:

```bash
cargo coverage-lcov
```

The LCOV output is written to:

```text
target/lcov.info
```

## CI

The GitHub Actions CI workflow runs a dedicated coverage job on Ubuntu, installs the required LLVM tooling, uploads the generated LCOV report to Codecov, and stores the same `lcov.info` file as a workflow artifact.

For private repositories, add a GitHub Actions repository secret named `CODECOV_TOKEN`.

For public repositories, tokenless upload may also work with `codecov/codecov-action@v5` if token authentication has been disabled in the Codecov organization settings. If you want the least surprising setup, keep the `CODECOV_TOKEN` secret configured either way.
