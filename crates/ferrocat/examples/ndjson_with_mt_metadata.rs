use ferrocat::{
    CatalogMode, CatalogUpdateInput, EffectiveTranslationRef, SourceExtractedMessage,
    UpdateCatalogOptions, machine_translation_hash, update_catalog,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let hash = machine_translation_hash(EffectiveTranslationRef::Singular("Hallo"));
    let existing = format!(
        concat!(
            "---\n",
            "format: ferrocat.ndjson.v1\n",
            "locale: de\n",
            "source_locale: en\n",
            "---\n",
            "{{\"id\":\"Hello\",\"str\":\"Hallo\",\"mt\":{{\"model\":\"example/mt\",\"confidence\":92,\"hash\":\"{hash}\"}}}}\n"
        ),
        hash = hash
    );

    let result = update_catalog(UpdateCatalogOptions {
        locale: Some("de"),
        mode: CatalogMode::IcuNdjson,
        existing: Some(&existing),
        input: CatalogUpdateInput::SourceFirst(vec![SourceExtractedMessage {
            msgid: "Hello".to_owned(),
            ..SourceExtractedMessage::default()
        }]),
        ..UpdateCatalogOptions::new("en", CatalogUpdateInput::default())
    })?;

    assert!(result.content.contains("format: ferrocat.ndjson.v1"));
    assert!(result.content.contains("\"model\":\"example/mt\""));
    assert!(result.content.contains(&hash));

    println!("{}", result.content);
    Ok(())
}
