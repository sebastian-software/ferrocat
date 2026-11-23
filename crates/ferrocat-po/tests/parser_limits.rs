use ferrocat_po::{merge_catalog, parse_po, parse_po_borrowed, parse_po_bytes};

fn check_all(input: &str, valid: bool) {
    let results = [
        parse_po(input).map(|_| ()),
        parse_po_borrowed(input).map(|_| ()),
        parse_po_bytes(input.as_bytes()).map(|_| ()),
        merge_catalog(input, &[]).map(|_| ()),
    ];
    for result in results {
        if valid {
            assert!(result.is_ok(), "{result:?}");
        } else {
            let error = result.expect_err("input exceeds the plural slot limit");
            assert!(
                error
                    .to_string()
                    .contains("plural slot count exceeds the limit of 128")
            );
            assert!(error.position().is_some());
        }
    }
}

#[test]
fn plural_index_boundaries() {
    for (index, valid) in [
        ("127", true),
        ("128", false),
        ("184467440737095516160", false),
    ] {
        check_all(
            &format!("msgid \"item\"\nmsgid_plural \"items\"\nmsgstr[{index}] \"value\"\n"),
            valid,
        );
    }
    check_all("msgid \"item [128]\"\nmsgstr \"value [128]\"\n", true);
}

#[test]
fn plural_header_boundaries() {
    for (width, valid) in [
        ("128", true),
        ("129", false),
        ("184467440737095516160", false),
        ("unset", true),
    ] {
        check_all(
            &format!(
                "msgid \"\"\nmsgstr \"\"\n\"Plural-Forms: nplurals={width}; plural=0;\\n\"\n\nmsgid \"item\"\nmsgid_plural \"items\"\nmsgstr \"value\"\n"
            ),
            valid,
        );
    }
}

#[test]
fn header_at_eof_and_decimal_spelling() {
    for width in ["129", "+129", "000129"] {
        check_all(
            &format!("msgid \"\"\nmsgstr \"Plural-Forms: nplurals={width};\\n\"\n"),
            false,
        );
    }
    for width in ["+128", "000128"] {
        check_all(
            &format!("msgid \"\"\nmsgstr \"Plural-Forms: nplurals={width};\\n\"\n"),
            true,
        );
    }
    check_all("msgid \"x\"\nmsgstr[+000127] \"x\"\n", true);
}
