#![cfg(feature = "catalog")]

use ferrocat_po::{
    CatalogCombineInput, CatalogMode, CatalogUpdateInput, CombineCatalogOptions,
    ConvertCatalogOptions, ExtractedMessage, ExtractedPluralMessage, MergeCatalogsThreeWayOptions,
    PluralSource, UpdateCatalogOptions, combine_catalogs, convert_catalog,
    merge_catalogs_three_way, parse_po, update_catalog,
};

fn po(slots: &str, header: &str) -> String {
    format!(
        "msgid \"\"\nmsgstr \"\"\n\"Language: de\\n\"\n{header}\nmsgid \"book\"\nmsgid_plural \"books\"\n{slots}"
    )
}

fn update(input: &str) -> String {
    let message = ExtractedMessage::Plural(ExtractedPluralMessage {
        msgid: "book".into(),
        source: PluralSource {
            one: Some("book".into()),
            other: "books".into(),
        },
        ..ExtractedPluralMessage::default()
    });
    update_catalog(
        UpdateCatalogOptions::new("en", CatalogUpdateInput::Structured(vec![message]))
            .with_mode(CatalogMode::GettextPo)
            .with_locale("de")
            .with_existing(input),
    )
    .expect("catalog operation succeeds")
    .content
}

fn outputs(input: &str) -> Vec<String> {
    let inputs = [CatalogCombineInput::new(input)];
    let side = CatalogCombineInput::new(input);
    vec![
        update(input),
        convert_catalog(ConvertCatalogOptions::new(
            input,
            "en",
            CatalogMode::GettextPo,
            CatalogMode::GettextPo,
        ))
        .expect("catalog operation succeeds")
        .content,
        combine_catalogs(
            CombineCatalogOptions::new(&inputs, "en").with_mode(CatalogMode::GettextPo),
        )
        .expect("catalog operation succeeds")
        .content,
        merge_catalogs_three_way(
            MergeCatalogsThreeWayOptions::new(side, side, side, "en")
                .with_mode(CatalogMode::GettextPo),
        )
        .expect("catalog operation succeeds")
        .content,
    ]
}

#[test]
fn header_defaults_preserve_unmatched_plural_slots_across_operations() {
    let input = po(
        "msgstr[0] \"first\"\nmsgstr[1] \"second\"\nmsgstr[2] \"third\"\n",
        "",
    );
    for output in outputs(&input) {
        let parsed = parse_po(&output).expect("catalog operation succeeds");
        assert!(!parsed.headers.iter().any(|h| h.key == "Plural-Forms"));
        assert!(update(&output).contains("third"));
    }
}

#[test]
fn matching_slots_get_default_and_explicit_header_is_preserved() {
    let slots = "msgstr[0] \"first\"\nmsgstr[1] \"second\"\n";
    for output in outputs(&po(slots, "")) {
        assert!(output.contains("nplurals=2"));
    }
    let header = "\"Plural-Forms: nplurals=3; plural=(n != 1);\\n\"\n";
    for output in outputs(&po(slots, header)) {
        assert!(output.contains("nplurals=3; plural=(n != 1);"));
    }
}
