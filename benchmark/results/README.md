# Benchmark Reports

Store official `ferrocat-bench compare` JSON reports in this directory or a
dated subdirectory below it.

Scheduled GitHub Actions benchmark runs upload JSON reports as workflow
artifacts instead of committing them here. Copy a report into this directory
only when it is an intentional release or publication checkpoint.

## Consumer contract

[`compare-report.schema.json`](../compare-report.schema.json) describes the reusable subset of the existing
`ferrocat-bench compare` output. It is a JSON Schema (draft 2020-12); extra
fields are allowed. Consumers should tolerate additions and use scenario IDs
and comparison groups rather than assuming a fixed order; IDs can repeat. The
schema does not declare all producer fields stable. See [ADR 0035](../../docs/app/routes/architecture/adr/0035-benchmark-report-contract.mdx)
for the compatibility and runner boundaries.

Validate retained reports, or a newly generated report, with Node.js:

```sh
node scripts/check-benchmark-report-contract.mjs
node scripts/check-benchmark-report-contract.mjs target/benchmark/report.json
```

`generated_at` is the UTC generation time; `environment.git_sha` identifies the
checkout commit when available and can be `unknown`. Host and toolchain fields
describe the measured environment without a hostname. The report does not
record dirty state or build flags, so retain those separately when needed.
The schema checks structure only: use the workload, fixture, semantic digest,
samples, noise and environment to judge whether results can be compared.
