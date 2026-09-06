# Changelog

> Version 3.0.0 was published and then yanked; 2.2.0 superseded it without the
> breaking change, so the 2.2.0 compare links below run backwards from 3.0.0.

## Unreleased

### Bug Fixes

- `ferrocat audit` now reports active fuzzy entries through the shared
  review-aware catalog classifier.

## [2.2.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-cli-v3.0.0...ferrocat-cli-v2.2.0) (2026-07-05)


### Miscellaneous Chores

* **ferrocat-cli:** Synchronize ferrocat versions


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-po bumped from 3.0.0 to 2.2.0

## [3.0.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-cli-v2.1.1...ferrocat-cli-v3.0.0) (2026-07-05)


### Miscellaneous Chores

* **ferrocat-cli:** Synchronize ferrocat versions


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-po bumped from 2.1.1 to 3.0.0

## [2.1.1](https://github.com/sebastian-software/ferrocat/compare/ferrocat-cli-v2.1.0...ferrocat-cli-v2.1.1) (2026-07-03)


### Miscellaneous Chores

* **ferrocat-cli:** Synchronize ferrocat versions


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-po bumped from 2.1.0 to 2.1.1

## [2.1.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-cli-v2.0.0...ferrocat-cli-v2.1.0) (2026-07-02)


### Features

* **api:** make options extensible ([#213](https://github.com/sebastian-software/ferrocat/issues/213)) ([b8c9a6a](https://github.com/sebastian-software/ferrocat/commit/b8c9a6adea3b6d063599964e6ae66e997458f0f1))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-po bumped from 2.0.0 to 2.1.0

## [2.0.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-cli-v1.3.2...ferrocat-cli-v2.0.0) (2026-06-30)


### ⚠ BREAKING CHANGES

* **po:** the NDJSON catalog storage format and its public types (NdjsonCatalogReader/Writer + options, CatalogStorageFormat::Ndjson, CatalogFileFormat::Ndjson, CatalogMode::IcuNdjson) are removed. Use FCL (CatalogMode::IcuFcl, .fcl files) instead.

### Features

* **po:** remove NDJSON catalog format in favor of FCL ([9606441](https://github.com/sebastian-software/ferrocat/commit/96064410a154b3c05f92813d10156e4a2f454ed4))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-po bumped from 1.3.2 to 2.0.0

## [1.3.2](https://github.com/sebastian-software/ferrocat/compare/ferrocat-cli-v1.3.1...ferrocat-cli-v1.3.2) (2026-06-29)


### Miscellaneous Chores

* **ferrocat-cli:** Synchronize ferrocat versions


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-po bumped from 1.3.1 to 1.3.2

## [1.3.1](https://github.com/sebastian-software/ferrocat/compare/ferrocat-cli-v1.3.0...ferrocat-cli-v1.3.1) (2026-06-25)


### Miscellaneous Chores

* **ferrocat-cli:** Synchronize ferrocat versions


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-po bumped from 1.3.0 to 1.3.1

## [1.3.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-cli-v1.2.1...ferrocat-cli-v1.3.0) (2026-06-23)


### Miscellaneous Chores

* **ferrocat-cli:** Synchronize ferrocat versions


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-po bumped from 1.2.1 to 1.3.0

## [1.2.1](https://github.com/sebastian-software/ferrocat/compare/ferrocat-cli-v1.2.0...ferrocat-cli-v1.2.1) (2026-06-23)


### Miscellaneous Chores

* **ferrocat-cli:** Synchronize ferrocat versions


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-po bumped from 1.2.0 to 1.2.1

## [1.2.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-cli-v1.1.1...ferrocat-cli-v1.2.0) (2026-06-23)


### Miscellaneous Chores

* **ferrocat-cli:** Synchronize ferrocat versions


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-po bumped from 1.1.1 to 1.2.0

## [1.1.1](https://github.com/sebastian-software/ferrocat/compare/ferrocat-cli-v1.1.0...ferrocat-cli-v1.1.1) (2026-06-19)


### Miscellaneous Chores

* **ferrocat-cli:** Synchronize ferrocat versions


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-po bumped from 1.1.0 to 1.1.1

## [1.1.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-cli-v1.0.0...ferrocat-cli-v1.1.0) (2026-06-19)


### Miscellaneous Chores

* **ferrocat-cli:** Synchronize ferrocat versions


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-po bumped from 1.0.0 to 1.1.0

## [1.0.0](https://github.com/sebastian-software/ferrocat/compare/ferrocat-cli-v0.13.0...ferrocat-cli-v1.0.0) (2026-06-18)


### ⚠ BREAKING CHANGES

* **api:** collapse redundant option fields into CatalogMode ([#102](https://github.com/sebastian-software/ferrocat/issues/102))

### Features

* **api:** collapse redundant option fields into CatalogMode ([#102](https://github.com/sebastian-software/ferrocat/issues/102)) ([2687df6](https://github.com/sebastian-software/ferrocat/commit/2687df6363755b0ef863594168da8be027d34614))
* **cli:** add audit release gate ([#108](https://github.com/sebastian-software/ferrocat/issues/108)) ([e4d9727](https://github.com/sebastian-software/ferrocat/commit/e4d9727c2e2010cdf48416f8ed52e45ff56982e6))


### Bug Fixes

* trigger build ([682508b](https://github.com/sebastian-software/ferrocat/commit/682508b0cabf1f31ddbcfe6d2c76687600531eb4))
* trigger build ([fc674b8](https://github.com/sebastian-software/ferrocat/commit/fc674b859b4483459892279a9ebc8aa191ab4da4))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * ferrocat-po bumped from 0.13.0 to 1.0.0
