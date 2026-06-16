#![no_main]

use std::borrow::Cow;

use ferrocat_po::{MergeExtractedMessage, merge_catalog};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let extracted = [MergeExtractedMessage {
            msgid: Cow::Borrowed("fuzz.message"),
            references: vec![Cow::Borrowed("fuzz.rs:1")],
            ..MergeExtractedMessage::default()
        }];
        let _ = merge_catalog(input, &extracted);
    }
});
