#[test]
fn fallback_code_type_matches_ferrocat_icu_definition() {
    let fallback = sync_block(include_str!("../src/diagnostic_codes.rs"));
    let canonical = sync_block(include_str!("../../ferrocat-icu/src/diagnostic_codes.rs"));

    assert_eq!(
        fallback, canonical,
        "the fallback DiagnosticCode in ferrocat-po drifted from ferrocat-icu; \
         apply the change inside both sync(diagnostic-code-type) blocks"
    );
}

/// Extracts the marked block, normalizing the module indentation of the
/// fallback copy away so both sides compare structurally.
fn sync_block(source: &str) -> Vec<&str> {
    let block: Vec<&str> = source
        .lines()
        .skip_while(|line| !line.contains("sync(diagnostic-code-type): begin"))
        .skip(1)
        .take_while(|line| !line.contains("sync(diagnostic-code-type): end"))
        .map(str::trim_start)
        .collect();
    assert!(
        !block.is_empty(),
        "sync(diagnostic-code-type) markers missing"
    );
    block
}
