# Changelog

## [2.1.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-po-v2.0.0...ferrocat-po-v2.1.0) (2026-07-02)

### ⚠ BREAKING CLEANUPS

See the [2.0 to 2.1 migration guide](https://ferrocat.dev/guide/upgrading#upgrading-to-210) for old-to-new names and construction examples.

* **api:** `catalog_review` is now `review_catalogs`, and `catalog_coverage` is now `measure_catalog_coverage`.
* **po:** `MergeExtractedMessage` is removed; use `MergeMessageInput`.
* **po:** `MsgStr::first()` now returns `Option<&str>`, `MsgStr::first_str()` is removed, and `MsgStr` iterator items are `&str`.
* **po:** `PoVec` is an opaque newtype. Read access remains slice-like, but constructing affected fields from `Vec<T>` now needs `From<Vec<T>>` or `.into()`.
* **api:** public options structs are `#[non_exhaustive]`; downstream code should use `Options::new().with_*()` builders instead of functional-record-update syntax.

### Features

* **api:** add ergonomic message helpers ([214130a](https://github.com/sebastian-software/ferrocat/commit/214130ab1c4de50dec0a6f15b2c85b6b68d87068))
* **api:** add option builder setters ([#208](https://github.com/sebastian-software/ferrocat/issues/208)) ([8b4c279](https://github.com/sebastian-software/ferrocat/commit/8b4c2793a74871b435c74b405bb965ad88e6d4bd))
* **api:** align public cleanup names ([80924ea](https://github.com/sebastian-software/ferrocat/commit/80924eac840d9445f36bf5f5f23b7153e1a12eca))
* **api:** make options extensible ([#213](https://github.com/sebastian-software/ferrocat/issues/213)) ([b8c9a6a](https://github.com/sebastian-software/ferrocat/commit/b8c9a6adea3b6d063599964e6ae66e997458f0f1))
* **po:** make PoVec opaque ([#212](https://github.com/sebastian-software/ferrocat/issues/212)) ([2098808](https://github.com/sebastian-software/ferrocat/commit/20988081242d791723abe59337dbbc5e8bc729a9))


### Bug Fixes

* **api:** infer gettext template suffixes ([75505e9](https://github.com/sebastian-software/ferrocat/commit/75505e92209d7c792936161761fe00fab42f1eba))


### Performance Improvements

* **po:** avoid catalog sort and dedupe allocations ([#201](https://github.com/sebastian-software/ferrocat/issues/201)) ([524dfea](https://github.com/sebastian-software/ferrocat/commit/524dfea5afd38b331f38105a58752678ae063bcf))
* **po:** avoid fcl update allocation leftovers ([#211](https://github.com/sebastian-software/ferrocat/issues/211)) ([72bad7d](https://github.com/sebastian-software/ferrocat/commit/72bad7d953bebf12797357bfb93491eb87658102))
* **po:** avoid helper and combine clone churn ([#204](https://github.com/sebastian-software/ferrocat/issues/204)) ([5d451f2](https://github.com/sebastian-software/ferrocat/commit/5d451f2cf8ca642625f70d5b7e4f0db883fa768e))
* **po:** compact normalized catalog lookups ([#210](https://github.com/sebastian-software/ferrocat/issues/210)) ([3b348ab](https://github.com/sebastian-software/ferrocat/commit/3b348abc743e9717abe9f4621d5397427b27c110))
* **po:** move catalog update inputs through merge ([#202](https://github.com/sebastian-software/ferrocat/issues/202)) ([f2053e0](https://github.com/sebastian-software/ferrocat/commit/f2053e0f6362f2e734204db591949cc4e4c2e127))
* **po:** reduce parser and serializer allocations ([#197](https://github.com/sebastian-software/ferrocat/issues/197)) ([20ca38e](https://github.com/sebastian-software/ferrocat/commit/20ca38ea8fcbe90578badc8a455c828556448cd8))
* **po:** skip provenance allocations for plain artifacts ([#205](https://github.com/sebastian-software/ferrocat/issues/205)) ([9aa4a95](https://github.com/sebastian-software/ferrocat/commit/9aa4a95791a6cf769cb34e526d9c08c4e2be718a))
* **po:** write catalog PO output directly ([#209](https://github.com/sebastian-software/ferrocat/issues/209)) ([344cdc6](https://github.com/sebastian-software/ferrocat/commit/344cdc6dca65d93cea6fbe36894dc522f11e3ac0))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 2.0.0 to 2.1.0

## [2.0.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-po-v1.3.2...ferrocat-po-v2.0.0) (2026-06-30)


### ⚠ BREAKING CHANGES

* **po:** `CatalogMessage::obsolete` is now `Option<ObsoleteInfo>` instead of `bool`, `ObsoleteStrategy` gains a non-`Copy` `DropObsoleteBefore(String)` variant, and `UpdateCatalogOptions` gains a `now` field.
* **po:** `CatalogMessageExtra`, `CatalogMessage::extra`, `CatalogMessageStatus::Fuzzy`, `CatalogAuditChecks::fuzzy_flags`, the `catalog.fuzzy_flag` diagnostic, and `CatalogLocaleCoverage::fuzzy` are removed; `CatalogOrigin` gains a required `scope` field; the FCL `tc=` and `f=` tags are removed. PO/FCL output no longer carries `fuzzy`/format flags and renders origins as `file#scope`.
* **po:** `MachineTranslationMetadata` (with `model`/`modified`/ `confidence: u8`/`hash`) is replaced by `MachineMetadata { lock, ai }` + `AiProvenance`; `CatalogMessage::machine_translation` is renamed to `machine`; `confidence` is now a `[0,1]` f32. PO machine metadata is written as `#@ lock:`/`#@ ai:` instead of `#@ ferrocat-mt`, and FCL as `lock=`/`ai=`.
* **po:** `CatalogOrigin::line` and the `include_line_numbers` fields on `RenderOptions`, `CombineCatalogOptions`, and `CombineCatalogFilesOptions` are removed. Rendered references no longer include line numbers.
* **po:** the NDJSON catalog storage format and its public types (NdjsonCatalogReader/Writer + options, CatalogStorageFormat::Ndjson, CatalogFileFormat::Ndjson, CatalogMode::IcuNdjson) are removed. Use FCL (CatalogMode::IcuFcl, .fcl files) instead.
* **po:** `CatalogMessage::origin` is now `PoVec<CatalogOrigin>` (`SmallVec<[CatalogOrigin; 1]>`) instead of `Vec<CatalogOrigin>`. Reads are unaffected (Deref to slice, iteration, indexing, serde); constructing it from a `Vec` needs `.into()` and direct `PartialEq` against `Vec<_>` needs `.as_slice()`.
* **po:** `BorrowedPoItem::{references, comments, extracted_comments, flags, metadata}` are now `PoVec<Cow<'_, str>>` (`SmallVec<[_; 1]>`) instead of `Vec<Cow<'_, str>>`, mirroring the owned `PoItem` change.
* **po:** `PoItem::references`, `comments`, `extracted_comments`, `flags`, and `metadata` are now `PoVec<T>` (`SmallVec<[T; 1]>`) instead of `Vec<T>`. Read-heavy code is unaffected (Deref to slice, iteration, indexing, serde), but constructing a field from a `Vec` now needs `.into()` and direct `PartialEq` against `Vec<_>` needs `.as_slice()` on both sides.

### Features

* **po:** add FCL (Ferrocat Catalog Lines) line-oriented catalog format ([a35ccc8](https://github.com/sebastian-software/ferrocat/commit/a35ccc803b041f926f2f9000371cbc7ff6451215))
* **po:** drop source line numbers from the catalog layer ([45beaa2](https://github.com/sebastian-software/ferrocat/commit/45beaa2ac313568f0d90f696f1259789bab4803a))
* **po:** obsolete age with clock-injected since and age-based cleanup ([3b9789e](https://github.com/sebastian-software/ferrocat/commit/3b9789ef89e6bbbc7d808cb8c13027294c8bee4b))
* **po:** remove NDJSON catalog format in favor of FCL ([9606441](https://github.com/sebastian-software/ferrocat/commit/96064410a154b3c05f92813d10156e4a2f454ed4))
* **po:** replace MT metadata with machine lock + AI provenance ([027440b](https://github.com/sebastian-software/ferrocat/commit/027440b25599209d159e366e186e63369fc1c002))
* **po:** store borrowed per-item collections inline with SmallVec ([c34cd64](https://github.com/sebastian-software/ferrocat/commit/c34cd64598cd9dc021276ca08919cd3550606823))
* **po:** store catalog message origins inline with SmallVec ([3cd4597](https://github.com/sebastian-software/ferrocat/commit/3cd4597e257c3a4c8b28e7733743352198a82881))
* **po:** store per-item PO collections inline with SmallVec ([db808e3](https://github.com/sebastian-software/ferrocat/commit/db808e3319a08d8e1fe7a810a25b0a848966055f))
* **po:** trim entry metadata to origin scope, notes, and obsolete ([0dd85d4](https://github.com/sebastian-software/ferrocat/commit/0dd85d490fde02c4600b68de974fedc7c4226bd3))


### Bug Fixes

* **po:** keep CatalogMode discriminants stable; cover FCL codec paths ([4f9f4b6](https://github.com/sebastian-software/ferrocat/commit/4f9f4b6c1f544bf92c15bc41012defc2e3fec3cd))
* **po:** respect FCL render options ([8dcc1ed](https://github.com/sebastian-software/ferrocat/commit/8dcc1ed830387eae97553b100f7aa7af8bacb777))


### Performance Improvements

* **po:** byte-oriented FCL codec (memchr field split + escape scan) ([18be854](https://github.com/sebastian-software/ferrocat/commit/18be854ed330aa1fa71f8c4a665325bfd183b2cb))
* **po:** parse FCL directly into CanonicalMessage; add fcl benchmark ([0f6c71d](https://github.com/sebastian-software/ferrocat/commit/0f6c71d511a310e61d72be27f2f24c91617ff8b8))
* **po:** reserve FCL buffers, skip mt.conf alloc; add FCL bench gates ([e847e4c](https://github.com/sebastian-software/ferrocat/commit/e847e4c7e04c4fbb4d6295c1cd5947ea1bda1cdf))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 1.3.2 to 2.0.0

## [1.3.2](https://github.com/sebastian-software/ferrocat/compare/ferrocat-po-v1.3.1...ferrocat-po-v1.3.2) (2026-06-29)


### Performance Improvements

* **po:** add single-token fast path to split_reference_comment ([76455f1](https://github.com/sebastian-software/ferrocat/commit/76455f1aa7ec1f97206ed025fdfee572362deabb))
* **po:** cut redundant allocations in catalog import ([b9596d1](https://github.com/sebastian-software/ferrocat/commit/b9596d1d8a67e39ce0d41c8b08a4520aa771f6f0))
* **po:** drop throwaway Vec when parsing reference comments ([b6750ec](https://github.com/sebastian-software/ferrocat/commit/b6750ecbc44d4ad1bf871e43094ddc4991c08394))
* **po:** fold backslash lookup into quoted-content validation ([2a1ce74](https://github.com/sebastian-software/ferrocat/commit/2a1ce745b4042ba481753be5b0599d78aec4fd8f))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 1.3.1 to 1.3.2

## [1.3.1](https://github.com/sebastian-software/ferrocat/compare/ferrocat-po-v1.3.0...ferrocat-po-v1.3.1) (2026-06-25)


### Performance Improvements

* **api:** borrow matched message instead of cloning during merge ([7f4e13b](https://github.com/sebastian-software/ferrocat/commit/7f4e13ba785a0b22638d89a9ca4bf92691f06425))
* **api:** build plural profile once per catalog merge ([7b04e16](https://github.com/sebastian-software/ferrocat/commit/7b04e16e6e617ca63bcae2b30e1a1fc9064f8bee))
* **api:** reuse line buffer in NDJSON reader ([81c0c9b](https://github.com/sebastian-software/ferrocat/commit/81c0c9b518d5c4993c0299977af9c399b99ac44c))
* parser/merge optimizations and cross-runtime benchmark comparisons ([3c635e5](https://github.com/sebastian-software/ferrocat/commit/3c635e55eb3e29f957e99556acaf0ad14d00b819))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 1.3.0 to 1.3.1

## [1.3.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-po-v1.2.1...ferrocat-po-v1.3.0) (2026-06-23)


### Features

* **api:** add catalog file combine workflow ([ff3ab76](https://github.com/sebastian-software/ferrocat/commit/ff3ab765373aaf241bc076dd4a0c2096e5e37644))
* **api:** add catalog file combine workflow ([c26486e](https://github.com/sebastian-software/ferrocat/commit/c26486ecdd2bedb209ad13c7b373ffa3fb3660e6))


### Bug Fixes

* **api:** keep non-empty translations during combine ([a21191a](https://github.com/sebastian-software/ferrocat/commit/a21191a8903e5c839de9b29352b746f61ceddeda))


### Performance Improvements

* **api:** stream catalog file combine inputs ([a952e55](https://github.com/sebastian-software/ferrocat/commit/a952e5560048ef9ecfcb403c3d831914e6969d64))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 1.2.1 to 1.3.0

## [1.2.1](https://github.com/sebastian-software/ferrocat/compare/ferrocat-po-v1.2.0...ferrocat-po-v1.2.1) (2026-06-23)


### Miscellaneous Chores

* **ferrocat-po:** Synchronize ferrocat versions


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 1.2.0 to 1.2.1

## [1.2.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-po-v1.1.1...ferrocat-po-v1.2.0) (2026-06-23)


### Features

* **api:** add artifact formatter support diagnostics ([754e053](https://github.com/sebastian-software/ferrocat/commit/754e053443605ec023abd5dfdf8d1375e7b82a6c))
* **api:** add runtime ICU syntax policy ([2e1fc69](https://github.com/sebastian-software/ferrocat/commit/2e1fc692fe96cce2a32e9dfebe8f01d413d5d202))
* **icu:** add ICU-aware pseudolocalization ([7553319](https://github.com/sebastian-software/ferrocat/commit/75533192985d6d781cd3d7dfd045fdbf4a337fc9))
* **po:** add artifact provenance report API ([c17caf4](https://github.com/sebastian-software/ferrocat/commit/c17caf4f568be2431f6b543b67e88da19560a321))
* **po:** add catalog coverage report API ([3409b33](https://github.com/sebastian-software/ferrocat/commit/3409b33e94f82b4e49194d224c98e0f69c5aecba))
* **po:** add catalog review report API ([4d9d825](https://github.com/sebastian-software/ferrocat/commit/4d9d82593b64ec0a8f7759599afb2569704bf1e2))


### Bug Fixes

* **api:** remove formatter ICU option equality ([13a5ee9](https://github.com/sebastian-software/ferrocat/commit/13a5ee9641c228eaf63aff4c7bc93114b94bbb97))
* **po:** address catalog review report feedback ([638bd2b](https://github.com/sebastian-software/ferrocat/commit/638bd2b708c81316228c8fef4f0f5ceae14983bc))
* **po:** honor artifact pseudolocalization syntax policy ([da8fb59](https://github.com/sebastian-software/ferrocat/commit/da8fb597b2b3a131b04a6881e112d8dfcfa82daf))
* **po:** simplify provenance report rows ([871482b](https://github.com/sebastian-software/ferrocat/commit/871482b0573812b85d8dc8f5fa0d7e052cbeac31))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 1.1.1 to 1.2.0

## [1.1.1](https://github.com/sebastian-software/ferrocat/compare/ferrocat-po-v1.1.0...ferrocat-po-v1.1.1) (2026-06-19)


### Miscellaneous Chores

* **ferrocat-po:** Synchronize ferrocat versions


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 1.1.0 to 1.1.1

## [1.1.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-po-v1.0.0...ferrocat-po-v1.1.0) (2026-06-19)


### Features

* **po:** add bytes parser charset guard ([#115](https://github.com/sebastian-software/ferrocat/issues/115)) ([e9873a0](https://github.com/sebastian-software/ferrocat/commit/e9873a096f609d0658dd31363fe7de645d97fdeb))


### Bug Fixes

* **po:** align parser line ending handling ([#123](https://github.com/sebastian-software/ferrocat/issues/123)) ([f9007b8](https://github.com/sebastian-software/ferrocat/commit/f9007b8de2a914d246aef388855a90d71575cfb4))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 1.0.0 to 1.1.0

## [1.0.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-po-v0.13.0...ferrocat-po-v1.0.0) (2026-06-18)


### ⚠ BREAKING CHANGES

* **api:** collapse redundant option fields into CatalogMode ([#102](https://github.com/sebastian-software/ferrocat/issues/102))

### Features

* **api:** add diagnostic codes and io path context ([#103](https://github.com/sebastian-software/ferrocat/issues/103)) ([0a97a9e](https://github.com/sebastian-software/ferrocat/commit/0a97a9e1495f4a9dbec16e6261e7523982deb911))
* **api:** add serializable schema outputs ([#107](https://github.com/sebastian-software/ferrocat/issues/107)) ([8e8bff0](https://github.com/sebastian-software/ferrocat/commit/8e8bff054ac76c6ce0aa76d5d7a707c92f24d9d8))
* **api:** collapse redundant option fields into CatalogMode ([#102](https://github.com/sebastian-software/ferrocat/issues/102)) ([2687df6](https://github.com/sebastian-software/ferrocat/commit/2687df6363755b0ef863594168da8be027d34614))
* **api:** mark growth-prone enums non-exhaustive ([#101](https://github.com/sebastian-software/ferrocat/issues/101)) ([b4e1ca4](https://github.com/sebastian-software/ferrocat/commit/b4e1ca4262345f6e8927582ae477f104075b1474))
* **features:** add lean parser profiles ([#106](https://github.com/sebastian-software/ferrocat/issues/106)) ([2887beb](https://github.com/sebastian-software/ferrocat/commit/2887bebfedabb7664f15c12912ea96626b0d103b))
* **ndjson:** add streaming catalog reader writer ([#112](https://github.com/sebastian-software/ferrocat/issues/112)) ([de69e0f](https://github.com/sebastian-software/ferrocat/commit/de69e0f6d8e18fcef457a561c2aef7afbea69a39))
* **po:** add safe gettext plural forms table ([#105](https://github.com/sebastian-software/ferrocat/issues/105)) ([c20b61b](https://github.com/sebastian-software/ferrocat/commit/c20b61bbfddba0b9e20929907e6b28f32bef0d1d))


### Bug Fixes

* **api:** preserve parse and io error sources ([#95](https://github.com/sebastian-software/ferrocat/issues/95)) ([9173e0e](https://github.com/sebastian-software/ferrocat/commit/9173e0e64041277df0c19bb5c5396a568958eea1))
* **po:** reject unrecognized parser lines ([#110](https://github.com/sebastian-software/ferrocat/issues/110)) ([40044e6](https://github.com/sebastian-software/ferrocat/commit/40044e6e7c29df1c77b77ac5856db234f122765b))
* **po:** use durable unique atomic writes ([#80](https://github.com/sebastian-software/ferrocat/issues/80)) ([34a80d5](https://github.com/sebastian-software/ferrocat/commit/34a80d5cb0d21d7b064d848fbcc6520651739d71))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 0.13.0 to 1.0.0

## [0.13.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-po-v0.12.0...ferrocat-po-v0.13.0) (2026-06-12)


### Miscellaneous Chores

* **ferrocat-po:** Synchronize ferrocat versions


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 0.12.0 to 0.13.0

## [0.12.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-po-v0.11.0...ferrocat-po-v0.12.0) (2026-05-21)


### Features

* **po:** add machine translation metadata ([275c4b0](https://github.com/sebastian-software/ferrocat/commit/275c4b0fb2a598ce3f21e48929a01e5a7d68aecb))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 0.11.0 to 0.12.0

## [0.11.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-po-v0.10.0...ferrocat-po-v0.11.0) (2026-05-12)


### Features

* **icu:** add authoring diagnostics ([ea53674](https://github.com/sebastian-software/ferrocat/commit/ea5367412fcfcd636a9ae1b3e08a3a33ecae9f74))
* **po:** add catalog audit reports ([e1b3591](https://github.com/sebastian-software/ferrocat/commit/e1b3591bbb5291539133d40d421bf5e5ceb84f0e))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 0.10.0 to 0.11.0

## [0.10.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-po-v0.9.0...ferrocat-po-v0.10.0) (2026-05-11)


### Features

* **api:** improve catalog ergonomics and release checks ([210c240](https://github.com/sebastian-software/ferrocat/commit/210c24013c0e27e0e0180c974ab1305103b7aad4))
* **po:** add catalog combine API ([761c291](https://github.com/sebastian-software/ferrocat/commit/761c29145b0aa20fc62b53f70d164dcb27abb027))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 0.9.0 to 0.10.0

## [0.9.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-po-v0.8.0...ferrocat-po-v0.9.0) (2026-03-19)


### Features

* **catalog:** add ndjson storage format ([f335df9](https://github.com/sebastian-software/ferrocat/commit/f335df94693c2cb59bf54d2a9543f89184bfa6c0))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 0.8.0 to 0.9.0

## [0.8.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-po-v0.7.0...ferrocat-po-v0.8.0) (2026-03-18)


### Features

* **catalog:** expose public compiled key helper ([5a3e2c8](https://github.com/sebastian-software/ferrocat/commit/5a3e2c8a9ad1e1d25eced87cffc0920dbef6d02a))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 0.7.0 to 0.8.0

## [0.7.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-po-v0.6.0...ferrocat-po-v0.7.0) (2026-03-17)


### Features

* **catalog:** add compiled catalog artifact API ([26486d2](https://github.com/sebastian-software/ferrocat/commit/26486d2d520523e335cb8a8796b57818b7b1bb99))
* **catalog:** add compiled id metadata helpers ([0a7cef0](https://github.com/sebastian-software/ferrocat/commit/0a7cef052cf918dfe362cd79575922121fce78fa))
* **catalog:** add selected-key artifact compilation ([30fd036](https://github.com/sebastian-software/ferrocat/commit/30fd036f05f433aa529f92c57941be7187608d76))
* **catalog:** add selected-key compiled catalog primitives ([355dd46](https://github.com/sebastian-software/ferrocat/commit/355dd46e7b05da81698de61c46dfc2a25bb2f394))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 0.6.0 to 0.7.0

## [0.6.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-po-v0.5.2...ferrocat-po-v0.6.0) (2026-03-17)


### Features

* add compiled catalog runtime API ([f59a3da](https://github.com/sebastian-software/ferrocat/commit/f59a3dacb6a94088cf8536f5053882b939af36a7))


### Bug Fixes

* harden Rust APIs and expand public docs ([3cec26c](https://github.com/sebastian-software/ferrocat/commit/3cec26c426766b77b544497500d4eaf2c5815e0c))
* **rust:** tighten public API docs and idioms ([dcffdd1](https://github.com/sebastian-software/ferrocat/commit/dcffdd1436e5d0060e1671017660a18c6a204aa0))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 0.5.2 to 0.6.0

## [0.5.2](https://github.com/sebastian-software/ferrocat/compare/ferrocat-po-v0.5.1...ferrocat-po-v0.5.2) (2026-03-17)


### Bug Fixes

* trigger build ([682508b](https://github.com/sebastian-software/ferrocat/commit/682508b0cabf1f31ddbcfe6d2c76687600531eb4))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 0.5.1 to 0.5.2

## [0.5.1](https://github.com/sebastian-software/ferrocat/compare/ferrocat-po-v0.5.0...ferrocat-po-v0.5.1) (2026-03-17)


### Bug Fixes

* trigger build ([fc674b8](https://github.com/sebastian-software/ferrocat/commit/fc674b859b4483459892279a9ebc8aa191ab4da4))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 0.5.0 to 0.5.1

## [0.5.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-po-v0.4.0...ferrocat-po-v0.5.0) (2026-03-17)


### Features

* add source-first catalog input and normalized view ([4b1272c](https://github.com/sebastian-software/ferrocat/commit/4b1272ceeacd718445c0d60eff490f780740f37e))
* add source-first catalog input and normalized view ([1a0d295](https://github.com/sebastian-software/ferrocat/commit/1a0d295971bec7524b9e6113f3b2c40b5df2ce18))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 0.4.0 to 0.5.0

## [0.4.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-po-v0.3.0...ferrocat-po-v0.4.0) (2026-03-17)


### Features

* **ferrocat:** migrate workspace from ferrox ([fa6bf5b](https://github.com/sebastian-software/ferrocat/commit/fa6bf5bcbc7f1552f43596ae941b3483916cab3a))


### Bug Fixes

* **release:** align versions for release please ([96c0729](https://github.com/sebastian-software/ferrocat/commit/96c072927ca1bbcef0a66b0f74d4759645ca1d51))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 0.3.0 to 0.4.0

## [0.3.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-po-v0.2.0...ferrocat-po-v0.3.0) (2026-03-17)


### Features

* **ferrocat:** migrate workspace from ferrox ([fa6bf5b](https://github.com/sebastian-software/ferrocat/commit/fa6bf5bcbc7f1552f43596ae941b3483916cab3a))


### Bug Fixes

* **release:** align versions for release please ([96c0729](https://github.com/sebastian-software/ferrocat/commit/96c072927ca1bbcef0a66b0f74d4759645ca1d51))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-icu bumped from 0.2.0 to 0.3.0
