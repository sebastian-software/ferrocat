# Benchmark Fixtures

`ferrocat-bench` uses two fixture classes:

- static fixtures
  - small, hand-written PO files for quick smoke runs
- generated mixed fixtures
  - deterministic corpora for realistic performance tracking

## Mixed Profiles

Current generated fixture presets:

- `mixed-1000`
- `mixed-10000`

The generator intentionally mixes common PO features:

- plurals: about 10%
- translator comments: about 5%
- extracted comments: about 4%
- references: about 33%
- context: about 8%
- metadata comments: about 2%
- obsolete entries: about 1%
- multiline strings: about 3%
- escaped strings: about 2-3%

These percentages are approximate by design. The important property is that the
fixture stays deterministic so benchmark history remains comparable.

## Useful Commands

```bash
cargo run -p ferrocat-bench -- describe mixed-1000
cargo run -p ferrocat-bench -- parse mixed-1000 200
cargo run -p ferrocat-bench -- stringify mixed-1000 200
cargo instruments --no-open -o target/instruments/parse-mixed-1000.trace -t "Time Profiler" --bin ferrocat-bench -- parse mixed-1000 200
```
