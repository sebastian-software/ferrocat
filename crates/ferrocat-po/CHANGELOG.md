# Changelog

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
