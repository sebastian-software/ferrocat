#![cfg(feature = "collation")]

use ferrocat_po::collation::{CollationKey, collation_key};

#[test]
fn public_keys_preserve_catalog_order_and_normalization_ties() {
    let mut messages = vec![
        "日本語",
        "Zebra",
        "über",
        "Uber",
        "éclair",
        "eclair",
        "Apple",
        "apple",
        "Álgebra",
        "<0>Continue</0>",
        "{count}",
        "“Quoted”",
        "",
        "café",
        "cafe\u{0301}",
    ];
    messages.sort_by_cached_key(|text| (collation_key(text), *text));
    assert_eq!(
        messages,
        vec![
            "",
            "“Quoted”",
            "{count}",
            "<0>Continue</0>",
            "Álgebra",
            "apple",
            "Apple",
            "cafe\u{0301}",
            "café",
            "eclair",
            "éclair",
            "Uber",
            "über",
            "Zebra",
            "日本語",
        ]
    );
    assert_eq!(collation_key("café"), collation_key("cafe\u{0301}"));
    assert!(collation_key("aaá") < collation_key("aáa"));
    assert!(collation_key("aáa") < collation_key("áaa"));
    assert!(collation_key("a") < collation_key("á"));
    assert!(collation_key("á") < collation_key("à"));
    assert!(collation_key("Ω") < collation_key("日"));
    assert!(collation_key("ω") < collation_key("Ω"));
    assert!(collation_key("\0") > collation_key("z"));
}

#[test]
fn owned_keys_allow_raw_identity_and_duplicate_tiebreaks() {
    fn identity(
        message: &str,
        context: Option<&str>,
        obsolete: bool,
    ) -> (CollationKey, CollationKey, String, Option<String>, bool) {
        (
            collation_key(message),
            collation_key(context.unwrap_or("")),
            message.to_owned(),
            context.map(str::to_owned),
            obsolete,
        )
    }
    let mut identities = [
        identity("café", Some(""), true),
        identity("café", Some(""), false),
        identity("café", None, false),
        identity("cafe\u{0301}", None, false),
        identity("café", Some("menu"), false),
        identity("café", None, false),
    ];
    identities.sort();
    assert_eq!(identities[0].2, "cafe\u{0301}");
    assert_eq!(identities[1], identities[2]);
    assert_eq!(identities[1].3, None);
    assert_eq!(identities[3].3.as_deref(), Some(""));
    assert!(!identities[3].4);
    assert!(identities[4].4);
    assert_eq!(identities[5].3.as_deref(), Some("menu"));
    let key = collation_key(&String::from("temporary"));
    assert_eq!(key, collation_key("temporary"));
}
