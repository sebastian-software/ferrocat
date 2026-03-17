# ADR Index

This folder contains architecture decision records in a lightweight Nygard-style format.

Current records:

- [0001: Rust-Native, Performance-First Core](0001-rust-native-performance-first-core.md)
- [0002: Provide Both Owned and Borrowed Parse APIs](0002-owned-and-borrowed-parse-apis.md)
- [0003: Use Byte-Oriented Scanning and Separate Structure From Semantics](0003-byte-oriented-scanning-and-structural-separation.md)
- [0004: Model `msgstr` by Semantics Instead of Always Using `Vec<String>`](0004-model-msgstr-by-semantics.md)
- [0005: Treat ICU as the Canonical Model and Gettext as a Compatibility Bridge](0005-icu-canonical-model-and-gettext-bridge.md)
- [0006: Separate the PO Core from the High-Level Catalog API](0006-separate-po-core-from-high-level-catalog-api.md)
- [0007: Do Not Support `previous_msgid` History](0007-drop-previous-msgid-history.md)
- [0008: Normalize Headerless PO Files on Write](0008-normalize-headerless-po-output.md)
- [0009: Use Versioned, Truncated SHA-256 Keys for Runtime Catalog Compilation](0009-runtime-catalog-compiled-key-strategy.md)
- [0010: Add a Locale-Resolved Compiled Catalog Artifact API](0010-compiled-catalog-artifact-api.md)
