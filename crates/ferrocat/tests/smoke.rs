use ferrocat::{
    COMPILED_CATALOG_ARTIFACT_SCHEMA_VERSION, ParsePosition,
    catalog::{
        CatalogAuditIcuOptions, CatalogAuditOptions, CatalogCombineInput, CatalogMessageKey,
        CatalogMode, CatalogUpdateInput, CombineCatalogOptions, CompileCatalogArtifactOptions,
        CompileSelectedCatalogArtifactOptions, CompiledCatalogIdIndex, CompiledKeyStrategy,
        EffectiveTranslation, EffectiveTranslationRef, MergeCatalogsThreeWayOptions,
        ParseCatalogOptions, SourceExtractedMessage, WriteDurability, audit_catalogs,
        combine_catalogs, compile_catalog_artifact, compile_catalog_artifact_selected,
        merge_catalogs_three_way, parse_catalog, parse_catalog_for_review,
    },
    icu, parse_po_bytes, po,
};

#[test]
fn umbrella_crate_reexports_po_and_icu_surfaces() {
    assert!(
        ferrocat::collation::collation_key("apple") < ferrocat::collation::collation_key("Apple")
    );
    assert_eq!(WriteDurability::Rename, ferrocat::WriteDurability::Rename);
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
    let three_way = merge_catalogs_three_way(MergeCatalogsThreeWayOptions::new(
        CatalogCombineInput::new("msgid \"removed\"\nmsgstr \"old\"\n"),
        CatalogCombineInput::new(""),
        CatalogCombineInput::new(""),
        "en",
    ))
    .expect("merge catalogs three-way");
    assert!(!three_way.content.contains("removed"));

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
    let review_source = parse_catalog_for_review(
        ParseCatalogOptions::new("msgid \"hello\"\nmsgstr \"hello\"\n", "en").with_locale("en"),
    )
    .expect("parse review source catalog");
    let review_target = parse_catalog_for_review(
        ParseCatalogOptions::new("msgid \"hello\"\nmsgstr \"world\"\n", "en").with_locale("de"),
    )
    .expect("parse review target catalog");
    let audit = audit_catalogs(
        &[&review_source, &review_target],
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

#[test]
fn umbrella_runtime_policy_keeps_artifacts_and_selected_reports_consistent() {
    use ferrocat::catalog::{
        CompileCatalogArtifactReportOptions, IcuSyntaxPolicy, compile_catalog_artifact_report,
        compiled_key_with_policy,
    };
    let catalog = |locale, content| {
        parse_catalog(ParseCatalogOptions::new(content, "en").with_locale(locale))
            .expect("catalog")
            .into_normalized_view()
            .expect("normalized")
    };
    let source = catalog(
        "en",
        "msgid \"Use '<name>'\"\nmsgstr \"Use '<name>'\"\n\nmsgid \"Open <0>{name}</0>\"\nmsgstr \"Open <0>{name}</0>\"\n",
    );
    let target = catalog(
        "de",
        "msgid \"Use '<name>'\"\nmsgstr \"Nutze '<name>'\"\n\nmsgid \"Open <0>{name}</0>\"\nmsgstr \"Öffne <0>{name}</0>\"\n",
    );
    let catalogs = [&target, &source];
    let policy = IcuSyntaxPolicy::RuntimeLiteralApostrophes;
    assert_eq!(
        compiled_key_with_policy("''<0>Hello</0>'", None, policy),
        ferrocat::catalog::compiled_key("''<0>Hello</0>''", None),
        "migrating visible apostrophes around a rich tag preserves its former canonical key",
    );
    let mut options = CompileCatalogArtifactOptions::new("de", "en");
    options.icu_options.syntax_policy = policy;
    let full = compile_catalog_artifact(&catalogs, &options).expect("full artifact");
    let mut report_options = CompileCatalogArtifactReportOptions::new("de", "en");
    report_options.options.icu_options.syntax_policy = policy;
    let report = compile_catalog_artifact_report(&catalogs, &report_options).expect("report");
    assert_eq!(report.artifact, full);

    let index =
        CompiledCatalogIdIndex::new_with_policy(&catalogs, CompiledKeyStrategy::FerrocatV1, policy)
            .expect("policy-aware index");
    let literal_id = compiled_key_with_policy("Use '<name>'", None, policy);
    let rich_id = compiled_key_with_policy("Open <0>{name}</0>", None, policy);
    let ids = [literal_id.as_str(), rich_id.as_str()];
    let selected = compile_catalog_artifact_selected(
        &catalogs,
        &index,
        &CompileSelectedCatalogArtifactOptions::new("de", "en", &ids).with_options(options),
    )
    .expect("selected artifact");
    let mut selected_report =
        CompileCatalogArtifactReportOptions::selected("de", "en", &index, &ids);
    selected_report.options.icu_options.syntax_policy = policy;
    assert_eq!(
        compile_catalog_artifact_report(&catalogs, &selected_report)
            .expect("selected report")
            .artifact,
        full
    );
    assert_eq!(selected, full);
    let literal = icu::parse_icu(full.messages.get(&literal_id).expect("literal ID"))
        .expect("literal parses");
    assert_eq!(
        literal.nodes,
        vec![icu::IcuNode::Literal("Nutze <name>".into())]
    );
    let rich =
        icu::parse_icu(full.messages.get(&rich_id).expect("rich ID")).expect("rich text parses");
    assert!(
        rich.nodes
            .iter()
            .any(|node| matches!(node, icu::IcuNode::Tag { name, .. } if name == "0"))
    );
}

#[test]
fn umbrella_artifact_apis_reject_invalid_icu_by_default_and_can_collect_diagnostics() {
    use ferrocat::catalog::{CompileCatalogArtifactReportOptions, compile_catalog_artifact_report};
    let catalog = |locale, content| {
        parse_catalog(ParseCatalogOptions::new(content, "en").with_locale(locale))
            .expect("catalog")
            .into_normalized_view()
            .expect("normalized")
    };
    let source = catalog(
        "en",
        "msgid \"One\"\nmsgstr \"One\"\n\nmsgid \"Two\"\nmsgstr \"Two\"\n",
    );
    let target = catalog(
        "de",
        "msgid \"One\"\nmsgstr \"{broken\"\n\nmsgid \"Two\"\nmsgstr \"{also broken\"\n",
    );
    let catalogs = [&target, &source];
    let index =
        CompiledCatalogIdIndex::new(&catalogs, CompiledKeyStrategy::FerrocatV1).expect("index");
    let ids = index.iter().map(|(id, _)| id).collect::<Vec<_>>();
    assert!(
        compile_catalog_artifact(&catalogs, &CompileCatalogArtifactOptions::new("de", "en"))
            .is_err()
    );
    assert!(
        compile_catalog_artifact_selected(
            &catalogs,
            &index,
            &CompileSelectedCatalogArtifactOptions::new("de", "en", &ids)
        )
        .is_err()
    );
    assert!(
        compile_catalog_artifact_report(
            &catalogs,
            &CompileCatalogArtifactReportOptions::new("de", "en")
        )
        .is_err()
    );
    assert!(
        compile_catalog_artifact_report(
            &catalogs,
            &CompileCatalogArtifactReportOptions::selected("de", "en", &index, &ids)
        )
        .is_err()
    );
    let options = CompileCatalogArtifactOptions::new("de", "en").with_strict_icu(false);
    let full = compile_catalog_artifact(&catalogs, &options).expect("collect all diagnostics");
    assert_eq!(full.diagnostics.len(), 2);
    assert!(
        full.diagnostics
            .iter()
            .all(|d| d.code == "compile.invalid_icu_message")
    );
    assert_eq!(full.messages.len(), 2);
    assert_eq!(
        compile_catalog_artifact_selected(
            &catalogs,
            &index,
            &CompileSelectedCatalogArtifactOptions::new("de", "en", &ids)
                .with_options(options.clone())
        )
        .expect("selected diagnostics"),
        full
    );
    assert_eq!(
        compile_catalog_artifact_report(
            &catalogs,
            &CompileCatalogArtifactReportOptions::new("de", "en").with_options(options.clone())
        )
        .expect("report diagnostics")
        .artifact,
        full
    );
    assert_eq!(
        compile_catalog_artifact_report(
            &catalogs,
            &CompileCatalogArtifactReportOptions::selected("de", "en", &index, &ids)
                .with_options(options)
        )
        .expect("selected report diagnostics")
        .artifact,
        full
    );
}

#[test]
fn runtime_policy_reports_icu_error_positions_in_original_translation() {
    use ferrocat::catalog::{CompileCatalogArtifactIcuOptions, IcuSyntaxPolicy};
    let source = parse_catalog_for_review(
        ParseCatalogOptions::new("msgid \"Open\"\nmsgstr \"Open\"\n", "en").with_locale("en"),
    )
    .expect("source");
    let target = parse_catalog_for_review(
        ParseCatalogOptions::new("msgid \"Open\"\nmsgstr \"é don't {name\"\n", "en")
            .with_locale("de"),
    )
    .expect("target");
    let options = CompileCatalogArtifactOptions::new("de", "en").with_icu_options(
        CompileCatalogArtifactIcuOptions::new()
            .with_syntax_policy(IcuSyntaxPolicy::RuntimeLiteralApostrophes),
    );
    let audit = audit_catalogs(
        &[&target, &source],
        &CatalogAuditOptions::new("en").with_icu_options(
            CatalogAuditIcuOptions::new()
                .with_syntax_policy(IcuSyntaxPolicy::RuntimeLiteralApostrophes),
        ),
    )
    .expect("audit report");
    assert!(audit.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "icu.invalid_syntax" && diagnostic.message.contains("line 1, column 14")
    }));
    let error = compile_catalog_artifact(&[&target, &source], &options).expect_err("invalid ICU");
    assert!(error.to_string().contains("line 1, column 14"), "{error}");
    let artifact = compile_catalog_artifact(&[&target, &source], &options.with_strict_icu(false))
        .expect("diagnostic artifact");
    assert!(
        artifact.diagnostics[0]
            .message
            .contains("line 1, column 14"),
        "{:?}",
        artifact.diagnostics
    );
}
