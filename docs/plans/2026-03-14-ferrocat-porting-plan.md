# ferrocat Porting Plan for `pofile-ts`

## Goal

Build a 100% idiomatic, high-performance Rust implementation of the relevant
`pofile-ts` features and beat the original by a wide margin on realistic PO
workloads.

This should not be a line-by-line port. The design target is:

- zero-copy or low-copy parsing where it matters
- predictable allocation behavior
- cache-friendly data structures
- explicit hot-path specialization
- optional CPU acceleration where it measurably helps

## Confirmed Decisions

The current direction for `ferrocat` is:

1. phase 1 includes both PO and the important ICU core
2. not every small helper from `pofile-ts` needs parity immediately
3. future N-API bindings should be considered in the architecture, but not built first
4. implementation starts with:
   - `parse_po`
   - `stringify_po`
   - low-level escape/unescape and scanning hot paths

## Reference Scope Observed in `pofile-ts`

`pofile-ts` currently exposes four major capability groups:

1. PO parsing and serialization
2. PO item/catalog transformation helpers
3. plural-form and message-id helpers
4. ICU parsing, conversion, and compilation

For a Rust-first product, these should be treated as separate layers rather than
one monolithic crate.

## Recommended Product Split

### Phase 1 crate boundary

- `ferrocat-po`
  - parse `.po` text into typed structures
  - serialize typed structures back into `.po`
  - support comments, metadata, flags, references, context, plurals, obsolete items
- `ferrocat-icu`
  - parse ICU messages into a compact Rust AST
  - provide a performance-first foundation for later validation/compilation
- `ferrocat-bench`
  - benchmark harness and corpus tooling

### Later crates

- `ferrocat-catalog`
  - catalog transforms and message-id helpers
- `ferrocat-icu`
- `ferrocat-compile`
  - runtime formatter / codegen / compiled catalogs
- `ferrocat-napi`
  - N-API bindings once the Rust core stabilizes

This keeps the hot-path parser work lean, starts ICU early as requested, and
still keeps the binding layer decoupled from the core.

## Recommended Priority

If the immediate objective is "beat `pofile-ts` by a lot", the order should be:

1. `parse_po`
2. `stringify_po`
3. low-level string scanning / escape / unescape
4. ICU parser core
5. plural/header helpers
6. catalog transforms
7. ICU compiler/codegen

Reason: PO parse/stringify is the clearest place to win early, prove the
architecture, and establish benchmark discipline before broadening further into
the much larger ICU surface.

## High-Value Features to Port First

### Must-have parity for PO v1

- file-level comments and extracted comments
- ordered headers
- `msgid`
- `msgstr`
- `msgctxt`
- `msgid_plural`
- plural `msgstr[n]`
- references (`#:`)
- flags (`#,`)
- extracted comments (`#.`)
- translator comments (`#`)
- metadata comments (`#@ key: value`)
- obsolete items (`#~`)
- CRLF normalization
- multiline strings and escape handling

### Nice-to-have after core parity

- catalog conversion helpers
- message-id generation
- plural helper API
- richer parse diagnostics with byte offsets and spans
- focused ICU analysis helpers such as variable extraction and validation

### Defer unless strategically needed

- JS/TS code generation compatibility
- browser-oriented API symmetry
- N-API bindings
- long-tail helper parity

## What the Current TS Implementation Optimizes

The relevant `pofile-ts` parser hot path is already fairly disciplined:

- first-character dispatch for line classification
- fast-path handling for common `msgid` / `msgstr` cases
- limited regex use in the hot path
- compact state machine
- escape/unescape shortcuts

That means beating it "by a lot" will likely come from Rust fundamentals more
than from fancy algorithms alone:

- fewer intermediate strings
- fewer passes over the data
- arena-like ownership strategy
- faster scanning for newline / quote / backslash / `#` / `m`
- better serialization buffering

## Rust Design Recommendation

### Public model

Use a typed, idiomatic owned model for the stable API:

```rust
pub struct PoFile {
    pub comments: Vec<String>,
    pub extracted_comments: Vec<String>,
    pub headers: Vec<Header>,
    pub items: Vec<PoItem>,
}

pub struct Header {
    pub key: String,
    pub value: String,
}

pub struct PoItem {
    pub msgid: String,
    pub msgctxt: Option<String>,
    pub references: Vec<String>,
    pub msgid_plural: Option<String>,
    pub msgstr: Vec<String>,
    pub comments: Vec<String>,
    pub extracted_comments: Vec<String>,
    pub flags: Vec<String>,
    pub metadata: Vec<(String, String)>,
    pub obsolete: bool,
}
```

Avoid `HashMap` in the core model for ordered/compact fields such as headers,
flags, and metadata. `Vec` is usually more cache-friendly and preserves source
ordering naturally.

### Internal model

Internally parse from `&[u8]`, not `&str`, and convert to UTF-8 strings only
when a field is finalized.

Recommended internal tactics:

- byte-slice scanner
- offsets into the source buffer during parse
- one owned allocation per finalized logical field where possible
- `memchr`/`memchr2`/`memchr3` style scanning for structural bytes

### Error strategy

Offer two modes:

- forgiving parser for broad compatibility
- strict parser with structured errors and byte/line/column positions

That gives us parity with permissive JS behavior without sacrificing a strong
Rust API.

## Performance Plan

### First-order optimizations

- parse from bytes, not chars
- use `memchr` for newline, quote, colon, backslash
- avoid `String` creation until a field is complete
- reserve output vectors with rough heuristics
- serialize into a single `String` with capacity pre-estimation
- split hot paths for:
  - simple single-line unescaped strings
  - multiline strings
  - escaped strings
  - plural items

### Second-order optimizations

- small-vector strategy for tiny lists:
  - comments
  - flags
  - references
  - `msgstr`
- branch-friendly line classifier using first byte
- specialized fast path for ASCII-only lines
- compact enum state machine for current field

### SIMD and CPU-specific acceleration

SIMD can help, but only in specific places:

- scanning for newline / quote / backslash / `#`
- ASCII validation / fast-path detection
- escape detection during serialization

Suggested approach:

1. start with `memchr`
2. benchmark
3. only then add optional SIMD paths

Good candidates:

- `memchr` crate as default baseline
- `std::arch` intrinsics for opt-in SSE2/AVX2/NEON kernels
- runtime feature detection on x86_64 and aarch64

Important: NEON/SIMD should live behind a narrow internal module boundary.
Do not leak architecture-specific complexity into the parser state machine.

## Benchmark Strategy

We should not benchmark only synthetic files. Use three corpus classes:

1. tiny
   - very small files
   - many repeated invocations
2. realistic
   - medium real-world PO files with comments, plurals, and context
3. stress
   - very large files
   - long strings
   - heavy escaping
   - many obsolete/comment blocks

Metrics:

- ops/s
- bytes/s
- ns/item
- allocations/item
- total allocated bytes

Targets:

- parse: at least 3-5x faster than `pofile-ts`
- stringify: at least 2-3x faster than `pofile-ts`
- lower allocation count by an order of magnitude on common cases

If we hit those numbers first, SIMD may widen the gap further.

## Profiling Strategy

Primary profiling tools on this machine:

- `cargo-instruments`
- Apple Instruments / `xctrace`

Recommended workflow:

1. use Criterion or a small dedicated benchmark binary for repeatable timings
2. use `cargo-instruments` for hotspot discovery on realistic fixtures
3. only optimize after confirming the top self-time and allocation sites
4. re-run both benchmark and profile after each optimization batch

Recommended Instruments templates:

- `Time Profiler`
  - default choice for parser and serializer hotspots
- `Allocations`
  - confirm whether wins come from fewer heap operations
- `System Trace`
  - only if we later investigate scheduling or I/O effects

Suggested profiling targets once code exists:

- `parse_po` on tiny, realistic, and stress fixtures
- `stringify_po` on parsed realistic/stress catalogs
- isolated escape/unescape microbenchmarks
- isolated line-scanning microbenchmarks

Important:

- always profile release builds
- keep fixture inputs pinned and versioned
- compare pre- and post-optimization call trees, not only wall-clock numbers
- treat SIMD/NEON as justified only if the profile still shows scanning as a
  dominant cost after the baseline Rust parser is tuned

Example commands for the expected workflow:

```bash
cargo bench
cargo instruments -t "Time Profiler" --bench parse_po
cargo instruments -t "Allocations" --bench parse_po
```

## Proposed Milestones

### M0: Ground truth

- mirror the `pofile-ts` fixtures
- define parity expectations
- port benchmark corpus
- define success thresholds

### M1: Parser MVP

- parse headers
- parse items
- parse comments/flags/references
- parse plurals/context/obsolete
- parse multiline strings
- support forgiving mode

Deliverable:

- `parse_po(&str) -> Result<PoFile, ParseError>`

### M2: Serializer MVP

- serialize complete `PoFile`
- support fold length policy
- preserve header ordering
- preserve item structure

Deliverable:

- `stringify_po(&PoFile, SerializeOptions) -> String`

### M3: Fast-path tuning

- profile parser and serializer
- add capacity heuristics
- optimize escape/unescape
- introduce `smallvec` if it wins
- introduce optional SIMD scanning if it wins

### M4: ICU parser MVP

- define compact ICU AST
- support arguments, plurals, selects, selectordinals, and tags
- add strict and forgiving parse modes
- benchmark parser against realistic message sets

### M5: Transformation helpers

- catalog conversion
- plural helper API
- message-id generation

### M6: ICU decision point

Only after M3/M4 do we choose between:

- full idiomatic Rust ICU parser/compiler
- partial ICU support
- separate crate with independent roadmap

## Concrete Hotspots to Focus On First

If we want the first 20% of work to produce 80% of the gain, focus on:

1. line scanning
2. quoted string extraction
3. unescape handling
4. multiline continuation appending
5. serialization escape detection and output buffering

These are the most likely places where Rust will dominate the TS original.

## Suggested API Direction

Prefer a Rust-native API, then add compatibility adapters if needed.

Good:

```rust
pub fn parse_po(input: &str) -> Result<PoFile, ParseError>;
pub fn parse_po_lossy(input: &str) -> PoFile;
pub fn stringify_po(file: &PoFile) -> String;
```

Avoid:

- JS-shaped APIs that privilege object-literal flexibility over type safety
- overusing maps where ordered vectors are enough
- carrying browser/runtime constraints into the Rust core

## Biggest Architectural Choice Still Open

The critical scope question is whether `ferrocat` is:

1. a best-in-class PO engine first
2. a full Rust i18n toolkit matching `pofile-ts`
3. a PO engine plus optional ICU crates later

Recommendation:

Start with option 3.

That still matches the direction above: PO is the first performance milestone,
ICU starts early as a separate crate, and N-API stays a later adapter layer.

## Recommended Immediate Next Step

Implement and benchmark only these methods first:

1. `parse_po`
2. `stringify_po`
3. low-level string escape/unescape helpers

Once those are measurably ahead, we can lock the public data model and expand
sideways into catalog/plural helpers.
