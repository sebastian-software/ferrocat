#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let parsed = ferrocat_icu::parse_icu(input).map(|_| ());
        let validated = ferrocat_icu::validate_icu(input);
        assert_eq!(validated, parsed);
    }
});
