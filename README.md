# ferrocat

`ferrocat` is a standalone gettext and ICU toolkit with a Rust-first core.

The repository currently contains the public Rust crate `ferrocat` and the
internal Node.js bridge crate `ferrocat-node`.

It provides:

- PO parsing and serialization
- ICU MessageFormat parsing and serializable compilation
- Catalog compilation for serializable host bindings
- Message ID generation and plural helpers
- A Rust-only runtime module for locale-aware formatting and tag rendering

The Rust crate is intended to be usable directly from Rust and to serve as the
shared core for thin host bindings such as Node.js.

## Installation

```bash
cargo add ferrocat
```

## Parse and stringify PO files

```rust
use ferrocat::{parse_po, stringify_po, SerializeOptions};

let po = parse_po(
    r#"
msgid ""
msgstr ""
"Language: de\n"

msgid "Hello"
msgstr "Hallo"
"#,
);

assert_eq!(po.items[0].msgid, "Hello");
assert_eq!(po.items[0].msgstr, vec!["Hallo"]);

let rendered = stringify_po(&po, SerializeOptions::default());
assert!(rendered.contains(r#"msgid "Hello""#));
```

## Compile ICU messages to a serializable payload

```rust
use ferrocat::{compile_icu, CompileIcuOptions, SerializedCompiledMessageKind};

let compiled = compile_icu(
    "{count, plural, one {# file} other {# files}}",
    &CompileIcuOptions::new("en"),
)
.expect("message should compile");

match compiled.kind {
    SerializedCompiledMessageKind::Icu { ast } => assert!(!ast.is_empty()),
    other => panic!("expected icu payload, got {other:?}"),
}
```

## Compile catalogs to a serializable payload

```rust
use ferrocat::{
    compile_catalog, Catalog, CatalogEntry, CatalogTranslation, CompileCatalogOptions,
    SerializedCompiledMessageKind,
};

let catalog = Catalog::from([(
    "Hello {name}!".to_owned(),
    CatalogEntry {
        translation: Some(CatalogTranslation::Singular("Hallo {name}!".to_owned())),
        ..CatalogEntry::default()
    },
)]);

let compiled = compile_catalog(&catalog, &CompileCatalogOptions::new("de"))
    .expect("catalog should compile");
assert_eq!(compiled.entries.len(), 1);
match &compiled.entries[0].message.kind {
    SerializedCompiledMessageKind::Icu { ast } => assert!(!ast.is_empty()),
    other => panic!("expected icu payload, got {other:?}"),
}
```

## Rust runtime formatting

For direct runtime formatting in Rust, use [`ferrocat::runtime`]. It exposes the
runtime compiler, compiled message/catalog types, and host hooks such as
`FormatHost`.

That keeps the crate root host-neutral for thin bindings while preserving the
richer Rust-native execution model for direct integrations.
