# Changelog

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
