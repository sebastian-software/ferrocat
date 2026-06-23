# Fuzz Seed Corpus

These inputs bootstrap the scheduled fuzz workflow. They are intentionally
small, deterministic examples that cover valid and invalid PO, NDJSON, and ICU
MessageFormat shapes before libFuzzer mutates them.

Do not store generated corpus growth here. Keep `fuzz/corpus/` ignored and
promote only reduced, stable crash reproducers or high-value fixtures into this
directory.
