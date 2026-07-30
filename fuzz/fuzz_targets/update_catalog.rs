#![no_main]

use ferrocat_po::{CatalogMode, SourceExtractedMessage, UpdateCatalogOptions, update_catalog};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let extracted = vec![SourceExtractedMessage {
            msgid: "fuzz.message".to_owned(),
            ..SourceExtractedMessage::default()
        }];
        let _ = update_catalog(
            UpdateCatalogOptions::new("en", extracted)
                .with_existing(input)
                .with_mode(CatalogMode::IcuPo),
        );
    }
});
