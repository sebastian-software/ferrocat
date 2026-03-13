use std::collections::BTreeMap;

use ferrocat::{
    compare_variables, compile_catalog, compile_icu, create_default_headers, extract_variable_info,
    extract_variables, format_po_date, generate_message_ids, get_plural_categories,
    get_plural_count, get_plural_forms_header, get_plural_index, gettext_to_icu, has_icu_syntax,
    has_plural, has_select, has_select_ordinal, icu_to_gettext_source, is_plural_item,
    normalize_item_to_icu, normalize_to_icu, normalize_to_icu_in_place, parse_icu,
    parse_plural_forms, serialize_compiled_catalog, validate_icu, Catalog, CatalogEntry,
    CatalogKeyStrategy, CatalogTranslation, CompileCatalogOptions, CompileIcuOptions,
    CreateHeadersOptions, GettextToIcuOptions, IcuErrorKind, IcuNode, IcuParser, IcuParserOptions,
    IcuPluralType, ItemsToCatalogOptions, MessageIdInput, ParsedPluralForms, PoDateTime, PoFile,
    PoItem, SerializedCompiledMessageKind,
};

mod headers {
    use super::*;

    #[test]
    fn create_default_headers_uses_public_options_and_date_types() {
        let now = PoDateTime {
            year: 2026,
            month: 3,
            day: 12,
            hour: 9,
            minute: 30,
            offset_minutes: 60,
        };

        let headers = create_default_headers(&CreateHeadersOptions {
            language: Some(String::from("pl")),
            generator: Some(String::from("ferrocat-tests")),
            now: Some(now),
            custom: BTreeMap::from([(String::from("X-Domain"), String::from("frontend"))]),
            ..CreateHeadersOptions::default()
        });

        assert_eq!(format_po_date(now), "2026-03-12 09:30+0100");
        assert_eq!(
            headers.get("Plural-Forms").map(String::as_str),
            Some("nplurals=4; plural=(n != 1);")
        );
        assert_eq!(
            headers.get("POT-Creation-Date").map(String::as_str),
            Some("2026-03-12 09:30+0100")
        );
        assert_eq!(
            headers.get("X-Domain").map(String::as_str),
            Some("frontend")
        );
    }
}

mod message_ids {
    use super::*;

    #[test]
    fn generate_message_ids_accepts_message_id_input_constructor() {
        let ids = generate_message_ids(&[
            MessageIdInput::new("Hello", None::<String>),
            MessageIdInput::new("Open", Some("menu.file")),
        ]);

        assert_eq!(ids.get("Hello").map(String::as_str), Some("GF-NsyJx"));
        assert_eq!(
            ids.get("Open\u{0004}menu.file").map(String::as_str),
            Some("QWh_hL4_")
        );
    }
}

mod plurals {
    use super::*;

    #[test]
    fn plural_helpers_expose_categories_counts_indices_and_headers() {
        assert_eq!(
            parse_plural_forms(Some("nplurals=2; plural=(n != 1);")),
            ParsedPluralForms {
                nplurals: Some(String::from("2")),
                plural: Some(String::from("(n != 1)")),
            }
        );
        assert_eq!(
            get_plural_categories("pl"),
            &["one", "few", "many", "other"]
        );
        assert_eq!(get_plural_count("pl"), 4);
        assert_eq!(get_plural_index("pl", 2.0), 1);
        assert_eq!(get_plural_forms_header("zh"), "nplurals=1; plural=0;");
    }
}

mod icu {
    use super::*;

    fn plural_item() -> PoItem {
        let mut item = PoItem::new(2);
        item.msgid = String::from("{count} item");
        item.msgid_plural = Some(String::from("{count} items"));
        item.msgstr = vec![String::from("# Artikel"), String::from("# Artikel")];
        item
    }

    #[test]
    fn parse_and_validate_icu_use_public_ast_and_error_types() {
        let ast = parse_icu(
            "{count, selectordinal, one {#st} other {#th}}",
            IcuParserOptions::default(),
        )
        .expect("message should parse");

        match ast.first().expect("ast should contain a node") {
            IcuNode::Plural { plural_type, .. } => assert_eq!(*plural_type, IcuPluralType::Ordinal),
            other => panic!("expected plural node, got {other:?}"),
        }

        let parser_ast = IcuParser::new(
            "{name}",
            IcuParserOptions {
                ignore_tag: false,
                requires_other_clause: true,
            },
        )
        .parse()
        .expect("parser should parse");
        assert_eq!(
            parser_ast,
            vec![IcuNode::Argument {
                value: String::from("name")
            }]
        );

        let error = parse_icu("{name", IcuParserOptions::default()).expect_err("parse should fail");
        assert_eq!(error.kind, IcuErrorKind::SyntaxError);

        let validation = validate_icu("{count, plural, one {# file}}", IcuParserOptions::default());
        assert!(!validation.valid);
        assert_eq!(validation.errors[0].kind, IcuErrorKind::SyntaxError);
    }

    #[test]
    fn analysis_helpers_expose_variables_comparisons_and_syntax_checks() {
        let message = "{count, plural, one {{name}} other {<b>{count}</b>}}";

        assert_eq!(
            extract_variables(message),
            vec![String::from("count"), String::from("name")]
        );

        let variable_info = extract_variable_info(message);
        assert!(variable_info
            .iter()
            .any(|variable| variable.name == "count" && variable.kind == "plural"));
        assert!(variable_info
            .iter()
            .any(|variable| variable.name == "name" && variable.kind == "argument"));

        let comparison = compare_variables("{name} {count}", "{name}");
        assert_eq!(comparison.missing, vec![String::from("count")]);
        assert!(!comparison.is_match);

        assert!(has_plural(message));
        assert!(has_select("{gender, select, male {He} other {They}}"));
        assert!(has_select_ordinal(
            "{place, selectordinal, one {#st} other {#th}}"
        ));
        assert!(has_icu_syntax(message));
    }

    #[test]
    fn conversion_helpers_roundtrip_plural_items_through_public_surface() {
        let options = GettextToIcuOptions::new("de");
        let original = plural_item();

        assert!(is_plural_item(&original));

        let icu = gettext_to_icu(&original, &options).expect("plural item should convert");
        assert_eq!(
            icu,
            "{count, plural, one {{count} Artikel} other {{count} Artikel}}"
        );

        let (singular, plural, variable) =
            icu_to_gettext_source(&icu, true).expect("icu should roundtrip");
        assert_eq!(singular, "count Artikel");
        assert_eq!(plural, "count Artikel");
        assert_eq!(variable, "count");

        let mut item = original.clone();
        assert!(normalize_item_to_icu(&mut item, &options));
        assert_eq!(item.msgstr, vec![icu.clone()]);
        assert_eq!(item.msgid_plural.as_deref(), Some(""));

        let mut po = PoFile::new();
        po.items.push(original.clone());

        let normalized = normalize_to_icu(&po, &options);
        assert_eq!(po.items[0].msgstr.len(), 2);
        assert_eq!(normalized.items[0].msgstr, vec![icu.clone()]);

        normalize_to_icu_in_place(&mut po, &options);
        assert_eq!(po.items[0].msgstr, vec![icu]);
    }
}

mod compile {
    use super::*;

    #[test]
    fn crate_root_compile_surface_returns_serialized_payloads() {
        let message = compile_icu("Hallo {name}!", &CompileIcuOptions::new("de"))
            .expect("message should compile");
        match message.kind {
            SerializedCompiledMessageKind::Icu { ast } => assert!(!ast.is_empty()),
            other => panic!("expected icu payload, got {other:?}"),
        }

        let catalog = Catalog::from([(
            String::from("Hello {name}!"),
            CatalogEntry {
                translation: Some(CatalogTranslation::Singular(String::from("Hallo {name}!"))),
                ..CatalogEntry::default()
            },
        )]);

        let compiled = compile_catalog(&catalog, &CompileCatalogOptions::new("de"))
            .expect("catalog should compile");
        let serialized = serialize_compiled_catalog(&catalog, &CompileCatalogOptions::new("de"))
            .expect("catalog should serialize");

        assert_eq!(compiled, serialized);
        match &compiled.entries[0].message.kind {
            SerializedCompiledMessageKind::Icu { ast } => assert!(!ast.is_empty()),
            other => panic!("expected icu payload, got {other:?}"),
        }
    }

    #[test]
    fn compile_uses_entry_message_when_catalog_key_contains_context() {
        let mut item = PoItem::new(2);
        item.msgid = String::from("Open");
        item.msgctxt = Some(String::from("menu.file"));
        item.msgstr = vec![String::from("Öffnen")];

        let catalog = ferrocat::items_to_catalog(&[item], ItemsToCatalogOptions::default())
            .expect("catalog conversion should succeed");
        let hashed = serialize_compiled_catalog(
            &catalog,
            &CompileCatalogOptions {
                locale: String::from("de"),
                use_message_id: true,
                strict: false,
            },
        )
        .expect("catalog should serialize");

        assert_eq!(hashed.entries[0].key, "QWh_hL4_");

        let plain = serialize_compiled_catalog(
            &catalog,
            &CompileCatalogOptions {
                locale: String::from("de"),
                use_message_id: false,
                strict: false,
            },
        )
        .expect("catalog should serialize");

        assert_eq!(plain.entries[0].key, "menu.file\u{0004}Open");
    }

    #[test]
    fn public_catalog_key_strategy_can_still_use_plain_msgid_keys() {
        let mut item = PoItem::new(2);
        item.msgid = String::from("Open");
        item.msgctxt = Some(String::from("menu.file"));
        item.msgstr = vec![String::from("Öffnen")];

        let catalog = ferrocat::items_to_catalog(
            &[item],
            ItemsToCatalogOptions {
                key_strategy: CatalogKeyStrategy::Msgid,
                ..ItemsToCatalogOptions::default()
            },
        )
        .expect("catalog conversion should succeed");

        assert!(catalog.contains_key("Open"));
    }
}
