# Changelog

## [2.1.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-icu-v2.0.0...ferrocat-icu-v2.1.0) (2026-07-02)


### Features

* **api:** add option builder setters ([#208](https://github.com/sebastian-software/ferrocat/issues/208)) ([8b4c279](https://github.com/sebastian-software/ferrocat/commit/8b4c2793a74871b435c74b405bb965ad88e6d4bd))
* **api:** align public cleanup names ([80924ea](https://github.com/sebastian-software/ferrocat/commit/80924eac840d9445f36bf5f5f23b7153e1a12eca))
* **api:** make options extensible ([#213](https://github.com/sebastian-software/ferrocat/issues/213)) ([b8c9a6a](https://github.com/sebastian-software/ferrocat/commit/b8c9a6adea3b6d063599964e6ae66e997458f0f1))


### Performance Improvements

* **po:** reduce parser and serializer allocations ([#197](https://github.com/sebastian-software/ferrocat/issues/197)) ([20ca38e](https://github.com/sebastian-software/ferrocat/commit/20ca38ea8fcbe90578badc8a455c828556448cd8))

## [2.0.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-icu-v1.3.2...ferrocat-icu-v2.0.0) (2026-06-30)


### Miscellaneous Chores

* **ferrocat-icu:** Synchronize ferrocat versions

## [1.3.2](https://github.com/sebastian-software/ferrocat/compare/ferrocat-icu-v1.3.1...ferrocat-icu-v1.3.2) (2026-06-29)


### Miscellaneous Chores

* **ferrocat-icu:** Synchronize ferrocat versions

## [1.3.1](https://github.com/sebastian-software/ferrocat/compare/ferrocat-icu-v1.3.0...ferrocat-icu-v1.3.1) (2026-06-25)


### Performance Improvements

* **icu:** bulk-copy apostrophe and style literals with memchr ([46aabb9](https://github.com/sebastian-software/ferrocat/commit/46aabb9070d640ac2b73fc87b64f7e39b4b23fba))
* **icu:** jump to syntax bytes with memchr in parse_nodes ([c6554a1](https://github.com/sebastian-software/ferrocat/commit/c6554a13512507b108b2c44c543f256f19f08a4e))
* parser/merge optimizations and cross-runtime benchmark comparisons ([3c635e5](https://github.com/sebastian-software/ferrocat/commit/3c635e55eb3e29f957e99556acaf0ad14d00b819))

## [1.3.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-icu-v1.2.1...ferrocat-icu-v1.3.0) (2026-06-23)


### Miscellaneous Chores

* **ferrocat-icu:** Synchronize ferrocat versions

## [1.2.1](https://github.com/sebastian-software/ferrocat/compare/ferrocat-icu-v1.2.0...ferrocat-icu-v1.2.1) (2026-06-23)


### Miscellaneous Chores

* **ferrocat-icu:** Synchronize ferrocat versions

## [1.2.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-icu-v1.1.1...ferrocat-icu-v1.2.0) (2026-06-23)


### Features

* **icu:** add ICU-aware pseudolocalization ([7553319](https://github.com/sebastian-software/ferrocat/commit/75533192985d6d781cd3d7dfd045fdbf4a337fc9))

## [1.1.1](https://github.com/sebastian-software/ferrocat/compare/ferrocat-icu-v1.1.0...ferrocat-icu-v1.1.1) (2026-06-19)


### Miscellaneous Chores

* **ferrocat-icu:** Synchronize ferrocat versions

## [1.1.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-icu-v1.0.0...ferrocat-icu-v1.1.0) (2026-06-19)


### Miscellaneous Chores

* **ferrocat-icu:** Synchronize ferrocat versions

## [1.0.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-icu-v0.13.0...ferrocat-icu-v1.0.0) (2026-06-18)


### Features

* **api:** add diagnostic codes and io path context ([#103](https://github.com/sebastian-software/ferrocat/issues/103)) ([0a97a9e](https://github.com/sebastian-software/ferrocat/commit/0a97a9e1495f4a9dbec16e6261e7523982deb911))
* **api:** mark growth-prone enums non-exhaustive ([#101](https://github.com/sebastian-software/ferrocat/issues/101)) ([b4e1ca4](https://github.com/sebastian-software/ferrocat/commit/b4e1ca4262345f6e8927582ae477f104075b1474))
* **features:** add lean parser profiles ([#106](https://github.com/sebastian-software/ferrocat/issues/106)) ([2887beb](https://github.com/sebastian-software/ferrocat/commit/2887bebfedabb7664f15c12912ea96626b0d103b))
* **icu:** expose stringify_icu ([#96](https://github.com/sebastian-software/ferrocat/issues/96)) ([a581dcb](https://github.com/sebastian-software/ferrocat/commit/a581dcb171a1f1321640f9f22f6da165d23c7f06))
* **icu:** validate formatter support from analysis ([#79](https://github.com/sebastian-software/ferrocat/issues/79)) ([7089df6](https://github.com/sebastian-software/ferrocat/commit/7089df67cdb4803fa72dd79e9f077565aefc91cc))

## [0.13.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-icu-v0.12.0...ferrocat-icu-v0.13.0) (2026-06-12)


### Features

* validate icu formatter runtime support ([#44](https://github.com/sebastian-software/ferrocat/issues/44)) ([1ede8b3](https://github.com/sebastian-software/ferrocat/commit/1ede8b376f693bdc3f0b3bb2ad70308d7b6a19bb))

## [0.12.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-icu-v0.11.0...ferrocat-icu-v0.12.0) (2026-05-21)


### Miscellaneous Chores

* **ferrocat-icu:** Synchronize ferrocat versions

## [0.11.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-icu-v0.10.0...ferrocat-icu-v0.11.0) (2026-05-12)


### Features

* **icu:** add authoring diagnostics ([ea53674](https://github.com/sebastian-software/ferrocat/commit/ea5367412fcfcd636a9ae1b3e08a3a33ecae9f74))
* **icu:** add semantic message metadata ([1a0a7bc](https://github.com/sebastian-software/ferrocat/commit/1a0a7bcb477d8643d63175c4f4584911572fb934))

## [0.10.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-icu-v0.9.0...ferrocat-icu-v0.10.0) (2026-05-11)


### Miscellaneous Chores

* **ferrocat-icu:** Synchronize ferrocat versions

## [0.9.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-icu-v0.8.0...ferrocat-icu-v0.9.0) (2026-03-19)


### Miscellaneous Chores

* **ferrocat-icu:** Synchronize ferrocat versions

## [0.8.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-icu-v0.7.0...ferrocat-icu-v0.8.0) (2026-03-18)


### Miscellaneous Chores

* **ferrocat-icu:** Synchronize ferrocat versions

## [0.7.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-icu-v0.6.0...ferrocat-icu-v0.7.0) (2026-03-17)


### Miscellaneous Chores

* **ferrocat-icu:** Synchronize ferrocat versions

## [0.6.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-icu-v0.5.2...ferrocat-icu-v0.6.0) (2026-03-17)


### Bug Fixes

* harden Rust APIs and expand public docs ([3cec26c](https://github.com/sebastian-software/ferrocat/commit/3cec26c426766b77b544497500d4eaf2c5815e0c))
* **rust:** tighten public API docs and idioms ([dcffdd1](https://github.com/sebastian-software/ferrocat/commit/dcffdd1436e5d0060e1671017660a18c6a204aa0))

## [0.5.2](https://github.com/sebastian-software/ferrocat/compare/ferrocat-icu-v0.5.1...ferrocat-icu-v0.5.2) (2026-03-17)


### Bug Fixes

* trigger build ([682508b](https://github.com/sebastian-software/ferrocat/commit/682508b0cabf1f31ddbcfe6d2c76687600531eb4))

## [0.5.1](https://github.com/sebastian-software/ferrocat/compare/ferrocat-icu-v0.5.0...ferrocat-icu-v0.5.1) (2026-03-17)


### Bug Fixes

* trigger build ([fc674b8](https://github.com/sebastian-software/ferrocat/commit/fc674b859b4483459892279a9ebc8aa191ab4da4))

## [0.5.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-icu-v0.4.0...ferrocat-icu-v0.5.0) (2026-03-17)


### Miscellaneous Chores

* **ferrocat-icu:** Synchronize ferrocat versions

## [0.4.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-icu-v0.3.0...ferrocat-icu-v0.4.0) (2026-03-17)


### Features

* **ferrocat:** migrate workspace from ferrox ([fa6bf5b](https://github.com/sebastian-software/ferrocat/commit/fa6bf5bcbc7f1552f43596ae941b3483916cab3a))


### Bug Fixes

* **release:** align versions for release please ([96c0729](https://github.com/sebastian-software/ferrocat/commit/96c072927ca1bbcef0a66b0f74d4759645ca1d51))

## [0.3.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-icu-v0.2.0...ferrocat-icu-v0.3.0) (2026-03-17)


### Features

* **ferrocat:** migrate workspace from ferrox ([fa6bf5b](https://github.com/sebastian-software/ferrocat/commit/fa6bf5bcbc7f1552f43596ae941b3483916cab3a))


### Bug Fixes

* **release:** align versions for release please ([96c0729](https://github.com/sebastian-software/ferrocat/commit/96c072927ca1bbcef0a66b0f74d4759645ca1d51))
