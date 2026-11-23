#![cfg(feature = "catalog")]

use ferrocat_po::{
    ApiError, CatalogCombineInput, CatalogMode, CatalogUpdateInput, CombineCatalogOptions,
    ConvertCatalogOptions, MergeCatalogsThreeWayOptions, ParseCatalogOptions,
    UpdateCatalogFileOptions, UpdateCatalogOptions, combine_catalogs, convert_catalog,
    merge_catalogs_three_way, parse_catalog, parse_po, update_catalog, update_catalog_file,
};

fn input(surplus: &str) -> String {
    format!(
        "msgid \"\"\nmsgstr \"\"\n\"Language: de\\n\"\n\"Plural-Forms: nplurals=2; plural=(n != 1);\\n\"\n\nmsgid \"book\"\nmsgid_plural \"books\"\nmsgstr[0] \"first\"\nmsgstr[1] \"second\"\nmsgstr[2] \"{surplus}\"\n"
    )
}

fn update_options() -> UpdateCatalogOptions<'static> {
    UpdateCatalogOptions::new("en", CatalogUpdateInput::default()).with_mode(CatalogMode::GettextPo)
}

fn check_error(error: ApiError) {
    assert!(matches!(error, ApiError::Unsupported(_)));
    let message = error.to_string();
    assert!(message.contains("book"), "{message}");
    assert!(message.contains("msgstr[2]"), "{message}");
    assert!(message.contains("slot count 2"), "{message}");
}

#[test]
fn surplus_nonempty_slots_are_readable_but_writes_fail() {
    let po = input("third");
    assert!(parse_po(&po).is_ok());
    assert!(
        parse_catalog(ParseCatalogOptions::new(&po, "en").with_mode(CatalogMode::GettextPo))
            .is_ok()
    );
    check_error(
        update_catalog(update_options().with_existing(&po)).expect_err("update rejects loss"),
    );
    check_error(
        convert_catalog(ConvertCatalogOptions::new(
            &po,
            "en",
            CatalogMode::GettextPo,
            CatalogMode::GettextPo,
        ))
        .expect_err("conversion rejects loss"),
    );
    let inputs = [CatalogCombineInput::new(&po)];
    check_error(
        combine_catalogs(
            CombineCatalogOptions::new(&inputs, "en").with_mode(CatalogMode::GettextPo),
        )
        .expect_err("combine rejects loss"),
    );
    let side = CatalogCombineInput::new(&po);
    check_error(
        merge_catalogs_three_way(
            MergeCatalogsThreeWayOptions::new(side, side, side, "en")
                .with_mode(CatalogMode::GettextPo),
        )
        .expect_err("merge rejects loss"),
    );

    let dir = tempfile::tempdir().expect("temporary directory");
    let path = dir.path().join("catalog.po");
    std::fs::write(&path, &po).expect("write fixture");
    check_error(
        update_catalog_file(
            UpdateCatalogFileOptions::new(&path, "en", CatalogUpdateInput::default())
                .with_options(update_options()),
        )
        .expect_err("file update rejects loss"),
    );
    assert_eq!(std::fs::read_to_string(path).expect("read original"), po);
}

#[test]
fn surplus_empty_slots_are_safe_to_normalize() {
    let po = input("");
    assert!(update_catalog(update_options().with_existing(&po)).is_ok());
    assert!(
        convert_catalog(ConvertCatalogOptions::new(
            &po,
            "en",
            CatalogMode::GettextPo,
            CatalogMode::GettextPo
        ))
        .is_ok()
    );
    let inputs = [CatalogCombineInput::new(&po)];
    assert!(
        combine_catalogs(
            CombineCatalogOptions::new(&inputs, "en").with_mode(CatalogMode::GettextPo)
        )
        .is_ok()
    );
    let side = CatalogCombineInput::new(&po);
    assert!(
        merge_catalogs_three_way(
            MergeCatalogsThreeWayOptions::new(side, side, side, "en")
                .with_mode(CatalogMode::GettextPo)
        )
        .is_ok()
    );
}

#[test]
fn headerless_catalog_rejects_values_beyond_the_supported_profile() {
    let po = "msgid \"\"\nmsgstr \"\"\n\"Language: de\\n\"\n\nmsgid \"book\"\nmsgid_plural \"books\"\nmsgstr[6] \"seventh\"\n";
    assert!(
        parse_catalog(ParseCatalogOptions::new(po, "en").with_mode(CatalogMode::GettextPo)).is_ok()
    );
    let error = update_catalog(update_options().with_existing(po))
        .expect_err("seventh slot is unsupported");
    assert!(matches!(error, ApiError::Unsupported(_)));
    assert!(error.to_string().contains("msgstr[6]"));
}
