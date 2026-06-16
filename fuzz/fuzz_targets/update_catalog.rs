#![no_main]

use ferrocat_po::{
    CatalogSemantics, CatalogStorageFormat, PluralEncoding, SourceExtractedMessage,
    UpdateCatalogOptions, update_catalog,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let extracted = vec![SourceExtractedMessage {
            msgid: "fuzz.message".to_owned(),
            ..SourceExtractedMessage::default()
        }];
        let _ = update_catalog(UpdateCatalogOptions {
            source_locale: "en",
            input: extracted.into(),
            existing: Some(input),
            storage_format: CatalogStorageFormat::Po,
            semantics: CatalogSemantics::IcuNative,
            plural_encoding: PluralEncoding::Icu,
            ..UpdateCatalogOptions::default()
        });
    }
});
