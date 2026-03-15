# Performance History

Append-only history for benchmark and profiling checkpoints.

Rules:

- add rows, do not replace older numbers
- keep command, fixture, build profile, and notes explicit
- prefer comparable reruns over ad-hoc measurements

| Date | Area | Build | Command | Fixture | Iterations | Iter/s | MiB/s | Notes |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | --- |
| 2026-03-14 | parse | dev | `cargo run -p ferrox-bench -- parse realistic 1000` | `realistic.po` | 1000 | 20121.0 | 15.10 | Pre byte-line-scanner baseline |
| 2026-03-14 | parse | dev | `cargo run -p ferrox-bench -- parse realistic 1000` | `realistic.po` | 1000 | 29211.6 | 21.92 | Post byte-line-scanner + `memchr` refactor |
| 2026-03-14 | parse | dev | `cargo run -p ferrox-bench -- parse mixed-1000 200` | generated `mixed-1000` | 200 | 412.1 | 47.88 | 1000 entries, mixed features, deterministic corpus |
| 2026-03-14 | parse | release | `cargo run --release -p ferrox-bench -- parse mixed-1000 200` | generated `mixed-1000` | 200 | 2830.9 | 328.91 | Release baseline after byte-line-scanner refactor |
| 2026-03-14 | parse | release | `cargo run --release -p ferrox-bench -- parse mixed-1000 200` | generated `mixed-1000` | 200 | 2957.0 | 343.56 | Added borrow-or-own fast path for quoted strings |
| 2026-03-14 | parse | release | `cargo run --release -p ferrox-bench -- parse mixed-1000 200` | generated `mixed-1000` | 200 | 3041.8 | 353.41 | Centralized scanner classification/helpers without borrowed-item overhead |
| 2026-03-14 | parse | release | `cargo run --release -p ferrox-bench -- parse mixed-1000 200` | generated `mixed-1000` | 200 | 3393.1 | 394.23 | Scanner backend helpers added; repeated runs showed noticeable single-run variance |
| 2026-03-15 | parse | release | `cargo run --release -p ferrox-bench -- parse mixed-10000 200` | generated `mixed-10000` | 200 | 392.4 | 463.22 | Byte-based quoted extraction plus unchecked UTF-8 conversion on parser-owned slices; profiled run reached 395.5 iter/s |
| 2026-03-15 | parse | release | `cargo run --release -p ferrox-bench -- parse mixed-10000 200` | generated `mixed-10000` | 200 | 401.2 | 473.63 | `LineScanner` now trims only leading ASCII whitespace on the hot path and carries a smaller line record. |
| 2026-03-15 | parse | release | `cargo run --release -p ferrox-bench -- parse mixed-10000 200` | generated `mixed-10000` | 200 | 406.4 | 479.70 | Singular `msgstr[0]` now stays on a scratch string in parser state and only promotes to a `Vec<String>` for plural cases. |
| 2026-03-15 | parse | release | `cargo run --release -p ferrox-bench -- parse mixed-10000 200` | generated `mixed-10000` | 200 | 415.5 | 490.45 | Inlined scanner helpers, specialized keyword classification, and replaced `Plural-Forms` parsing with a byte-based path. |
| 2026-03-15 | parse | release | `cargo run --release -p ferrox-bench -- parse mixed-10000 200` | generated `mixed-10000` | 200 | 457.9 | 540.49 | Replaced always-`Vec<String>` `msgstr` storage with a `MsgStr` enum (`None`/`Singular`/`Plural`), removing per-item vector overhead for the common singular case. |
| 2026-03-15 | parse | release | `cargo run --release -p ferrox-bench -- parse mixed-10000 200` | generated `mixed-10000` | 200 | 473.3 | 558.71 | `ParserState::reset` now clears fields in place and reuses `PoItem` allocations instead of rebuilding the whole parser state every item. |
| 2026-03-15 | parse | release | `cargo run --release -p ferrox-bench -- parse mixed-10000 200` | generated `mixed-10000` | 200 | 483.5 | 570.72 | Split reset paths so the common post-`mem::take` case avoids a full `PoItem::clear_for_reuse`, while the header path still clears in place. |
| 2026-03-15 | parse-borrowed | release | `cargo run --release -p ferrox-bench -- parse-borrowed mixed-10000 200` | generated `mixed-10000` | 200 | 816.0 | 963.27 | First zero-copy parse path with borrowed `Cow`-backed items; header key/value are still materialized today. |
| 2026-03-15 | parse-borrowed | release | `cargo run --release -p ferrox-bench -- parse-borrowed mixed-10000 200` | generated `mixed-10000` | 200 | 835.9 | 986.75 | Borrowed parser now extracts standard header fragments directly from raw `msgstr` lines, so common `...\\n` header entries stay borrowed too. |
| 2026-03-15 | stringify | release | `cargo run --release -p ferrox-bench -- stringify mixed-10000 200` | generated `mixed-10000` | 200 | 896.5 | 1065.16 | Same `MsgStr` enum change preserved stringify throughput while using a more compact translation model. |
| 2026-03-14 | stringify | release | `cargo run --release -p ferrox-bench -- stringify mixed-1000 200` | generated `mixed-1000` | 200 | 1268.0 | 148.29 | Baseline before simple-keyword direct-write fast path |
| 2026-03-14 | stringify | release | `cargo run --release -p ferrox-bench -- stringify mixed-1000 200` | generated `mixed-1000` | 200 | 3213.3 | 375.80 | Direct fast path for common single-line keyword writes |
| 2026-03-14 | stringify | release | `cargo run --release -p ferrox-bench -- stringify mixed-1000 200` | generated `mixed-1000` | 200 | 4532.1 | 530.03 | Replaced multiline/folding `Vec<String>` pipeline with direct segmented writes; repeated runs ranged from 4246.4 to 4532.1 iter/s |
| 2026-03-14 | stringify | release | `cargo run --release -p ferrox-bench -- stringify mixed-1000 200` | generated `mixed-1000` | 200 | 7507.4 | 877.99 | Replaced temporary escaped strings with direct buffer writes; scratch buffer reused for multiline segments |
| 2026-03-14 | stringify | release | `cargo run --release -p ferrox-bench -- stringify mixed-10000 200` | generated `mixed-10000` | 200 | 830.1 | 986.28 | Same direct-escape write path confirmed on larger corpus after Time Profiler-guided optimization |
| 2026-03-15 | stringify | release | `cargo run --release -p ferrox-bench -- stringify mixed-10000 200` | generated `mixed-10000` | 200 | 881.6 | 1047.44 | Added `aarch64` NEON escape-byte scan path; repeated `mixed-10000` runs stayed around 868.7-887.2 iter/s |
| 2026-03-15 | stringify | release | `cargo run --release -p ferrox-bench -- stringify mixed-10000 200` | generated `mixed-10000` | 200 | 919.2 | 1092.16 | Reused one scratch buffer across the whole stringify pass instead of recreating multiline escape buffers per keyword |
