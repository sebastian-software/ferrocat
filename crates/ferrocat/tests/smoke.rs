use ferrocat::{
    COMPILED_CATALOG_ARTIFACT_SCHEMA_VERSION, ParsePosition,
    catalog::{
        CatalogAuditIcuOptions, CatalogAuditOptions, CatalogCombineInput, CatalogMessageKey,
        CatalogMode, CatalogUpdateInput, CombineCatalogOptions, CompileCatalogArtifactOptions,
        CompileSelectedCatalogArtifactOptions, CompiledCatalogIdIndex, CompiledKeyStrategy,
        EffectiveTranslation, EffectiveTranslationRef, ParseCatalogOptions, SourceExtractedMessage,
        audit_catalogs, combine_catalogs, compile_catalog_artifact,
        compile_catalog_artifact_selected, parse_catalog,
    },
    icu, parse_po_bytes, po,
};

#[test]
fn umbrella_crate_reexports_po_and_icu_surfaces() {
    let mut file = po::parse_po(
        r#"
msgid "hello"
msgstr "world"
"#,
    )
    .expect("parse po");

    file.items[0].msgstr = "Welt".to_owned().into();

    let rendered = po::stringify_po(&file, &po::SerializeOptions::default());
    assert!(rendered.contains(r#"msgstr "Welt""#));

    let bytes = parse_po_bytes(b"msgid \"bytes\"\nmsgstr \"ok\"\n").expect("parse po bytes");
    assert_eq!(bytes.items[0].msgid, "bytes");
    let byte_position = ParsePosition::new(12, 2, 3);
    assert_eq!(byte_position.line(), 2);
    assert_eq!(po::ParsePosition::new(4, 1, 5).column(), 5);
    assert_eq!(
        po::diagnostic_codes::plural::UNSUPPORTED_GETTEXT_EXPORT,
        "plural.unsupported_gettext_export"
    );

    let merged = po::merge_catalog(
        rendered.as_str(),
        &[po::MergeMessageInput {
            msgid: "hello".into(),
            ..po::MergeMessageInput::default()
        }],
    )
    .expect("merge catalog");
    assert!(merged.contains(r#"msgid "hello""#));

    let combine_inputs = [
        CatalogCombineInput::new("msgid \"hello\"\nmsgstr \"world\"\n"),
        CatalogCombineInput::new("msgid \"bye\"\nmsgstr \"\"\n"),
    ];
    let combined = combine_catalogs(CombineCatalogOptions::new(&combine_inputs, "en"))
        .expect("combine catalogs");
    assert!(combined.content.contains(r#"msgid "bye""#));

    let message =
        icu::parse_icu("{count, selectordinal, one {#st} other {#th}}").expect("parse icu");
    assert!(icu::has_select_ordinal(&message));
    assert_eq!(
        icu::diagnostic_codes::icu::MISSING_ARGUMENT,
        "icu.missing_argument"
    );

    let _source_input = CatalogUpdateInput::SourceFirst(vec![SourceExtractedMessage {
        msgid: "hello".into(),
        ..SourceExtractedMessage::default()
    }]);

    // Every catalog mode must be selectable through umbrella re-exports alone.
    parse_catalog(
        ParseCatalogOptions::new("msgid \"hello\"\nmsgstr \"world\"\n", "en")
            .with_locale("de")
            .with_mode(CatalogMode::GettextPo),
    )
    .expect("parse gettext-compat catalog");

    let parsed_catalog = parse_catalog(
        ParseCatalogOptions::new("msgid \"hello\"\nmsgstr \"world\"\n", "en").with_locale("de"),
    )
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

    let source = parse_catalog(
        ParseCatalogOptions::new("msgid \"hello\"\nmsgstr \"hello\"\n", "en").with_locale("en"),
    )
    .expect("parse source catalog")
    .into_normalized_view()
    .expect("normalized source catalog");
    let audit = audit_catalogs(
        &[&source, &normalized],
        &CatalogAuditOptions::new("en").with_icu_options(CatalogAuditIcuOptions::default()),
    )
    .expect("audit catalogs with icu options");
    assert!(!audit.has_errors());
    assert_eq!(
        ferrocat::catalog::diagnostic_codes::catalog::MISSING_TRANSLATION,
        "catalog.missing_translation"
    );
    assert_eq!(
        ferrocat::catalog::COMPILED_CATALOG_ARTIFACT_SCHEMA_VERSION,
        COMPILED_CATALOG_ARTIFACT_SCHEMA_VERSION
    );

    let artifact = compile_catalog_artifact(
        &[&normalized, &source],
        &CompileCatalogArtifactOptions::new("de", "en"),
    )
    .expect("compile artifact");
    assert_eq!(artifact.messages.len(), 1);

    let index =
        CompiledCatalogIdIndex::new(&[&normalized, &source], CompiledKeyStrategy::FerrocatV1)
            .expect("compiled id index");
    let compiled_ids = index.iter().map(|(id, _)| id).collect::<Vec<_>>();
    let selected_artifact = compile_catalog_artifact_selected(
        &[&normalized, &source],
        &index,
        &CompileSelectedCatalogArtifactOptions::new("de", "en", &compiled_ids),
    )
    .expect("compile selected artifact");
    assert_eq!(selected_artifact.messages.len(), 1);
}
