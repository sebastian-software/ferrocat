# API Overview

`ferrocat` currently exposes three practical layers of API:

- low-level PO parsing and writing
- catalog-level gettext workflows
- ICU MessageFormat parsing

This page is a lightweight guide for choosing the right function before there is a fuller generated API reference.

## Quick Choice

| If you want to... | Use |
|---|---|
| Parse a `.po` file into an owned Rust structure | `parse_po` |
| Parse a `.po` file while borrowing from the input string where possible | `parse_po_borrowed` |
| Turn a `PoFile` back into `.po` text | `stringify_po` |
| Merge fresh extracted gettext messages into an existing `.po` file | `merge_catalog` |
| Read a `.po` file into the higher-level canonical catalog model | `parse_catalog` |
| Build keyed lookup/helpers on top of a parsed catalog | `ParsedCatalog::into_normalized_view` |
| Derive the default stable runtime key from `msgid` and `msgctxt` | `compiled_key` |
| Compile a normalized catalog into runtime lookup entries | `NormalizedParsedCatalog::compile` |
| Compile a requested-locale runtime artifact with fallbacks and missing reports | `compile_catalog_artifact` |
| Compile only a selected subset of compiled runtime IDs | `compile_catalog_artifact_selected` |
| Perform a full in-memory catalog update | `update_catalog` |
| Perform a full catalog update and write the result to disk only when changed | `update_catalog_file` |
| Parse ICU MessageFormat into a structural AST | `parse_icu` |
| Only validate ICU syntax | `validate_icu` |
| Extract variable names from a parsed ICU message | `extract_variables` |

## PO Core

### `parse_po`

Use this when you want the normal, owned Rust representation of a PO file.

Typical use cases:

- application code that wants a straightforward editable `PoFile`
- transforms that keep parsed data around beyond the source input lifetime
- tools where simplicity matters more than minimizing allocations

### `parse_po_borrowed`

Use this when you want to parse without copying more than necessary.

Typical use cases:

- read-heavy workflows
- performance-sensitive inspection or transformation passes
- benchmarks or pipelines where borrowing from the source text is helpful

### `stringify_po`

Use this when you already have a `PoFile` and want canonical PO output.

Typical use cases:

- writing back modified parsed files
- generating PO content from your own tooling
- normalizing formatting after edits

## Catalog Workflows

The high-level catalog request structs are now intentionally borrowing-first:

- string inputs such as catalog text and locales are accepted as `&str`
- selected compiled IDs and fallback chains are accepted as borrowed slices
- file-oriented updates accept `&Path`

That keeps the API ergonomic for callers while avoiding avoidable request-side allocation and clone pressure before the real catalog work even starts.

### `merge_catalog`

Use this for the basic gettext merge step:

- start from an existing `.po`
- feed in freshly extracted messages
- keep matching translations
- add new entries
- mark removed entries as obsolete

This is the closest `ferrocat` equivalent to the core "merge updated template/messages into an existing catalog" workflow that users often associate with GNU `msgmerge`.

Choose `merge_catalog` when you want the leaner, more direct merge operation and already have data in classic gettext-like shapes.

In practice this is the "fast path" workflow API: it stays close to classic PO merge behavior and avoids the extra canonical catalog projection and post-processing done by `update_catalog`.

### `parse_catalog`

Use this when you want more than raw PO syntax. It projects a PO file into `ferrocat`'s higher-level catalog model, including plural handling, diagnostics, and optional ICU-aware interpretation.

Choose this when your application wants semantic catalog data rather than just PO syntax nodes.

`ParseCatalogOptions` borrows the source text and locale strings, so you can parse directly from existing buffers without first building owned request strings.

`parse_catalog` intentionally stays as the neutral parse step. If you want keyed lookups or effective-translation helpers, build a richer view explicitly with `ParsedCatalog::into_normalized_view()`.

### `NormalizedParsedCatalog::compile`

Use this when you want a runtime-facing lookup structure with stable compiled keys rather than raw gettext identities.

This sits one layer above parsed catalog lookup:

- start with `parse_catalog`
- build the normalized keyed view
- compile to `CompiledCatalog` for runtime-oriented consumption

The default behavior keeps translations as they exist in the catalog. Optional source-locale fallback is explicit rather than automatic.

The built-in `CompiledKeyStrategy::FerrocatV1` contract is intentionally compact:

- SHA-256 over a versioned source-identity payload
- truncated to 64 bits
- encoded as unpadded Base64URL
- no visible version prefix in the emitted key
- hard compile failure on collisions

### `compiled_key`

Use this when a host adapter, source transform, or manifest builder needs the
same default runtime key that Ferrocat emits during catalog compilation, but
only has `msgid` and optional `msgctxt` available.

This is the public, host-facing helper for the current default key contract.
It corresponds to `CompiledKeyStrategy::FerrocatV1`.

### `compile_catalog_artifact`

Use this when you want the final host-neutral runtime artifact for one requested locale rather than one catalog's typed lookup payload.

This sits one step above `NormalizedParsedCatalog::compile`:

- start from one or more normalized catalogs
- choose a requested locale and source locale
- optionally provide a fallback chain
- compile a final `key -> ICU string` runtime map
- collect missing-message records for non-source locales
- validate the final runtime strings as ICU messages

Choose this when your downstream tooling needs locale resolution semantics centralized in Ferrocat instead of rebuilding them in a host adapter.

`CompileCatalogArtifactOptions` borrows locale strings and the fallback-chain slice, which keeps host-side request assembly cheap even when compilation is performance-sensitive.

Important semantics:

- only non-obsolete messages participate in artifact compilation
- empty non-source translations are treated as unresolved and can fall through to the fallback chain
- source fallback is explicit for non-source locale compilation
- source-locale compilation always materializes empty source values from source text
- plural messages are emitted as final ICU plural strings using the preserved plural variable
- invalid final ICU strings become diagnostics by default and can become hard errors in strict mode

### `compile_catalog_artifact_selected`

Use this when a host adapter already knows the exact compiled runtime IDs it needs and wants only that slice of a requested-locale artifact.

This is the narrower companion to `compile_catalog_artifact`:

- build or reuse a `CompiledCatalogIdIndex`
- pass only the selected compiled IDs
- keep the same fallback, missing, and ICU-validation semantics
- return the same `CompiledCatalogArtifact` shape, but filtered to the requested subset

Choose this when a bundler/plugin layer has already mapped modules or chunks to the exact message IDs they require.

Like the broader artifact API, the request struct borrows locale data and selection slices, so callers can reuse existing vectors or arrays of compiled IDs without another owned wrapper.

### `CompiledCatalogIdIndex`

Use this when you need stable compiled-ID metadata without compiling message payloads immediately.

Useful helpers now include:

- `iter` for deterministic compiled-ID traversal
- `as_btreemap` / `into_btreemap` when another tool wants the raw ordered mapping
- `describe_compiled_ids` to ask which requested IDs are known, available in a given catalog set, and whether they are singular or plural

`describe_compiled_ids` returns a structured report:

- `described` for IDs that are known to the index and present in the provided catalog set
- `unknown_compiled_ids` for IDs that do not exist in the index at all
- `unavailable_compiled_ids` for IDs that are known to the index but not present in the provided catalog set

### `update_catalog`

Use this for the full high-level catalog update path in memory.

This goes beyond a raw merge. It can:

- parse an existing catalog into the canonical model
- merge extracted messages from either structured catalog input (`CatalogUpdateInput::Structured`) or source-first messages (`CatalogUpdateInput::SourceFirst`)
- handle locale/plural logic
- apply header defaults
- preserve or report diagnostics
- sort and export the final PO file

Choose `update_catalog` when you want a complete update operation rather than just the lower-level merge step.

Compared with `merge_catalog`, this is the "full semantics" path. It is the better fit when catalog correctness and consistency matter more than taking the shortest merge route, for example in release pipelines or when you want predictable headers, ordering, plural handling, and diagnostics.

`UpdateCatalogOptions` borrows locale strings, optional existing content, and optional custom-header maps. The extracted message payload itself stays owned, because that is usually the natural shape for upstream extractor output.

### `update_catalog_file`

Use this when you want the same high-level behavior as `update_catalog`, but against a file path.

It reads the current file if present, runs the full update, and only writes back when the result actually changed.

Choose this for CLI tools, task runners, or build/dev pipelines that work directly on catalog files on disk.

Like `update_catalog`, it accepts `CatalogUpdateInput`, so source-string-first tooling can hand off plural projection and catalog-shaping to Ferrocat instead of pre-projecting everything into `ExtractedMessage`.

`UpdateCatalogFileOptions` borrows both the path and the locale/header inputs, so file-based automation can call it without constructing throwaway owned request objects.

## ICU MessageFormat

### `parse_icu`

Use this when you need the parsed ICU AST.

Typical use cases:

- inspecting plural/select structure
- converting ICU messages into another internal representation
- extracting semantic information from messages

### `validate_icu`

Use this when you only need a yes/no syntax check with an error surface.

### `extract_variables`

Use this after `parse_icu` when you want the variable names referenced by the message.

## Practical Rule Of Thumb

- editing raw PO files: `parse_po` + `stringify_po`
- hot-path PO inspection: `parse_po_borrowed`
- classic gettext merge step: `merge_catalog`
- full app-level catalog maintenance: `update_catalog` or `update_catalog_file`
- parsed catalog consumption with keyed accessors: `parse_catalog` + `into_normalized_view`
- locale-specific runtime artifact generation: `compile_catalog_artifact`
- selected locale artifact generation by compiled ID: `compile_catalog_artifact_selected`
- ICU analysis: `parse_icu`
