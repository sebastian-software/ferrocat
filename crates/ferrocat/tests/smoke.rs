use ferrocat::{
    MergeExtractedMessage, SerializeOptions, has_select_ordinal, merge_catalog, parse_icu,
    parse_po, stringify_po,
};

#[test]
fn umbrella_crate_reexports_po_and_icu_surfaces() {
    let mut file = parse_po(
        r#"
msgid "hello"
msgstr "world"
"#,
    )
    .expect("parse po");

    file.items[0].msgstr = "Welt".to_owned().into();

    let rendered = stringify_po(&file, &SerializeOptions::default());
    assert!(rendered.contains(r#"msgstr "Welt""#));

    let merged = merge_catalog(
        rendered.as_str(),
        &[MergeExtractedMessage {
            msgid: "hello".into(),
            ..MergeExtractedMessage::default()
        }],
    )
    .expect("merge catalog");
    assert!(merged.contains(r#"msgid "hello""#));

    let message = parse_icu("{count, selectordinal, one {#st} other {#th}}").expect("parse icu");
    assert!(has_select_ordinal(&message));
}
