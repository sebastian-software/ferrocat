#![no_main]

use ferrocat_po::{
    CatalogSemantics, CatalogStorageFormat, ParseCatalogOptions, PluralEncoding, parse_catalog,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let _ = parse_catalog(ParseCatalogOptions {
            content: input,
            source_locale: "en",
            storage_format: CatalogStorageFormat::Ndjson,
            semantics: CatalogSemantics::IcuNative,
            plural_encoding: PluralEncoding::Icu,
            ..ParseCatalogOptions::default()
        });
    }
});
