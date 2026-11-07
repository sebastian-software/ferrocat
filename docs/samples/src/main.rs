//! The real output the ferrocat.dev homepage shows.
//!
//! This program depends on the published `ferrocat` crate from crates.io, at
//! the exact version in `Cargo.toml`, and prints one JSON document:
//! a catalog update, the same catalog as PO and as FCL, ICU checks, a review
//! after a human correction, and a compiled runtime artifact.
//! `pnpm sample:write` in `docs/` writes it to `app/data/home-samples.json`;
//! `pnpm sample:check` compares it with the committed file.

use ferrocat::catalog::{
    CatalogMessageKey, CatalogMode, CatalogOrigin, CatalogReviewOptions, CatalogUpdateInput,
    CompileCatalogArtifactOptions, ConvertCatalogOptions, EffectiveTranslationRef,
    ParseCatalogOptions, SourceExtractedMessage, UpdateCatalogOptions, compile_catalog_artifact,
    convert_catalog, machine_translation_hash, parse_catalog, parse_catalog_for_review,
    review_catalogs, update_catalog,
};
use ferrocat::icu::{
    IcuCompatibilityOptions, IcuDiagnosticSeverity, compare_icu_messages, parse_icu,
};
use serde_json::{Value, json};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// The day every update is stamped with, so the output stays reproducible.
const TODAY: &str = "2026-11-07";
const SOURCE_LOCALE: &str = "en";
const LOCALE: &str = "de";
const CART: &str = "{count, plural, one {# item} other {# items}} in your cart";
const CART_DE: &str = "{count, plural, one {# Artikel} other {# Artikel}} im Warenkorb";

/// One message as an extractor reports it from the application's code.
fn extracted(
    msgid: &str,
    msgctxt: Option<&str>,
    file: &str,
    scope: &str,
    comment: Option<&str>,
) -> SourceExtractedMessage {
    SourceExtractedMessage {
        msgid: msgid.to_owned(),
        msgctxt: msgctxt.map(str::to_owned),
        comments: comment.map(str::to_owned).into_iter().collect(),
        origin: vec![CatalogOrigin {
            file: file.to_owned(),
            scope: Some(scope.to_owned()),
        }],
        ..SourceExtractedMessage::default()
    }
}

fn greeting() -> SourceExtractedMessage {
    extracted(
        "Hello {name}",
        None,
        "src/Header.tsx",
        "Greeting",
        Some("Shown after sign-in"),
    )
}

fn pay_now() -> SourceExtractedMessage {
    extracted(
        "Pay now",
        Some("checkout"),
        "src/Checkout.tsx",
        "PayButton",
        Some("Primary checkout button"),
    )
}

fn cart() -> SourceExtractedMessage {
    extracted(CART, None, "src/Cart.tsx", "CartSummary", None)
}

/// The JSON shape the page shows for an extracted message.
fn describe(message: &SourceExtractedMessage) -> Value {
    let origin = &message.origin[0];
    json!({
        "msgid": message.msgid,
        "msgctxt": message.msgctxt,
        "origin": format!("{}#{}", origin.file, origin.scope.as_deref().unwrap_or_default()),
    })
}

fn update(
    mode: CatalogMode,
    existing: &str,
    messages: Vec<SourceExtractedMessage>,
) -> Result<String> {
    let result = update_catalog(
        UpdateCatalogOptions::new(SOURCE_LOCALE, CatalogUpdateInput::SourceFirst(messages))
            .with_locale(LOCALE)
            .with_mode(mode)
            .with_existing(existing)
            .with_now(TODAY),
    )?;
    Ok(result.content)
}

/// Last release's German catalog, then the update after the code changed.
fn catalog_update() -> Result<Value> {
    let translated = "msgid \"Hello {name}\"\nmsgstr \"Hallo {name}\"\n\n\
                      msgctxt \"checkout\"\nmsgid \"Pay now\"\nmsgstr \"Jetzt bezahlen\"\n\n\
                      msgid \"Continue shopping\"\nmsgstr \"Weiter einkaufen\"\n";
    let shopping = extracted(
        "Continue shopping",
        None,
        "src/Cart.tsx",
        "CartSummary",
        None,
    );
    let before = update(
        CatalogMode::IcuPo,
        translated,
        vec![greeting(), pay_now(), shopping],
    )?;
    let now = vec![greeting(), pay_now(), cart()];
    let extracted_now: Vec<Value> = now.iter().map(describe).collect();
    let result = update_catalog(
        UpdateCatalogOptions::new(SOURCE_LOCALE, CatalogUpdateInput::SourceFirst(now))
            .with_locale(LOCALE)
            .with_mode(CatalogMode::IcuPo)
            .with_existing(&before)
            .with_now(TODAY),
    )?;
    let stats = &result.stats;
    Ok(json!({
        "before": before,
        "extracted": extracted_now,
        "after": result.content,
        "stats": {
            "total": stats.total,
            "added": stats.added,
            "changed": stats.changed,
            "unchanged": stats.unchanged,
            "obsoleteMarked": stats.obsolete_marked,
        },
    }))
}

/// The finished German catalog, written as PO and converted to FCL.
fn storage() -> Result<Value> {
    let translated = format!(
        "msgid \"Hello {{name}}\"\nmsgstr \"Hallo {{name}}\"\n\n\
         msgctxt \"checkout\"\nmsgid \"Pay now\"\nmsgstr \"Jetzt bezahlen\"\n\n\
         msgid \"{CART}\"\nmsgstr \"{CART_DE}\"\n"
    );
    let po = update(
        CatalogMode::IcuPo,
        &translated,
        vec![greeting(), pay_now(), cart()],
    )?;
    let fcl = convert_catalog(
        ConvertCatalogOptions::new(&po, SOURCE_LOCALE, CatalogMode::IcuPo, CatalogMode::IcuFcl)
            .with_locale(LOCALE),
    )?;
    Ok(json!({ "po": po, "fcl": fcl.content, "messages": fcl.message_count }))
}

fn severity(severity: IcuDiagnosticSeverity) -> &'static str {
    match severity {
        IcuDiagnosticSeverity::Error => "error",
        IcuDiagnosticSeverity::Warning => "warning",
        IcuDiagnosticSeverity::Info => "info",
    }
}

/// A translation compared with its source message, as a translator might write it.
fn icu_checks() -> Result<Value> {
    let source = parse_icu(CART)?;
    let attempts = [
        ("kept", "Structure kept", CART_DE),
        (
            "translated",
            "Placeholder translated",
            "{anzahl, plural, one {# Artikel} other {# Artikel}} im Warenkorb",
        ),
        (
            "flattened",
            "Plural left out",
            "{count} Artikel im Warenkorb",
        ),
    ];
    let mut cases = Vec::new();
    for (id, label, text) in attempts {
        let translation = parse_icu(text)?;
        let report =
            compare_icu_messages(&source, &translation, &IcuCompatibilityOptions::default());
        let diagnostics: Vec<Value> = report
            .diagnostics
            .iter()
            .map(|diagnostic| {
                json!({
                    "severity": severity(diagnostic.severity),
                    "code": diagnostic.code.as_str(),
                    "message": diagnostic.message,
                    "name": diagnostic.name,
                })
            })
            .collect();
        cases.push(json!({
            "id": id,
            "label": label,
            "translation": text,
            "hasErrors": report.has_errors(),
            "diagnostics": diagnostics,
        }));
    }
    Ok(json!({ "source": CART, "cases": cases }))
}

/// A machine translation with its fingerprint, then a reviewer's correction.
fn review() -> Result<Value> {
    let machine = "Jetzt bezahlen";
    let corrected = "Zahlungspflichtig bestellen";
    let lock = machine_translation_hash(EffectiveTranslationRef::Singular(machine));
    let seeded = format!(
        "%FCL1\tsource=en\tlocale=de\nPay now\tcheckout\t{machine}\tlock={lock}\tai=example/mt:0.92\n"
    );
    let before = update(CatalogMode::IcuFcl, &seeded, vec![pay_now()])?;
    let after = before.replace(&format!("\t{machine}\t"), &format!("\t{corrected}\t"));
    assert_ne!(
        before, after,
        "the correction replaces the machine translation"
    );

    let source_po = "msgctxt \"checkout\"\nmsgid \"Pay now\"\nmsgstr \"Pay now\"\n";
    let for_review = |content: &str, locale: &str, mode: CatalogMode| {
        parse_catalog_for_review(
            ParseCatalogOptions::new(content, SOURCE_LOCALE)
                .with_locale(locale)
                .with_mode(mode),
        )
    };
    let source = for_review(source_po, SOURCE_LOCALE, CatalogMode::IcuPo)?;
    let previous = for_review(&before, LOCALE, CatalogMode::IcuFcl)?;
    let current = for_review(&after, LOCALE, CatalogMode::IcuFcl)?;
    let report = review_catalogs(
        &[&source, &previous],
        &[&source, &current],
        &CatalogReviewOptions::new(SOURCE_LOCALE).with_details(true),
    )?;
    let locale = &report.locales[0];
    Ok(json!({
        "machine": machine,
        "corrected": corrected,
        "before": before,
        "after": after,
        "summary": serde_json::to_value(&report.summary)?,
        "translations": serde_json::to_value(&locale.translations)?,
        "machineTranslation": serde_json::to_value(&locale.machine_translation)?,
    }))
}

/// The German runtime artifact, where one message has no translation yet.
fn runtime() -> Result<Value> {
    let catalog = |content: &str, locale: &str| {
        parse_catalog(
            ParseCatalogOptions::new(content, SOURCE_LOCALE)
                .with_locale(locale)
                .with_mode(CatalogMode::IcuPo),
        )?
        .into_normalized_view()
    };
    let source_po = format!(
        "msgid \"Hello {{name}}\"\nmsgstr \"Hello {{name}}\"\n\n\
         msgctxt \"checkout\"\nmsgid \"Pay now\"\nmsgstr \"Pay now\"\n\n\
         msgid \"{CART}\"\nmsgstr \"{CART}\"\n"
    );
    let german_po = "msgid \"Hello {name}\"\nmsgstr \"Hallo {name}\"\n\n\
                     msgctxt \"checkout\"\nmsgid \"Pay now\"\nmsgstr \"Jetzt bezahlen\"\n";
    let source = catalog(&source_po, SOURCE_LOCALE)?;
    let german = catalog(german_po, LOCALE)?;
    let options =
        CompileCatalogArtifactOptions::new(LOCALE, SOURCE_LOCALE).with_source_fallback(true);
    let artifact = compile_catalog_artifact(&[&source, &german], &options)?;
    let keys: Vec<Value> = [
        CatalogMessageKey::new("Hello {name}", None),
        CatalogMessageKey::with_context("Pay now", "checkout"),
        CatalogMessageKey::new(CART, None),
    ]
    .iter()
    .map(|key| {
        json!({
            "msgid": key.msgid,
            "msgctxt": key.msgctxt,
            "key": ferrocat::catalog::compiled_key(&key.msgid, key.msgctxt.as_deref()),
        })
    })
    .collect();
    Ok(json!({
        "source": source_po,
        "german": german_po,
        "keys": keys,
        "artifact": serde_json::to_value(&artifact)?,
    }))
}

fn main() -> Result<()> {
    let output = json!({
        "update": catalog_update()?,
        "storage": storage()?,
        "icu": icu_checks()?,
        "review": review()?,
        "runtime": runtime()?,
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
