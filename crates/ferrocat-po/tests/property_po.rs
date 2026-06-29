use std::collections::BTreeMap;

use ferrocat_po::{
    CatalogMode, ParseCatalogOptions, SerializeOptions, parse_catalog, parse_po, parse_po_borrowed,
    stringify_po,
};
use proptest::prelude::*;

fn msgid_strategy() -> impl Strategy<Value = String> {
    "[A-Za-z0-9 .,!?_/-]{1,32}"
        .prop_filter("msgid must not be blank", |value| !value.trim().is_empty())
}

fn msgstr_strategy() -> impl Strategy<Value = String> {
    "[A-Za-z0-9 .,!?_/-]{0,32}"
}

fn entries_strategy() -> impl Strategy<Value = Vec<(String, String)>> {
    prop::collection::btree_map(msgid_strategy(), msgstr_strategy(), 1..8)
        .prop_map(BTreeMap::into_iter)
        .prop_map(Iterator::collect)
}

fn render_po(entries: &[(String, String)]) -> String {
    let mut out =
        String::from("msgid \"\"\nmsgstr \"\"\n\"Content-Type: text/plain; charset=utf-8\\n\"\n\n");
    for (index, (msgid, msgstr)) in entries.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        out.push_str("msgid \"");
        out.push_str(msgid);
        out.push_str("\"\nmsgstr \"");
        out.push_str(msgstr);
        out.push_str("\"\n");
    }
    out
}

fn render_fcl(entries: &[(String, String)]) -> String {
    // The entries come from a `BTreeMap`, so they are already sorted and unique
    // by id as FCL requires, and the generated charset contains no `\t`, `\n`, or
    // `\\`, so no escaping is needed.
    let mut out = String::from("%FCL1\tsource=en\tlocale=de\n");
    for (msgid, msgstr) in entries {
        out.push_str(msgid);
        out.push_str("\t\t");
        out.push_str(msgstr);
        out.push('\n');
    }
    out
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        ..ProptestConfig::default()
    })]

    #[test]
    fn stringify_po_roundtrips_generated_catalog(entries in entries_strategy()) {
        let input = render_po(&entries);
        let parsed = parse_po(&input).expect("generated PO should parse");
        let rendered = stringify_po(&parsed, &SerializeOptions::default());
        let reparsed = parse_po(&rendered).expect("rendered PO should parse");

        prop_assert_eq!(reparsed, parsed);
    }

    #[test]
    fn borrowed_parser_matches_owned_parser_for_lf_only_generated_catalog(entries in entries_strategy()) {
        let input = render_po(&entries);
        let owned = parse_po(&input).expect("generated PO should parse");
        let borrowed = parse_po_borrowed(&input)
            .expect("generated LF-only PO should parse as borrowed")
            .into_owned();

        prop_assert_eq!(borrowed, owned);
    }

    #[test]
    fn catalog_api_accepts_equivalent_generated_po_and_fcl(entries in entries_strategy()) {
        let po = parse_catalog(ParseCatalogOptions {
            content: &render_po(&entries),
            locale: Some("de"),
            mode: CatalogMode::IcuPo,
            ..ParseCatalogOptions::new("", "en")
        })
        .expect("generated PO catalog should parse");
        let fcl = parse_catalog(ParseCatalogOptions {
            content: &render_fcl(&entries),
            mode: CatalogMode::IcuFcl,
            ..ParseCatalogOptions::new("", "en")
        })
        .expect("generated FCL catalog should parse");

        prop_assert_eq!(po.messages.len(), fcl.messages.len());
    }
}
