#![no_main]

use ferrocat_po::{CatalogMode, ParseCatalogOptions, parse_catalog};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let _ = parse_catalog(ParseCatalogOptions::new(input, "en").with_mode(CatalogMode::IcuPo));
    }
});
