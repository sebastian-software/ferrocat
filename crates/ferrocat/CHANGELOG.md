# Changelog

## Unreleased

### ⚠ BREAKING CHANGES

- Compiling with `IcuSyntaxPolicy::RuntimeLiteralApostrophes` now
  canonicalizes both artifact messages and compiled IDs. Regenerate runtime
  artifacts and `CompiledCatalogIdIndex` values created with Ferrocat 3.1.0 or
  earlier; previously cached IDs do not match the canonicalized contract.
- Coverage, fuzzy-enabled audit, and current review targets now require
  review-aware normalized catalogs from `parse_catalog_for_review`.
- `CatalogMessageStatus::Fuzzy`, `CatalogLocaleCoverage::fuzzy()`,
  `CatalogAuditChecks::fuzzy_flags`, and `catalog.fuzzy_flag` are restored.

### Features

* **catalog:** add explicit PO/FCL conversion that preserves shared message metadata
* **po:** expose opt-in rename-only durability for catalog file updates
* **icu:** expose policy-aware runtime apostrophe canonicalization and compiled-key derivation

### Bug Fixes

- FCL catalog workflows now reject duplicate serialized `(id, ctxt)`
  identities before rendering, and file updates preserve an existing
  destination when validation fails.
- Active PO and FCL fuzzy entries no longer count as translated, including in
  the coverage rollups embedded in review reports.

## [3.2.2](https://github.com/sebastian-software/ferrocat/compare/ferrocat-v3.2.1...ferrocat-v3.2.2) (2026-07-30)


### Bug Fixes

* **deps:** merge pnpm 11.18.0 update ([1e720ba](https://github.com/sebastian-software/ferrocat/commit/1e720ba745ca42f44b56ee7be7168691569e0a9d))
* **deps:** merge React 19.2.8 update ([860555a](https://github.com/sebastian-software/ferrocat/commit/860555a3e6746b7808b4a552da04e26ed53de32f))
* **deps:** merge setup-node v7 update ([2429fb3](https://github.com/sebastian-software/ferrocat/commit/2429fb3805bf2f51448638e5197cdc13e7d5d980))
* **deps:** update pnpm to 11.18.0 ([b1f1b9d](https://github.com/sebastian-software/ferrocat/commit/b1f1b9d78784f5b9551abc74039c355960a19691))
* **deps:** update React to 19.2.8 ([6c6d496](https://github.com/sebastian-software/ferrocat/commit/6c6d4969ba9a14b1bdd9a78afcb0f022e04a8576))
* **deps:** update setup-node action to v7 ([9dc5715](https://github.com/sebastian-software/ferrocat/commit/9dc5715200179d09b859708f72a8db304f0ee417))

## [3.2.1](https://github.com/sebastian-software/ferrocat/compare/ferrocat-v3.2.0...ferrocat-v3.2.1) (2026-07-30)


### Bug Fixes

* **fcl:** validate identities at export boundary ([#285](https://github.com/sebastian-software/ferrocat/issues/285)) ([a04ef3f](https://github.com/sebastian-software/ferrocat/commit/a04ef3f61295ef077627fcdda740d4f075707f61))

## [3.2.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-v3.1.0...ferrocat-v3.2.0) (2026-07-30)


### Features

* **catalog:** add explicit PO/FCL conversion ([4779539](https://github.com/sebastian-software/ferrocat/commit/4779539b13d105894c3cf510566e8b8b0ea8780d))
* **icu:** canonicalize runtime apostrophe quoting ([c37bd11](https://github.com/sebastian-software/ferrocat/commit/c37bd11fb349ce8ece4f11f6ea25712ff4ae8f25))


### Bug Fixes

* **coverage:** classify fuzzy entries as incomplete ([7ad263b](https://github.com/sebastian-software/ferrocat/commit/7ad263b565f09d0cacea19cac1fefd494562e3f2))

## [3.1.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-v3.0.1...ferrocat-v3.1.0) (2026-07-30)


### Features

* **po:** carry opaque metadata through FCL tc/f tags ([faabfc4](https://github.com/sebastian-software/ferrocat/commit/faabfc44a19cca7ecf8172a978f9116beda74be4)), closes [#271](https://github.com/sebastian-software/ferrocat/issues/271)
* **po:** expose PO serialization options through catalog update and combine ([51eca62](https://github.com/sebastian-software/ferrocat/commit/51eca62c490cafe00b2c59b0b96a0bd01f37ee05))
* **po:** expose PO serialization options through catalog update and combine ([d0769df](https://github.com/sebastian-software/ferrocat/commit/d0769dfb102f6429ea72798f632eac256ef3e328)), closes [#272](https://github.com/sebastian-software/ferrocat/issues/272)
* **po:** preserve translator comments and flags through catalog updates ([d057d1d](https://github.com/sebastian-software/ferrocat/commit/d057d1d82d64cbf07ca752d18f623e4b28f23ddb)), closes [#271](https://github.com/sebastian-software/ferrocat/issues/271)
* **po:** preserve translator comments and opaque flags across catalog updates ([97bdb69](https://github.com/sebastian-software/ferrocat/commit/97bdb69215531ae083a733a2e6010d336aa395a7))


### Bug Fixes

* **deps:** update react-router monorepo to v8.3.0 ([9ed1a7d](https://github.com/sebastian-software/ferrocat/commit/9ed1a7d3661ed93b4a7fe71f9945907652e38032))
* **po:** correct ICU plural synthesis capacity estimate ([70ad183](https://github.com/sebastian-software/ferrocat/commit/70ad183458911f49299c64deff7c0224e2485574))
* **po:** derive combine metadata transfer from translation ownership ([b358744](https://github.com/sebastian-software/ferrocat/commit/b358744ee677dce910f270dddc00b33af0d6c2a7))


### Performance Improvements

* **bench:** ingest templates via the borrowed PO parser ([fc4a8fb](https://github.com/sebastian-software/ferrocat/commit/fc4a8fba6dfc721a6443a64159177ae4c945d437))
* **po:** allocation-focused catalog speedups and refreshed benchmark numbers ([4cd8796](https://github.com/sebastian-software/ferrocat/commit/4cd8796a48f899dc26eaaa2e321bee4cc717fa29))
* **po:** box the opaque metadata block ([6c6c280](https://github.com/sebastian-software/ferrocat/commit/6c6c28018200e7d2181396f99a347fce3915ce1b))
* **po:** continue collation prefixes on collision ([c3caa27](https://github.com/sebastian-software/ferrocat/commit/c3caa274664a992410d654fc0ac072c042af9e6d))
* **po:** cut allocation churn in plural synthesis and PO export ([454b7a8](https://github.com/sebastian-software/ferrocat/commit/454b7a80a9db646e089bb9a4f79875bae35246d0))
* **po:** cut per-message allocations in catalog export ([e6e14a0](https://github.com/sebastian-software/ferrocat/commit/e6e14a00c1c09290df9abee8abca4023212ab9e6))
* **po:** drop redundant buffers in catalog message import ([e933efb](https://github.com/sebastian-software/ferrocat/commit/e933efb8e36501c0b3bc8c02e390cfe835c61284))
* **po:** drop throwaway sets in catalog merge helpers ([f41268c](https://github.com/sebastian-software/ferrocat/commit/f41268c35869dbaee69ea867391b30d73c18de15))
* **po:** move each message once when applying collated order ([75f807d](https://github.com/sebastian-software/ferrocat/commit/75f807d79b66634e78f512774d9d683db5701bad))
* **po:** move merged message payloads instead of cloning ([658aed5](https://github.com/sebastian-software/ferrocat/commit/658aed534c359262a6600d40a073c4900f6bfb6f))
* **po:** parse catalogs through the borrowed PO parser ([f999064](https://github.com/sebastian-software/ferrocat/commit/f999064413ba6368bc6a9ed3ae9b1dcc9ceae0c2))
* **po:** skip opaque metadata capture on the public parse projection ([7b507d6](https://github.com/sebastian-software/ferrocat/commit/7b507d612d5762002731529756297fcbea44f190))

## [3.0.1](https://github.com/sebastian-software/ferrocat/compare/ferrocat-v2.2.0...ferrocat-v3.0.1) (2026-07-29)


### Features

* **bench:** cover catalog workflow regressions ([a4e3ab6](https://github.com/sebastian-software/ferrocat/commit/a4e3ab62aed6dd9d8d5ba81ed52f687c0af0e45c))
* **po:** add collated catalog ordering ([18fbd36](https://github.com/sebastian-software/ferrocat/commit/18fbd36a7ab277c55872cd90e875568740c35c52))


### Bug Fixes

* **bench:** use gettext mode for mixed fixtures ([28bc4e0](https://github.com/sebastian-software/ferrocat/commit/28bc4e0f96543af2ede6d7489ff80ce6d5c609e3))
* **ci:** restore blocking semver checks ([5edd119](https://github.com/sebastian-software/ferrocat/commit/5edd119605d2332243ba9f36bf4d94bd02c7b91d))
* **deps:** update dependency ardo to v3.8.1 ([#261](https://github.com/sebastian-software/ferrocat/issues/261)) ([70c4b8a](https://github.com/sebastian-software/ferrocat/commit/70c4b8aa9771aa8e422f09c0eb8d9ab5032f58d5))
* **deps:** update dependency gettext-parser to v9.1.1 ([f542ca1](https://github.com/sebastian-software/ferrocat/commit/f542ca1c3b772c88635da23c860b2fe1a04f71a1))
* **deps:** update dependency isbot to v5.2.1 ([e1244e2](https://github.com/sebastian-software/ferrocat/commit/e1244e20d2790b95423e1c615e291bc5f331d62e))
* **deps:** update dependency vite to v8.1.5 ([d9af196](https://github.com/sebastian-software/ferrocat/commit/d9af196ed07eb5a4e8e84048e7c53ceba8126623))
* **deps:** update formatjs monorepo to v3.5.15 ([cac79e7](https://github.com/sebastian-software/ferrocat/commit/cac79e777025a6c97ac26cf2aa99a142bc4d89a5))
* **deps:** update lucide monorepo to v1.27.0 ([3f9bdd3](https://github.com/sebastian-software/ferrocat/commit/3f9bdd3e3a5be0ad41254089b04924c21c7335e4))
* **deps:** update pnpm to v11.17.0 ([a36fbc1](https://github.com/sebastian-software/ferrocat/commit/a36fbc1f4597a959aecb57d23af77a7cc921cd98))
* **po:** preserve Intl accent ordering ([200cba0](https://github.com/sebastian-software/ferrocat/commit/200cba0ef75af85ca8cff35cb8e1deb3ed5e2bfb))


### Performance Improvements

* **audit:** reuse parsed ICU messages ([6664d6a](https://github.com/sebastian-software/ferrocat/commit/6664d6af17a66e6bff10787fca9f6915a89814a8))


### Miscellaneous Chores

* **release:** prepare 3.0.1 ([018ecfb](https://github.com/sebastian-software/ferrocat/commit/018ecfb3771b0bb3d936a317fdb7de03e63ddf84))
* **release:** prepare 3.0.1 ([#269](https://github.com/sebastian-software/ferrocat/issues/269)) ([c948b3c](https://github.com/sebastian-software/ferrocat/commit/c948b3c628eb65350da9b84f4d7e7f0a59eca6fd))

## [2.2.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-v3.0.0...ferrocat-v2.2.0) (2026-07-05)


### Miscellaneous Chores

* **ferrocat:** Synchronize ferrocat versions


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 3.0.0 to 2.2.0
    * ferrocat-po bumped from 3.0.0 to 2.2.0

## [3.0.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-v2.1.1...ferrocat-v3.0.0) (2026-07-05)


### Miscellaneous Chores

* **ferrocat:** Synchronize ferrocat versions


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 2.1.1 to 3.0.0
    * ferrocat-po bumped from 2.1.1 to 3.0.0

## [2.1.1](https://github.com/sebastian-software/ferrocat/compare/ferrocat-v2.1.0...ferrocat-v2.1.1) (2026-07-03)


### Bug Fixes

* **api:** address consolidation review feedback ([bc183e3](https://github.com/sebastian-software/ferrocat/commit/bc183e3379422b5193b32dc717919b5208679cc8))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 2.1.0 to 2.1.1
    * ferrocat-po bumped from 2.1.0 to 2.1.1

## [2.1.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-v2.0.0...ferrocat-v2.1.0) (2026-07-02)

### ⚠ BREAKING CLEANUPS

See the [2.0 to 2.1 migration guide](https://ferrocat.dev/guide/upgrading#upgrading-to-210) for old-to-new names and construction examples.

* **api:** `catalog_review` is now `review_catalogs`, `catalog_coverage` is now `measure_catalog_coverage`, and `has_selectordinal` is now `has_select_ordinal`.
* **po:** `MergeExtractedMessage` is removed; use `MergeMessageInput`.
* **po:** `MsgStr::first()` now returns `Option<&str>`, `MsgStr::first_str()` is removed, and `MsgStr` iterator items are `&str`.
* **po:** `PoVec` is an opaque newtype. Read access remains slice-like, but constructing affected fields from `Vec<T>` now needs `From<Vec<T>>` or `.into()`.
* **api:** public options structs are `#[non_exhaustive]`; downstream code should use `Options::new().with_*()` builders instead of functional-record-update syntax.

### Features

* **api:** add option builder setters ([#208](https://github.com/sebastian-software/ferrocat/issues/208)) ([8b4c279](https://github.com/sebastian-software/ferrocat/commit/8b4c2793a74871b435c74b405bb965ad88e6d4bd))
* **api:** align public cleanup names ([80924ea](https://github.com/sebastian-software/ferrocat/commit/80924eac840d9445f36bf5f5f23b7153e1a12eca))
* **api:** make options extensible ([#213](https://github.com/sebastian-software/ferrocat/issues/213)) ([b8c9a6a](https://github.com/sebastian-software/ferrocat/commit/b8c9a6adea3b6d063599964e6ae66e997458f0f1))
* **po:** make PoVec opaque ([#212](https://github.com/sebastian-software/ferrocat/issues/212)) ([2098808](https://github.com/sebastian-software/ferrocat/commit/20988081242d791723abe59337dbbc5e8bc729a9))


### Bug Fixes

* **api:** repair umbrella crate re-exports ([#198](https://github.com/sebastian-software/ferrocat/issues/198)) ([13a279a](https://github.com/sebastian-software/ferrocat/commit/13a279a460bf754d03541a541bd9c6e48715b067))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 2.0.0 to 2.1.0
    * ferrocat-po bumped from 2.0.0 to 2.1.0

## [2.0.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-v1.3.2...ferrocat-v2.0.0) (2026-06-30)


### ⚠ BREAKING CHANGES

* **po:** `CatalogMessage::obsolete` is now `Option<ObsoleteInfo>` instead of `bool`, `ObsoleteStrategy` gains a non-`Copy` `DropObsoleteBefore(String)` variant, and `UpdateCatalogOptions` gains a `now` field.
* **po:** `CatalogMessageExtra`, `CatalogMessage::extra`, `CatalogMessageStatus::Fuzzy`, `CatalogAuditChecks::fuzzy_flags`, the `catalog.fuzzy_flag` diagnostic, and `CatalogLocaleCoverage::fuzzy` are removed; `CatalogOrigin` gains a required `scope` field; the FCL `tc=` and `f=` tags are removed. PO/FCL output no longer carries `fuzzy`/format flags and renders origins as `file#scope`.
* **po:** `MachineTranslationMetadata` (with `model`/`modified`/ `confidence: u8`/`hash`) is replaced by `MachineMetadata { lock, ai }` + `AiProvenance`; `CatalogMessage::machine_translation` is renamed to `machine`; `confidence` is now a `[0,1]` f32. PO machine metadata is written as `#@ lock:`/`#@ ai:` instead of `#@ ferrocat-mt`, and FCL as `lock=`/`ai=`.
* **po:** `CatalogOrigin::line` and the `include_line_numbers` fields on `RenderOptions`, `CombineCatalogOptions`, and `CombineCatalogFilesOptions` are removed. Rendered references no longer include line numbers.
* **po:** the NDJSON catalog storage format and its public types (NdjsonCatalogReader/Writer + options, CatalogStorageFormat::Ndjson, CatalogFileFormat::Ndjson, CatalogMode::IcuNdjson) are removed. Use FCL (CatalogMode::IcuFcl, .fcl files) instead.
* **po:** `CatalogMessage::origin` is now `PoVec<CatalogOrigin>` (`SmallVec<[CatalogOrigin; 1]>`) instead of `Vec<CatalogOrigin>`. Reads are unaffected; constructing from a `Vec` now needs `.into()`.
* **po:** `BorrowedPoItem::{references, comments, extracted_comments, flags, metadata}` are now `PoVec<Cow<'_, str>>` (`SmallVec<[_; 1]>`) instead of `Vec<Cow<'_, str>>`.
* **po:** `PoItem::{references, comments, extracted_comments, flags, metadata}` are now `PoVec<T>` (`SmallVec<[T; 1]>`) instead of `Vec<T>`. Reads are unaffected; constructing from a `Vec` now needs `.into()`.

### Features

* **po:** obsolete age with clock-injected since and age-based cleanup ([3b9789e](https://github.com/sebastian-software/ferrocat/commit/3b9789ef89e6bbbc7d808cb8c13027294c8bee4b))
* **po:** remove NDJSON catalog format in favor of FCL ([9606441](https://github.com/sebastian-software/ferrocat/commit/96064410a154b3c05f92813d10156e4a2f454ed4))
* **po:** replace MT metadata with machine lock + AI provenance ([027440b](https://github.com/sebastian-software/ferrocat/commit/027440b25599209d159e366e186e63369fc1c002))
* **po:** trim entry metadata to origin scope, notes, and obsolete ([0dd85d4](https://github.com/sebastian-software/ferrocat/commit/0dd85d490fde02c4600b68de974fedc7c4226bd3))


### Bug Fixes

* **facade,docs:** re-export PoVec/SmallVec and fix the catalog API example ([abf009e](https://github.com/sebastian-software/ferrocat/commit/abf009eb685d250dc485f73d0a4b6ace3f32276a))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 1.3.2 to 2.0.0
    * ferrocat-po bumped from 1.3.2 to 2.0.0

## [1.3.2](https://github.com/sebastian-software/ferrocat/compare/ferrocat-v1.3.1...ferrocat-v1.3.2) (2026-06-29)


### Miscellaneous Chores

* **ferrocat:** Synchronize ferrocat versions


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 1.3.1 to 1.3.2
    * ferrocat-po bumped from 1.3.1 to 1.3.2

## [1.3.1](https://github.com/sebastian-software/ferrocat/compare/ferrocat-v1.3.0...ferrocat-v1.3.1) (2026-06-25)


### Miscellaneous Chores

* **ferrocat:** Synchronize ferrocat versions


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 1.3.0 to 1.3.1
    * ferrocat-po bumped from 1.3.0 to 1.3.1

## [1.3.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-v1.2.1...ferrocat-v1.3.0) (2026-06-23)


### Features

* **api:** add catalog file combine workflow ([ff3ab76](https://github.com/sebastian-software/ferrocat/commit/ff3ab765373aaf241bc076dd4a0c2096e5e37644))
* **api:** add catalog file combine workflow ([c26486e](https://github.com/sebastian-software/ferrocat/commit/c26486ecdd2bedb209ad13c7b373ffa3fb3660e6))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 1.2.1 to 1.3.0
    * ferrocat-po bumped from 1.2.1 to 1.3.0

## [1.2.1](https://github.com/sebastian-software/ferrocat/compare/ferrocat-v1.2.0...ferrocat-v1.2.1) (2026-06-23)


### Miscellaneous Chores

* **ferrocat:** Synchronize ferrocat versions


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 1.2.0 to 1.2.1
    * ferrocat-po bumped from 1.2.0 to 1.2.1

## [1.2.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-v1.1.1...ferrocat-v1.2.0) (2026-06-23)


### Features

* **icu:** add ICU-aware pseudolocalization ([7553319](https://github.com/sebastian-software/ferrocat/commit/75533192985d6d781cd3d7dfd045fdbf4a337fc9))
* **po:** add artifact provenance report API ([c17caf4](https://github.com/sebastian-software/ferrocat/commit/c17caf4f568be2431f6b543b67e88da19560a321))
* **po:** add catalog coverage report API ([3409b33](https://github.com/sebastian-software/ferrocat/commit/3409b33e94f82b4e49194d224c98e0f69c5aecba))
* **po:** add catalog review report API ([4d9d825](https://github.com/sebastian-software/ferrocat/commit/4d9d82593b64ec0a8f7759599afb2569704bf1e2))


### Bug Fixes

* **po:** honor artifact pseudolocalization syntax policy ([da8fb59](https://github.com/sebastian-software/ferrocat/commit/da8fb597b2b3a131b04a6881e112d8dfcfa82daf))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 1.1.1 to 1.2.0
    * ferrocat-po bumped from 1.1.1 to 1.2.0

## [1.1.1](https://github.com/sebastian-software/ferrocat/compare/ferrocat-v1.1.0...ferrocat-v1.1.1) (2026-06-19)


### Bug Fixes

* **release:** trigger dependency maintenance patch ([805109c](https://github.com/sebastian-software/ferrocat/commit/805109cb270c83ef030507fcf93c44e2e971969f))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 1.1.0 to 1.1.1
    * ferrocat-po bumped from 1.1.0 to 1.1.1

## [1.1.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-v1.0.0...ferrocat-v1.1.0) (2026-06-19)


### Features

* **api:** re-export ndjson streaming APIs ([#117](https://github.com/sebastian-software/ferrocat/issues/117)) ([15d3b9d](https://github.com/sebastian-software/ferrocat/commit/15d3b9dc4eb35289adac77b28d4f97216c72ad47))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 1.0.0 to 1.1.0
    * ferrocat-po bumped from 1.0.0 to 1.1.0

## [1.0.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-v0.13.0...ferrocat-v1.0.0) (2026-06-18)


### ⚠ BREAKING CHANGES

* **api:** collapse redundant option fields into CatalogMode ([#102](https://github.com/sebastian-software/ferrocat/issues/102))

### Features

* **api:** add serializable schema outputs ([#107](https://github.com/sebastian-software/ferrocat/issues/107)) ([8e8bff0](https://github.com/sebastian-software/ferrocat/commit/8e8bff054ac76c6ce0aa76d5d7a707c92f24d9d8))
* **api:** add umbrella namespace modules ([#104](https://github.com/sebastian-software/ferrocat/issues/104)) ([f72d1a5](https://github.com/sebastian-software/ferrocat/commit/f72d1a573cff29cb1bc955359c86531355610ca0))
* **api:** collapse redundant option fields into CatalogMode ([#102](https://github.com/sebastian-software/ferrocat/issues/102)) ([2687df6](https://github.com/sebastian-software/ferrocat/commit/2687df6363755b0ef863594168da8be027d34614))
* **features:** add lean parser profiles ([#106](https://github.com/sebastian-software/ferrocat/issues/106)) ([2887beb](https://github.com/sebastian-software/ferrocat/commit/2887bebfedabb7664f15c12912ea96626b0d103b))
* **icu:** expose stringify_icu ([#96](https://github.com/sebastian-software/ferrocat/issues/96)) ([a581dcb](https://github.com/sebastian-software/ferrocat/commit/a581dcb171a1f1321640f9f22f6da165d23c7f06))
* **icu:** validate formatter support from analysis ([#79](https://github.com/sebastian-software/ferrocat/issues/79)) ([7089df6](https://github.com/sebastian-software/ferrocat/commit/7089df67cdb4803fa72dd79e9f077565aefc91cc))


### Bug Fixes

* **api:** re-export CatalogSemantics from the umbrella crate ([a16e699](https://github.com/sebastian-software/ferrocat/commit/a16e6993b09e86d19712edc601b2fc0271ffcdaf)), closes [#47](https://github.com/sebastian-software/ferrocat/issues/47)


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 0.13.0 to 1.0.0
    * ferrocat-po bumped from 0.13.0 to 1.0.0

## [0.13.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-v0.12.0...ferrocat-v0.13.0) (2026-06-12)


### Features

* validate icu formatter runtime support ([#44](https://github.com/sebastian-software/ferrocat/issues/44)) ([1ede8b3](https://github.com/sebastian-software/ferrocat/commit/1ede8b376f693bdc3f0b3bb2ad70308d7b6a19bb))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 0.12.0 to 0.13.0
    * ferrocat-po bumped from 0.12.0 to 0.13.0

## [0.12.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-v0.11.0...ferrocat-v0.12.0) (2026-05-21)


### Features

* **po:** add machine translation metadata ([275c4b0](https://github.com/sebastian-software/ferrocat/commit/275c4b0fb2a598ce3f21e48929a01e5a7d68aecb))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 0.11.0 to 0.12.0
    * ferrocat-po bumped from 0.11.0 to 0.12.0

## [0.11.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-v0.10.0...ferrocat-v0.11.0) (2026-05-12)


### Features

* **icu:** add authoring diagnostics ([ea53674](https://github.com/sebastian-software/ferrocat/commit/ea5367412fcfcd636a9ae1b3e08a3a33ecae9f74))
* **icu:** add semantic message metadata ([1a0a7bc](https://github.com/sebastian-software/ferrocat/commit/1a0a7bcb477d8643d63175c4f4584911572fb934))
* **po:** add catalog audit reports ([e1b3591](https://github.com/sebastian-software/ferrocat/commit/e1b3591bbb5291539133d40d421bf5e5ceb84f0e))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 0.10.0 to 0.11.0
    * ferrocat-po bumped from 0.10.0 to 0.11.0

## [0.10.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-v0.9.0...ferrocat-v0.10.0) (2026-05-11)


### Features

* **po:** add catalog combine API ([761c291](https://github.com/sebastian-software/ferrocat/commit/761c29145b0aa20fc62b53f70d164dcb27abb027))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 0.9.0 to 0.10.0
    * ferrocat-po bumped from 0.9.0 to 0.10.0

## [0.9.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-v0.8.0...ferrocat-v0.9.0) (2026-03-19)


### Features

* **catalog:** add ndjson storage format ([f335df9](https://github.com/sebastian-software/ferrocat/commit/f335df94693c2cb59bf54d2a9543f89184bfa6c0))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 0.8.0 to 0.9.0
    * ferrocat-po bumped from 0.8.0 to 0.9.0

## [0.8.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-v0.7.0...ferrocat-v0.8.0) (2026-03-18)


### Features

* **catalog:** expose public compiled key helper ([5a3e2c8](https://github.com/sebastian-software/ferrocat/commit/5a3e2c8a9ad1e1d25eced87cffc0920dbef6d02a))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 0.7.0 to 0.8.0
    * ferrocat-po bumped from 0.7.0 to 0.8.0

## [0.7.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-v0.6.0...ferrocat-v0.7.0) (2026-03-17)


### Features

* **catalog:** add compiled catalog artifact API ([26486d2](https://github.com/sebastian-software/ferrocat/commit/26486d2d520523e335cb8a8796b57818b7b1bb99))
* **catalog:** add compiled id metadata helpers ([0a7cef0](https://github.com/sebastian-software/ferrocat/commit/0a7cef052cf918dfe362cd79575922121fce78fa))
* **catalog:** add selected-key artifact compilation ([30fd036](https://github.com/sebastian-software/ferrocat/commit/30fd036f05f433aa529f92c57941be7187608d76))
* **catalog:** add selected-key compiled catalog primitives ([355dd46](https://github.com/sebastian-software/ferrocat/commit/355dd46e7b05da81698de61c46dfc2a25bb2f394))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 0.6.0 to 0.7.0
    * ferrocat-po bumped from 0.6.0 to 0.7.0

## [0.6.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-v0.5.2...ferrocat-v0.6.0) (2026-03-17)


### Features

* add compiled catalog runtime API ([f59a3da](https://github.com/sebastian-software/ferrocat/commit/f59a3dacb6a94088cf8536f5053882b939af36a7))


### Bug Fixes

* harden Rust APIs and expand public docs ([3cec26c](https://github.com/sebastian-software/ferrocat/commit/3cec26c426766b77b544497500d4eaf2c5815e0c))
* **rust:** tighten public API docs and idioms ([dcffdd1](https://github.com/sebastian-software/ferrocat/commit/dcffdd1436e5d0060e1671017660a18c6a204aa0))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 0.5.2 to 0.6.0
    * ferrocat-po bumped from 0.5.2 to 0.6.0

## [0.5.2](https://github.com/sebastian-software/ferrocat/compare/ferrocat-v0.5.1...ferrocat-v0.5.2) (2026-03-17)


### Bug Fixes

* trigger build ([682508b](https://github.com/sebastian-software/ferrocat/commit/682508b0cabf1f31ddbcfe6d2c76687600531eb4))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 0.5.1 to 0.5.2
    * ferrocat-po bumped from 0.5.1 to 0.5.2

## [0.5.1](https://github.com/sebastian-software/ferrocat/compare/ferrocat-v0.5.0...ferrocat-v0.5.1) (2026-03-17)


### Bug Fixes

* trigger build ([fc674b8](https://github.com/sebastian-software/ferrocat/commit/fc674b859b4483459892279a9ebc8aa191ab4da4))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 0.5.0 to 0.5.1
    * ferrocat-po bumped from 0.5.0 to 0.5.1

## [0.5.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-v0.4.0...ferrocat-v0.5.0) (2026-03-17)


### Features

* add source-first catalog input and normalized view ([4b1272c](https://github.com/sebastian-software/ferrocat/commit/4b1272ceeacd718445c0d60eff490f780740f37e))
* add source-first catalog input and normalized view ([1a0d295](https://github.com/sebastian-software/ferrocat/commit/1a0d295971bec7524b9e6113f3b2c40b5df2ce18))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 0.4.0 to 0.5.0
    * ferrocat-po bumped from 0.4.0 to 0.5.0

## [0.4.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-v0.3.0...ferrocat-v0.4.0) (2026-03-17)


### Features

* **ferrocat:** migrate workspace from ferrox ([fa6bf5b](https://github.com/sebastian-software/ferrocat/commit/fa6bf5bcbc7f1552f43596ae941b3483916cab3a))


### Bug Fixes

* **release:** align versions for release please ([96c0729](https://github.com/sebastian-software/ferrocat/commit/96c072927ca1bbcef0a66b0f74d4759645ca1d51))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 0.3.0 to 0.4.0
    * ferrocat-po bumped from 0.3.0 to 0.4.0

## [0.3.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-v0.2.0...ferrocat-v0.3.0) (2026-03-17)


### Features

* **ferrocat:** migrate workspace from ferrox ([fa6bf5b](https://github.com/sebastian-software/ferrocat/commit/fa6bf5bcbc7f1552f43596ae941b3483916cab3a))


### Bug Fixes

* **release:** align versions for release please ([96c0729](https://github.com/sebastian-software/ferrocat/commit/96c072927ca1bbcef0a66b0f74d4759645ca1d51))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 0.2.0 to 0.3.0
    * ferrocat-po bumped from 0.2.0 to 0.3.0
