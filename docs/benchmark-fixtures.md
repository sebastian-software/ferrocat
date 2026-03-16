# Benchmark Fixtures

`ferrocat-bench` uses three fixture classes:

- static fixtures
  - small, hand-written PO files for quick smoke runs
- generated mixed fixtures
  - deterministic corpora for realistic performance tracking
- generated gettext compatibility fixtures
  - deterministic corpora for realistic cross-tool gettext benchmarks

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

## Gettext Compatibility Profiles

The official external benchmark suite uses `gettext-*` fixtures built for classic gettext workflows.

Current family and locale combinations:

- `gettext-ui-de-1000`
- `gettext-ui-de-10000`
- `gettext-commerce-pl-1000`
- `gettext-commerce-pl-10000`
- `gettext-saas-fr-1000`
- `gettext-saas-fr-10000`
- `gettext-content-ar-1000`
- `gettext-content-ar-10000`

These fixtures intentionally stay within broadly supported gettext features:

- headers with `Language` and `Plural-Forms`
- `msgid`, `msgstr`, `msgid_plural`, `msgstr[n]`
- translator comments `#`
- extracted comments `#.`
- references `#:`
- flags such as `#, fuzzy` and `#, c-format`
- `msgctxt`
- multiline strings and escaped quotes

They intentionally avoid:

- ICU content
- `#@` metadata comments
- obsolete entries
- parser-specific extensions

The official external compare profile still applies a support matrix on top of these families:

- `polib` and `pofile` are currently only used on the most conservative `gettext-ui-de-*` scenarios
- `msgcat` is used on the broader stringify scenarios, including plural-heavier locale profiles
- the other gettext families stay valuable as realistic classical workload corpora even when a given third-party parser is not part of that comparison group

## Useful Commands

```bash
cargo run -p ferrocat-bench -- describe mixed-1000
cargo run -p ferrocat-bench -- parse mixed-1000 200
cargo run -p ferrocat-bench -- stringify mixed-1000 200
cargo run -p ferrocat-bench -- describe gettext-ui-de-1000
cargo instruments --no-open -o target/instruments/parse-mixed-1000.trace -t "Time Profiler" --bin ferrocat-bench -- parse mixed-1000 200
```
