use ferrocat::{
    CatalogMessageKey, CatalogUpdateInput, CompileCatalogArtifactOptions, EffectiveTranslation,
    EffectiveTranslationRef, MergeExtractedMessage, ParseCatalogOptions, SerializeOptions,
    SourceExtractedMessage, compile_catalog_artifact, has_select_ordinal, merge_catalog,
    parse_catalog, parse_icu, parse_po, stringify_po,
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

    let _source_input = CatalogUpdateInput::SourceFirst(vec![SourceExtractedMessage {
        msgid: "hello".into(),
        ..SourceExtractedMessage::default()
    }]);

    let parsed_catalog = parse_catalog(ParseCatalogOptions {
        content: "msgid \"hello\"\nmsgstr \"world\"\n".to_owned(),
        locale: Some("de".to_owned()),
        source_locale: "en".to_owned(),
        ..ParseCatalogOptions::default()
    })
    .expect("parse catalog");
    let normalized = parsed_catalog
        .into_normalized_view()
        .expect("normalized view");
    let key = CatalogMessageKey::new("hello", None);
    assert!(matches!(
        normalized.effective_translation(&key),
        Some(EffectiveTranslationRef::Singular("world"))
    ));
    assert_eq!(
        normalized.effective_translation_with_source_fallback(&key, "en"),
        Some(EffectiveTranslation::Singular("world".to_owned()))
    );

    let source = parse_catalog(ParseCatalogOptions {
        content: "msgid \"hello\"\nmsgstr \"hello\"\n".to_owned(),
        locale: Some("en".to_owned()),
        source_locale: "en".to_owned(),
        ..ParseCatalogOptions::default()
    })
    .expect("parse source catalog")
    .into_normalized_view()
    .expect("normalized source catalog");
    let artifact = compile_catalog_artifact(
        &[&normalized, &source],
        &CompileCatalogArtifactOptions {
            requested_locale: "de".to_owned(),
            source_locale: "en".to_owned(),
            ..CompileCatalogArtifactOptions::default()
        },
    )
    .expect("compile artifact");
    assert_eq!(artifact.messages.len(), 1);
}
