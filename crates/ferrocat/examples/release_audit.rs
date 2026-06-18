use ferrocat::{CatalogAuditOptions, NormalizedParsedCatalog, ParseCatalogOptions, audit_catalogs};

fn catalog(
    content: &str,
    locale: &str,
) -> Result<NormalizedParsedCatalog, Box<dyn std::error::Error>> {
    Ok(ferrocat::parse_catalog(ParseCatalogOptions {
        content,
        locale: Some(locale),
        source_locale: "en",
        ..ParseCatalogOptions::new("", "en")
    })?
    .into_normalized_view()?)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = catalog("msgid \"Checkout\"\nmsgstr \"Checkout\"\n", "en")?;
    let target = catalog("msgid \"Checkout\"\nmsgstr \"\"\n", "de")?;

    let report = audit_catalogs(&[&source, &target], &CatalogAuditOptions::new("en"))?;

    assert!(report.has_errors());
    assert!(
        report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "catalog.empty_translation")
    );

    for diagnostic in &report.diagnostics {
        println!("{}: {}", diagnostic.code, diagnostic.message);
    }

    Ok(())
}
