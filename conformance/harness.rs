#![allow(dead_code)]

use std::borrow::Cow;

use ferrocat_conformance::{
    ConformanceCase, ConformanceManifest, Expectation, ExpectedArtifact, IcuParseExpected,
    IcuRejectExpected, PoItemExpected, PoParseExpected, load_all_manifests, read_fixture_text,
};
use ferrocat_icu::{IcuNode, IcuPluralKind, parse_icu};
use ferrocat_po::{
    MergeExtractedMessage, ParseCatalogOptions, PluralEncoding, PoFile, PoItem, SerializeOptions,
    merge_catalog, parse_catalog, parse_po, stringify_po,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvaluationStatus {
    Matched,
    Failed,
    KnownGap,
}

#[derive(Debug, Clone)]
pub struct CaseEvaluation {
    pub suite: String,
    pub capability: String,
    pub case_id: String,
    pub expectation: Expectation,
    pub status: EvaluationStatus,
    pub detail: String,
}

#[derive(Debug, Clone, Default)]
pub struct EvaluationSummary {
    pub total: usize,
    pub pass: usize,
    pub reject: usize,
    pub known_gap: usize,
    pub failures: Vec<CaseEvaluation>,
}

pub fn evaluate_all_cases() -> Result<Vec<CaseEvaluation>, String> {
    let manifests = load_all_manifests().map_err(|error| error.to_string())?;
    Ok(evaluate_manifests(&manifests))
}

pub fn evaluate_manifests(manifests: &[ConformanceManifest]) -> Vec<CaseEvaluation> {
    let mut evaluations = Vec::new();
    for manifest in manifests {
        for case in &manifest.cases {
            evaluations.push(evaluate_case(manifest, case));
        }
    }
    evaluations
}

pub fn summarize_evaluations(evaluations: &[CaseEvaluation]) -> EvaluationSummary {
    let mut summary = EvaluationSummary::default();
    for evaluation in evaluations {
        summary.total += 1;
        match evaluation.expectation {
            Expectation::Pass => summary.pass += 1,
            Expectation::Reject => summary.reject += 1,
            Expectation::KnownGap => summary.known_gap += 1,
        }
        if evaluation.status == EvaluationStatus::Failed {
            summary.failures.push(evaluation.clone());
        }
    }
    summary
}

pub fn evaluate_suite(suite: &str) -> Result<Vec<CaseEvaluation>, String> {
    let manifests = load_all_manifests().map_err(|error| error.to_string())?;
    let filtered = manifests
        .into_iter()
        .filter(|manifest| manifest.suite == suite)
        .collect::<Vec<_>>();
    Ok(evaluate_manifests(&filtered))
}

pub fn failure_messages(evaluations: &[CaseEvaluation]) -> Vec<String> {
    evaluations
        .iter()
        .filter(|evaluation| evaluation.status == EvaluationStatus::Failed)
        .map(|evaluation| {
            format!(
                "[{}:{}:{}] {}",
                evaluation.suite, evaluation.capability, evaluation.case_id, evaluation.detail
            )
        })
        .collect()
}

fn evaluate_case(manifest: &ConformanceManifest, case: &ConformanceCase) -> CaseEvaluation {
    if case.expectation == Expectation::KnownGap {
        return CaseEvaluation {
            suite: manifest.suite.clone(),
            capability: case.capability.clone(),
            case_id: case.id.clone(),
            expectation: case.expectation.clone(),
            status: EvaluationStatus::KnownGap,
            detail: case
                .notes
                .clone()
                .unwrap_or_else(|| "classified as known gap".to_owned()),
        };
    }

    let detail_result = match case.runner.as_str() {
        "po_parse" => evaluate_po_parse(case),
        "po_roundtrip" => evaluate_po_roundtrip(case),
        "po_reject" => evaluate_po_reject(case),
        "po_merge" => evaluate_po_merge(case),
        "po_plural_header" => evaluate_po_plural_header(case),
        "icu_parse" => evaluate_icu_parse(case),
        "icu_reject" => evaluate_icu_reject(case),
        other => Err(format!("unknown runner `{other}` for case {}", case.id)),
    };

    let (status, detail) = match detail_result {
        Ok(detail) if detail.is_empty() => {
            (EvaluationStatus::Matched, "matched expectation".to_owned())
        }
        Ok(detail) | Err(detail) => (EvaluationStatus::Failed, detail),
    };

    CaseEvaluation {
        suite: manifest.suite.clone(),
        capability: case.capability.clone(),
        case_id: case.id.clone(),
        expectation: case.expectation.clone(),
        status,
        detail,
    }
}

fn evaluate_po_parse(case: &ConformanceCase) -> Result<String, String> {
    let input = read_fixture_text(&case.input).map_err(|error| error.to_string())?;
    let expected = load_po_parse_expected(case)?;
    let parsed = parse_po(&input).map_err(|error| format!("parse failed unexpectedly: {error}"))?;
    compare_po_parse(&parsed, &expected, case.item_start_index.unwrap_or(0))
}

fn evaluate_po_roundtrip(case: &ConformanceCase) -> Result<String, String> {
    let input = read_fixture_text(&case.input).map_err(|error| error.to_string())?;
    let expected_path = case
        .expected_fixture_path()
        .ok_or_else(|| format!("roundtrip case {} is missing expected fixture", case.id))?;
    let expected = read_fixture_text(expected_path).map_err(|error| error.to_string())?;
    let parsed = parse_po(&input).map_err(|error| format!("parse failed unexpectedly: {error}"))?;
    let options = SerializeOptions {
        fold_length: case.fold_length.unwrap_or(80),
        compact_multiline: case.compact_multiline.unwrap_or(true),
    };
    let rendered = stringify_po(&parsed, &options);
    if equivalent_text(&rendered, &expected) {
        Ok(String::new())
    } else {
        Err(format!(
            "rendered output mismatch for {}\nexpected:\n{}\nactual:\n{}",
            case.id, expected, rendered
        ))
    }
}

fn evaluate_po_reject(case: &ConformanceCase) -> Result<String, String> {
    let input = read_fixture_text(&case.input).map_err(|error| error.to_string())?;
    let expected = match case
        .expected_artifact()
        .map_err(|error| error.to_string())?
    {
        ExpectedArtifact::PoReject(expected) => expected,
        other => {
            return Err(format!(
                "case {} expected po_reject artifact, got {:?}",
                case.id, other
            ));
        }
    };

    match parse_po(&input) {
        Ok(_) => Err("parse unexpectedly succeeded".to_owned()),
        Err(error) if error.to_string().contains(&expected.message_contains) => Ok(String::new()),
        Err(error) => Err(format!("parse failed with unexpected message: {error}")),
    }
}

fn evaluate_po_merge(case: &ConformanceCase) -> Result<String, String> {
    let existing = read_fixture_text(&case.input).map_err(|error| error.to_string())?;
    let template_path = case
        .companion_input
        .as_deref()
        .ok_or_else(|| format!("merge case {} is missing companion_input", case.id))?;
    let template = read_fixture_text(template_path).map_err(|error| error.to_string())?;
    let expected_path = case
        .expected_fixture_path()
        .ok_or_else(|| format!("merge case {} is missing expected output", case.id))?;
    let expected = read_fixture_text(expected_path).map_err(|error| error.to_string())?;

    let extracted = parse_po(&template)
        .map_err(|error| format!("parse template failed: {error}"))?
        .items
        .into_iter()
        .map(|item| MergeExtractedMessage {
            msgctxt: item.msgctxt.map(Cow::Owned),
            msgid: Cow::Owned(item.msgid),
            msgid_plural: item.msgid_plural.map(Cow::Owned),
            references: item.references.into_iter().map(Cow::Owned).collect(),
            extracted_comments: item
                .extracted_comments
                .into_iter()
                .map(Cow::Owned)
                .collect(),
            flags: item.flags.into_iter().map(Cow::Owned).collect(),
        })
        .collect::<Vec<_>>();

    let rendered = merge_catalog(&existing, &extracted).map_err(|error| error.to_string())?;
    if equivalent_text(&rendered, &expected) {
        Ok(String::new())
    } else {
        Err(format!(
            "merge output mismatch for {}\nexpected:\n{}\nactual:\n{}",
            case.id, expected, rendered
        ))
    }
}

fn evaluate_po_plural_header(case: &ConformanceCase) -> Result<String, String> {
    let input = read_fixture_text(&case.input).map_err(|error| error.to_string())?;
    let expected = match case
        .expected_artifact()
        .map_err(|error| error.to_string())?
    {
        ExpectedArtifact::PoPluralHeader(expected) => expected,
        other => {
            return Err(format!(
                "case {} expected po_plural_header artifact, got {:?}",
                case.id, other
            ));
        }
    };

    let parsed = parse_po(&input).map_err(|error| format!("parse failed unexpectedly: {error}"))?;
    let raw_value = parsed
        .headers
        .iter()
        .find(|header| header.key == "Plural-Forms")
        .map(|header| header.value.clone());

    if raw_value != expected.raw_value {
        return Err(format!(
            "Plural-Forms header mismatch: expected {:?}, got {:?}",
            expected.raw_value, raw_value
        ));
    }

    let parsed_header = raw_value
        .as_deref()
        .map(parse_plural_forms_header)
        .unwrap_or_default();
    if parsed_header.nplurals != expected.nplurals {
        return Err(format!(
            "nplurals mismatch: expected {:?}, got {:?}",
            expected.nplurals, parsed_header.nplurals
        ));
    }
    if parsed_header.plural_expression != expected.plural_expression {
        return Err(format!(
            "plural expression mismatch: expected {:?}, got {:?}",
            expected.plural_expression, parsed_header.plural_expression
        ));
    }
    if let Some(expected_len) = expected.first_item_msgstr_len {
        let actual_len = parsed.items.first().map_or(0, |item| item.msgstr.len());
        if actual_len != expected_len {
            return Err(format!(
                "first item msgstr length mismatch: expected {expected_len}, got {actual_len}"
            ));
        }
    }

    if let Some(locale) = case.locale.as_deref() {
        let source_locale = case
            .source_locale
            .clone()
            .unwrap_or_else(|| "en".to_owned());
        parse_catalog(ParseCatalogOptions {
            content: input,
            locale: Some(locale.to_owned()),
            source_locale,
            plural_encoding: PluralEncoding::Gettext,
            strict: false,
        })
        .map_err(|error| format!("parse_catalog failed unexpectedly: {error:?}"))?;
    }

    Ok(String::new())
}

fn evaluate_icu_parse(case: &ConformanceCase) -> Result<String, String> {
    let input = read_fixture_text(&case.input).map_err(|error| error.to_string())?;
    let expected = match case
        .expected_artifact()
        .map_err(|error| error.to_string())?
    {
        ExpectedArtifact::IcuParse(expected) => expected,
        other => {
            return Err(format!(
                "case {} expected icu_parse artifact, got {:?}",
                case.id, other
            ));
        }
    };

    let parsed =
        parse_icu(&input).map_err(|error| format!("parse failed unexpectedly: {error}"))?;
    compare_icu_parse(&parsed.nodes, &expected)
}

fn evaluate_icu_reject(case: &ConformanceCase) -> Result<String, String> {
    let input = read_fixture_text(&case.input).map_err(|error| error.to_string())?;
    let expected = match case
        .expected_artifact()
        .map_err(|error| error.to_string())?
    {
        ExpectedArtifact::IcuReject(expected) => expected,
        other => {
            return Err(format!(
                "case {} expected icu_reject artifact, got {:?}",
                case.id, other
            ));
        }
    };

    match parse_icu(&input) {
        Ok(_) => Err("ICU parse unexpectedly succeeded".to_owned()),
        Err(error) => compare_icu_reject(
            &error.to_string(),
            error.position.line,
            error.position.column,
            &expected,
        ),
    }
}

fn load_po_parse_expected(case: &ConformanceCase) -> Result<PoParseExpected, String> {
    match case
        .expected_artifact()
        .map_err(|error| error.to_string())?
    {
        ExpectedArtifact::PoParse(expected) => Ok(expected),
        other => Err(format!(
            "case {} expected po_parse artifact, got {:?}",
            case.id, other
        )),
    }
}

fn compare_po_parse(
    parsed: &PoFile,
    expected: &PoParseExpected,
    item_start_index: usize,
) -> Result<String, String> {
    if let Some(item_count) = expected.item_count {
        if parsed.items.len() != item_count {
            return Err(format!(
                "item count mismatch: expected {item_count}, got {}",
                parsed.items.len()
            ));
        }
    }
    if let Some(header_count) = expected.header_count {
        if parsed.headers.len() != header_count {
            return Err(format!(
                "header count mismatch: expected {header_count}, got {}",
                parsed.headers.len()
            ));
        }
    }
    for (key, value) in &expected.headers {
        let actual = parsed
            .headers
            .iter()
            .find(|header| header.key == *key)
            .map(|header| header.value.as_str());
        if actual != Some(value.as_str()) {
            return Err(format!(
                "header {key:?} mismatch: expected {value:?}, got {actual:?}"
            ));
        }
    }
    for (index, expected_item) in expected.items.iter().enumerate() {
        let actual = parsed
            .items
            .get(item_start_index + index)
            .ok_or_else(|| format!("missing item at index {}", item_start_index + index))?;
        compare_po_item(actual, expected_item)?;
    }
    Ok(String::new())
}

fn compare_po_item(actual: &PoItem, expected: &PoItemExpected) -> Result<(), String> {
    if actual.msgid != expected.msgid {
        return Err(format!(
            "msgid mismatch: expected {:?}, got {:?}",
            expected.msgid, actual.msgid
        ));
    }
    if actual.msgctxt != expected.msgctxt {
        return Err(format!(
            "msgctxt mismatch: expected {:?}, got {:?}",
            expected.msgctxt, actual.msgctxt
        ));
    }
    if actual.msgid_plural != expected.msgid_plural {
        return Err(format!(
            "msgid_plural mismatch: expected {:?}, got {:?}",
            expected.msgid_plural, actual.msgid_plural
        ));
    }
    let actual_msgstr = actual.msgstr.iter().cloned().collect::<Vec<_>>();
    if actual_msgstr != expected.msgstr {
        return Err(format!(
            "msgstr mismatch: expected {:?}, got {:?}",
            expected.msgstr, actual_msgstr
        ));
    }
    if actual.comments != expected.comments {
        return Err(format!(
            "comments mismatch: expected {:?}, got {:?}",
            expected.comments, actual.comments
        ));
    }
    if actual.extracted_comments != expected.extracted_comments {
        return Err(format!(
            "extracted comments mismatch: expected {:?}, got {:?}",
            expected.extracted_comments, actual.extracted_comments
        ));
    }
    if actual.references != expected.references {
        return Err(format!(
            "references mismatch: expected {:?}, got {:?}",
            expected.references, actual.references
        ));
    }
    if actual.flags != expected.flags {
        return Err(format!(
            "flags mismatch: expected {:?}, got {:?}",
            expected.flags, actual.flags
        ));
    }
    if actual.obsolete != expected.obsolete {
        return Err(format!(
            "obsolete mismatch: expected {}, got {}",
            expected.obsolete, actual.obsolete
        ));
    }
    Ok(())
}

#[expect(
    clippy::too_many_lines,
    reason = "Conformance expectations are intentionally checked in one linear routine for auditability."
)]
fn compare_icu_parse(nodes: &[IcuNode], expected: &IcuParseExpected) -> Result<String, String> {
    if let Some(count) = expected.top_level_count {
        if nodes.len() != count {
            return Err(format!(
                "top-level node count mismatch: expected {count}, got {}",
                nodes.len()
            ));
        }
    }
    if !expected.node_kinds.is_empty() {
        let actual = nodes.iter().map(node_kind).collect::<Vec<_>>();
        if actual != expected.node_kinds {
            return Err(format!(
                "node kinds mismatch: expected {:?}, got {:?}",
                expected.node_kinds, actual
            ));
        }
    }
    if let Some(expected_literal) = &expected.first_literal {
        match nodes.first() {
            Some(IcuNode::Literal(actual)) if actual == expected_literal => {}
            other => {
                return Err(format!(
                    "first literal mismatch: expected {expected_literal:?}, got {other:?}"
                ));
            }
        }
    }
    if let Some(expected_name) = &expected.first_argument_name {
        match nodes.get(1).or_else(|| nodes.first()) {
            Some(IcuNode::Argument { name }) if name == expected_name => {}
            other => {
                return Err(format!(
                    "argument name mismatch: expected {expected_name:?}, got {other:?}"
                ));
            }
        }
    }
    if let Some(expected_kind) = &expected.first_plural_kind {
        match nodes.first() {
            Some(IcuNode::Plural {
                kind,
                offset,
                options,
                ..
            }) => {
                if plural_kind_name(kind) != expected_kind {
                    return Err(format!(
                        "first plural kind mismatch: expected {:?}, got {:?}",
                        expected_kind,
                        plural_kind_name(kind)
                    ));
                }
                if let Some(expected_offset) = expected.first_plural_offset {
                    let expected_offset = u32::try_from(expected_offset)
                        .map_err(|_| format!("invalid expected offset {expected_offset}"))?;
                    if *offset != expected_offset {
                        return Err(format!(
                            "first plural offset mismatch: expected {expected_offset}, got {offset}"
                        ));
                    }
                }
                if let Some(expected_count) = expected.first_plural_option_count {
                    if options.len() != expected_count {
                        return Err(format!(
                            "first plural option count mismatch: expected {expected_count}, got {}",
                            options.len()
                        ));
                    }
                }
            }
            other => return Err(format!("expected first node to be plural, got {other:?}")),
        }
    }
    if let Some(expected_kind) = &expected.second_plural_kind {
        let plurals = nodes
            .iter()
            .filter_map(|node| match node {
                IcuNode::Plural { kind, options, .. } => Some((kind, options.len())),
                _ => None,
            })
            .collect::<Vec<_>>();
        match plurals.get(1) {
            Some((kind, option_count)) => {
                if plural_kind_name(kind) != expected_kind {
                    return Err(format!(
                        "second plural kind mismatch: expected {:?}, got {:?}",
                        expected_kind,
                        plural_kind_name(kind)
                    ));
                }
                if let Some(expected_count) = expected.second_plural_option_count {
                    if *option_count != expected_count {
                        return Err(format!(
                            "second plural option count mismatch: expected {expected_count}, got {option_count}"
                        ));
                    }
                }
            }
            None => return Err("expected second plural node but none was found".to_owned()),
        }
    }
    Ok(String::new())
}

fn compare_icu_reject(
    message: &str,
    line: usize,
    column: usize,
    expected: &IcuRejectExpected,
) -> Result<String, String> {
    if !message.contains(&expected.message_contains) {
        return Err(format!(
            "error message mismatch: expected substring {:?}, got {:?}",
            expected.message_contains, message
        ));
    }
    if let Some(expected_line) = expected.line {
        if line != expected_line {
            return Err(format!(
                "error line mismatch: expected {expected_line}, got {line}"
            ));
        }
    }
    if let Some(min_column) = expected.min_column {
        if column < min_column {
            return Err(format!(
                "error column mismatch: expected at least {min_column}, got {column}"
            ));
        }
    }
    Ok(String::new())
}

fn node_kind(node: &IcuNode) -> String {
    match node {
        IcuNode::Literal(_) => "literal",
        IcuNode::Argument { .. } => "argument",
        IcuNode::Number { .. } => "number",
        IcuNode::Date { .. } => "date",
        IcuNode::Time { .. } => "time",
        IcuNode::Duration { .. } => "duration",
        IcuNode::Ago { .. } => "ago",
        IcuNode::Name { .. } => "name",
        IcuNode::Select { .. } => "select",
        IcuNode::Plural { .. } => "plural",
        IcuNode::Tag { .. } => "tag",
        IcuNode::List { .. } => "list",
        IcuNode::Pound => "pound",
    }
    .to_owned()
}

const fn plural_kind_name(kind: &IcuPluralKind) -> &'static str {
    match kind {
        IcuPluralKind::Cardinal => "cardinal",
        IcuPluralKind::Ordinal => "ordinal",
    }
}

#[derive(Default)]
struct ParsedPluralHeader {
    nplurals: Option<usize>,
    plural_expression: Option<String>,
}

fn parse_plural_forms_header(header: &str) -> ParsedPluralHeader {
    let mut parsed = ParsedPluralHeader::default();
    for part in header.split(';') {
        let trimmed = part.trim();
        if let Some(value) = trimmed.strip_prefix("nplurals=") {
            parsed.nplurals = value.trim().parse().ok();
        } else if let Some(value) = trimmed.strip_prefix("plural=") {
            let value = value.trim();
            if !value.is_empty() {
                parsed.plural_expression = Some(value.to_owned());
            }
        }
    }
    parsed
}

fn equivalent_text(left: &str, right: &str) -> bool {
    normalize_text(left) == normalize_text(right)
}

fn normalize_text(input: &str) -> String {
    input
        .replace("\r\n", "\n")
        .trim_end_matches('\n')
        .to_owned()
}
