use std::borrow::Cow;

use ferrocat::{MergeExtractedMessage, merge_catalog};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let existing = r#"
msgid "Hello"
msgstr "Hallo"

msgid "Old CTA"
msgstr "Alter CTA"
"#;

    let extracted = [
        MergeExtractedMessage {
            msgid: Cow::Borrowed("Hello"),
            references: vec![Cow::Borrowed("src/app.rs:10")],
            ..MergeExtractedMessage::default()
        },
        MergeExtractedMessage {
            msgid: Cow::Borrowed("Checkout"),
            references: vec![Cow::Borrowed("src/checkout.rs:42")],
            extracted_comments: vec![Cow::Borrowed("Primary checkout button")],
            ..MergeExtractedMessage::default()
        },
    ];

    let merged = merge_catalog(existing, &extracted)?;

    assert!(merged.contains("msgstr \"Hallo\""));
    assert!(merged.contains("msgid \"Checkout\""));
    assert!(merged.contains("#~ msgid \"Old CTA\""));

    println!("{merged}");
    Ok(())
}
