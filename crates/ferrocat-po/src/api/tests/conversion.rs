use super::*;

use std::path::Path;

#[test]
fn po_fcl_po_roundtrip_preserves_shared_message_metadata() {
    let hash = machine_translation_hash(EffectiveTranslationRef::Singular("Maschinell"));
    let po = format!(
        r#"# Catalog-level note is PO-specific
msgid ""
msgstr ""
"Language: de\n"
"Project-Id-Version: example\n"

# translator note
#. extracted note
#. placeholder {{name}}: user name
#. placeholder {{0}}: first
#. placeholder {{0}}: second
#. placeholder {{0}}: third
#. placeholder {{0}}: fourth
#@ lock: {hash}
#@ ai: openai/gpt-5.5-high:0.95
#: src/app.ts#Greeting
#, fuzzy, x-custom
msgctxt "button"
msgid "Greeting"
msgstr "Maschinell"

#. ICU plural
#: src/files.ts#FileCount
msgid "{{count, plural, one {{# file}} other {{# files}}}}"
msgstr "{{count, plural, one {{# Datei}} other {{# Dateien}}}}"

#~ # old translator note
#~ #. old extracted note
#~ #@ obsolete-since: 2026-07-01
#~ #: src/old.ts#OldView
#~ #, fuzzy
#~ msgid "Old"
#~ msgstr "Alt"
"#
    );

    let fcl = convert_catalog(
        ConvertCatalogOptions::new(&po, "en", CatalogMode::IcuPo, CatalogMode::IcuFcl)
            .with_locale("de"),
    )
    .expect("PO to FCL");
    assert_eq!(fcl.locale.as_deref(), Some("de"));
    assert_eq!(fcl.message_count, 3);
    assert!(
        fcl.content
            .starts_with("%FCL1\tsource=en\tlocale=de\torder=collated\n")
    );
    assert!(
        fcl.content
            .contains("\ttc=translator note\tf=fuzzy\tf=x-custom")
    );
    assert!(fcl.content.contains("\tr=src/app.ts#Greeting"));
    assert!(fcl.content.contains("\tc=extracted note"));
    assert!(fcl.content.contains("\tc=placeholder {name}: user name"));
    assert!(fcl.content.contains("\tc=placeholder {0}: fourth"));
    assert!(fcl.content.contains("\to=2026-07-01"));
    assert!(
        fcl.content
            .contains(&format!("\tlock={hash}\tai=openai/gpt-5.5-high:0.95"))
    );
    assert!(
        fcl.content
            .contains("{count, plural, one {# Datei} other {# Dateien}}")
    );
    assert!(!fcl.content.contains("Catalog-level note"));
    assert!(!fcl.content.contains("Project-Id-Version"));

    let po_again = convert_catalog(
        ConvertCatalogOptions::new(&fcl.content, "en", CatalogMode::IcuFcl, CatalogMode::IcuPo)
            .with_locale("de"),
    )
    .expect("FCL to PO");
    let parsed = parse_po(&po_again.content).expect("parse converted PO");
    assert!(
        parsed
            .headers
            .iter()
            .any(|header| header.key == "Language" && header.value == "de")
    );
    assert_eq!(parsed.items.len(), 3);

    let greeting = parsed
        .items
        .iter()
        .find(|item| item.msgid == "Greeting")
        .expect("greeting");
    assert_eq!(greeting.msgctxt.as_deref(), Some("button"));
    assert_eq!(greeting.msgstr[0], "Maschinell");
    assert_eq!(greeting.comments.as_slice(), ["translator note"]);
    assert_eq!(
        greeting.extracted_comments.as_slice(),
        [
            "extracted note",
            "placeholder {0}: first",
            "placeholder {0}: second",
            "placeholder {0}: third",
            "placeholder {0}: fourth",
            "placeholder {name}: user name",
        ]
    );
    assert_eq!(greeting.references.as_slice(), ["src/app.ts#Greeting"]);
    assert_eq!(greeting.flags.as_slice(), ["fuzzy", "x-custom"]);
    assert!(
        greeting
            .metadata
            .iter()
            .any(|(key, value)| { key == "lock" && value == &hash })
    );
    assert!(
        greeting
            .metadata
            .iter()
            .any(|(key, value)| { key == "ai" && value == "openai/gpt-5.5-high:0.95" })
    );

    let plural = parsed
        .items
        .iter()
        .find(|item| item.msgid.starts_with("{count, plural,"))
        .expect("ICU plural");
    assert_eq!(
        plural.msgstr[0],
        "{count, plural, one {# Datei} other {# Dateien}}"
    );
    assert_eq!(plural.references.as_slice(), ["src/files.ts#FileCount"]);

    let obsolete = parsed
        .items
        .iter()
        .find(|item| item.msgid == "Old")
        .expect("obsolete");
    assert!(obsolete.obsolete);
    assert_eq!(obsolete.comments.as_slice(), ["old translator note"]);
    assert_eq!(
        obsolete.extracted_comments.as_slice(),
        ["old extracted note"]
    );
    assert_eq!(obsolete.references.as_slice(), ["src/old.ts#OldView"]);
    assert_eq!(obsolete.flags.as_slice(), ["fuzzy"]);
    assert!(
        obsolete
            .metadata
            .iter()
            .any(|(key, value)| { key == "obsolete-since" && value == "2026-07-01" })
    );
}

#[test]
fn conversion_rejects_semantic_changes_and_locale_mismatches() {
    let semantic_error = convert_catalog(ConvertCatalogOptions::new(
        "msgid \"file\"\nmsgid_plural \"files\"\nmsgstr[0] \"Datei\"\nmsgstr[1] \"Dateien\"\n",
        "en",
        CatalogMode::GettextPo,
        CatalogMode::IcuFcl,
    ))
    .expect_err("semantic change");
    assert!(matches!(
        semantic_error,
        ApiError::Unsupported(message) if message.contains("changes semantic mode")
    ));

    let locale_error = convert_catalog(
        ConvertCatalogOptions::new(
            "msgid \"\"\nmsgstr \"\"\n\"Language: fr\\n\"\n",
            "en",
            CatalogMode::IcuPo,
            CatalogMode::IcuFcl,
        )
        .with_locale("de"),
    )
    .expect_err("locale mismatch");
    assert!(matches!(
        locale_error,
        ApiError::InvalidArguments(message) if message.contains("did not match expected locale")
    ));
}

#[test]
fn file_conversion_infers_each_format_and_supports_same_path() {
    let directory = tempfile::tempdir().expect("tempdir");
    let po_path = directory.path().join("de.po");
    let fcl_path = directory.path().join("de.fcl");
    fs::write(
        &po_path,
        "msgid \"\"\nmsgstr \"\"\n\"Language: de\\n\"\n\nmsgid \"Hello\"\nmsgstr \"Hallo\"\n",
    )
    .expect("write PO");

    let result = convert_catalog_file(ConvertCatalogFileOptions::new(&po_path, &fcl_path, "en"))
        .expect("file conversion");
    assert_eq!(result.source_format, CatalogFileFormat::Po);
    assert_eq!(result.target_format, CatalogFileFormat::Fcl);
    assert_eq!(result.source_mode, CatalogMode::IcuPo);
    assert_eq!(result.target_mode, CatalogMode::IcuFcl);
    assert_eq!(result.message_count, 1);

    let first_fcl = fs::read_to_string(&fcl_path).expect("read FCL");
    let same_path_po = directory.path().join("same.po");
    fs::copy(&po_path, &same_path_po).expect("copy same-path PO");
    let same_path = convert_catalog_file(
        ConvertCatalogFileOptions::new(&same_path_po, &same_path_po, "en")
            .with_target_format(CatalogFileFormat::Fcl)
            .with_locale("de"),
    )
    .expect("same-path PO to FCL");
    assert_eq!(same_path.message_count, 1);
    assert_eq!(
        fs::read_to_string(&same_path_po).expect("read same-path FCL"),
        first_fcl
    );
}

#[test]
fn file_conversion_failures_leave_existing_output_unchanged() {
    let directory = tempfile::tempdir().expect("tempdir");
    let input = directory.path().join("de.po");
    let output = directory.path().join("de.fcl");
    fs::write(
        &input,
        "msgid \"\"\nmsgstr \"\"\n\"Language: de\\n\"\n\n\
         msgid \"Hello\"\nmsgstr \"Hallo\"\n\n\
         #~ msgid \"Hello\"\n#~ msgstr \"Alt\"\n",
    )
    .expect("write input");
    fs::write(&output, "sentinel").expect("write sentinel");

    let duplicate_error =
        convert_catalog_file(ConvertCatalogFileOptions::new(&input, &output, "en"))
            .expect_err("duplicate FCL identity");
    assert!(matches!(
        duplicate_error,
        ApiError::Conflict(message) if message.contains("duplicate FCL identity")
    ));
    assert_eq!(
        fs::read_to_string(&output).expect("read output"),
        "sentinel"
    );

    let semantic_error = convert_catalog_file(
        ConvertCatalogFileOptions::new(&input, &output, "en")
            .with_source_mode(CatalogMode::GettextPo),
    )
    .expect_err("semantic mismatch");
    assert!(matches!(semantic_error, ApiError::Unsupported(_)));
    assert_eq!(
        fs::read_to_string(&output).expect("read output"),
        "sentinel"
    );

    let invalid_mode = convert_catalog_file(
        ConvertCatalogFileOptions::new(&input, &output, "en").with_source_mode(CatalogMode::IcuFcl),
    )
    .expect_err("format/mode mismatch");
    assert!(matches!(invalid_mode, ApiError::InvalidArguments(_)));
    assert_eq!(
        fs::read_to_string(&output).expect("read output"),
        "sentinel"
    );
}

#[test]
fn file_conversion_accepts_explicit_formats_without_extensions() {
    let directory = tempfile::tempdir().expect("tempdir");
    let input = directory.path().join("source");
    let output = directory.path().join("target");
    fs::write(&input, "msgid \"Hello\"\nmsgstr \"Hallo\"\n").expect("write input");

    let result = convert_catalog_file(
        ConvertCatalogFileOptions::new(&input, &output, "en")
            .with_source_format(CatalogFileFormat::Po)
            .with_target_format(CatalogFileFormat::Fcl)
            .with_locale("de"),
    )
    .expect("explicit formats");
    assert_eq!(result.message_count, 1);
    assert!(
        fs::read_to_string(&output)
            .expect("read output")
            .starts_with("%FCL1\tsource=en\tlocale=de\torder=collated\n")
    );
}

#[test]
fn conversion_propagates_parse_diagnostics() {
    let po = "msgid \"\"\nmsgstr \"\"\n\
              \"Language: de\\n\"\n\
              \"Plural-Forms: nplurals=1; plural=n != 1;\\n\"\n";
    let result = convert_catalog(ConvertCatalogOptions::new(
        po,
        "en",
        CatalogMode::GettextPo,
        CatalogMode::GettextPo,
    ))
    .expect("gettext canonicalization");
    assert!(!result.diagnostics.is_empty());
}

#[test]
fn conversion_canonicalizes_multiline_named_placeholder_comments() {
    let fcl = "%FCL1\tsource=en\tlocale=de\torder=collated\n\
               Greeting\t\tHallo\tc=placeholder {name}: first\\nsecond\n";
    let result = convert_catalog(ConvertCatalogOptions::new(
        fcl,
        "en",
        CatalogMode::IcuFcl,
        CatalogMode::IcuPo,
    ))
    .expect("convert multiline placeholder");
    assert!(
        result
            .content
            .contains("#. placeholder {name}: first second\n")
    );
    assert!(!result.content.contains("first\nsecond"));
}

#[test]
fn file_option_paths_are_reported_on_io_errors() {
    let missing = Path::new("definitely-missing.po");
    let output = Path::new("unused.fcl");
    let error = convert_catalog_file(ConvertCatalogFileOptions::new(missing, output, "en"))
        .expect_err("missing input");
    assert_eq!(error.path(), Some(missing));
}
