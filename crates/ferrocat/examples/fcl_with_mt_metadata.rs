use ferrocat::{
    CatalogMode, CatalogUpdateInput, EffectiveTranslationRef, SourceExtractedMessage,
    UpdateCatalogOptions, machine_translation_hash, update_catalog,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let hash = machine_translation_hash(EffectiveTranslationRef::Singular("Hallo"));
    let existing = format!(
        "%FCL1\tsource=en\tlocale=de\nHello\t\tHallo\tlock={hash}\tai=example/mt:0.92\n",
        hash = hash
    );

    let input = CatalogUpdateInput::SourceFirst(vec![SourceExtractedMessage {
        msgid: "Hello".to_owned(),
        ..SourceExtractedMessage::default()
    }]);
    let result = update_catalog(
        UpdateCatalogOptions::new("en", input)
            .with_locale("de")
            .with_mode(CatalogMode::IcuFcl)
            .with_existing(&existing),
    )?;

    assert!(result.content.starts_with("%FCL1"));
    assert!(result.content.contains("ai=example/mt:0.92"));
    assert!(result.content.contains(&hash));

    println!("{}", result.content);
    Ok(())
}
