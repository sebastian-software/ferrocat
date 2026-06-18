use ferrocat::{
    CompileCatalogArtifactOptions, NormalizedParsedCatalog, ParseCatalogOptions,
    compile_catalog_artifact, parse_catalog,
};

fn catalog(
    content: &str,
    locale: &str,
) -> Result<NormalizedParsedCatalog, Box<dyn std::error::Error>> {
    Ok(parse_catalog(ParseCatalogOptions {
        content,
        locale: Some(locale),
        source_locale: "en",
        ..ParseCatalogOptions::new("", "en")
    })?
    .into_normalized_view()?)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = catalog(
        "msgid \"Checkout\"\nmsgstr \"Checkout\"\n\nmsgid \"Cart\"\nmsgstr \"Cart\"\n",
        "en",
    )?;
    let german = catalog("msgid \"Checkout\"\nmsgstr \"Zur Kasse\"\n", "de")?;

    let options = CompileCatalogArtifactOptions {
        source_fallback: true,
        ..CompileCatalogArtifactOptions::new("de", "en")
    };
    let artifact = compile_catalog_artifact(&[&source, &german], &options)?;

    assert_eq!(artifact.messages.len(), 2);
    assert_eq!(artifact.missing.len(), 1);
    assert!(artifact.messages.values().any(|value| value == "Zur Kasse"));
    assert!(artifact.messages.values().any(|value| value == "Cart"));

    for (key, value) in &artifact.messages {
        println!("{key} = {value}");
    }

    Ok(())
}
