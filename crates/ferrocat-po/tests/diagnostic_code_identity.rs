//! Consumer-facing feature-unification and wire-format contracts.
use ferrocat_po::DiagnosticCode;

trait ConsumerClassification {
    fn domain(&self) -> &'static str;
}

impl ConsumerClassification for DiagnosticCode {
    fn domain(&self) -> &'static str {
        "po"
    }
}

// Both local implementations must coexist with and without `catalog`.
// The dev-only dependency lets the lean profile express this consumer case.
impl ConsumerClassification for ferrocat_icu::DiagnosticCode {
    fn domain(&self) -> &'static str {
        "icu"
    }
}

#[test]
fn po_code_has_the_same_consumer_contract_in_every_feature_profile() {
    let code = DiagnosticCode::from("icu.missing_argument");
    assert_eq!(
        ferrocat_icu::DiagnosticCode::from("icu.missing_argument").domain(),
        "icu"
    );
    assert_eq!(code.domain(), "po");
    assert_eq!(code, "icu.missing_argument");
    assert_eq!("icu.missing_argument", code);
    assert_eq!(code.to_string(), "icu.missing_argument");
    assert_eq!(code.into_string(), "icu.missing_argument");
}

#[cfg(feature = "catalog")]
#[test]
fn crate_codes_are_distinct_and_convert_without_changing_spelling() {
    let icu = ferrocat_icu::DiagnosticCode::from("icu.missing_argument");
    assert_eq!(icu.domain(), "icu");
    let po: DiagnosticCode = icu.into();
    assert_eq!(po.domain(), "po");
    let icu: ferrocat_icu::DiagnosticCode = po.into();
    assert_eq!(icu.as_str(), "icu.missing_argument");
}

#[cfg(feature = "serde")]
#[test]
fn serde_keeps_the_plain_string_wire_format() {
    let code = DiagnosticCode::from("icu.missing_argument");
    assert_eq!(
        serde_json::to_string(&code).expect("serialize PO code"),
        "\"icu.missing_argument\""
    );
    assert_eq!(
        serde_json::from_str::<DiagnosticCode>("\"icu.missing_argument\"")
            .expect("deserialize PO code"),
        code
    );
    #[cfg(feature = "catalog")]
    {
        let icu = ferrocat_icu::DiagnosticCode::from(code);
        let wire = serde_json::to_string(&icu).expect("serialize ICU code");
        assert_eq!(wire, "\"icu.missing_argument\"");
        assert_eq!(
            serde_json::from_str::<ferrocat_icu::DiagnosticCode>(&wire)
                .expect("deserialize ICU code"),
            icu
        );
    }
}
