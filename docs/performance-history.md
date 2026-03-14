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
