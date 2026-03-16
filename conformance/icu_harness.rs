#![allow(dead_code)]

use ferrox_conformance::{
    ConformanceCase, ConformanceManifest, Expectation, ExpectedArtifact, IcuParseExpected,
    IcuRejectExpected, load_all_manifests, read_expected_artifact, read_fixture_text,
};
use ferrox_icu::{IcuNode, IcuPluralKind, parse_icu};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvaluationStatus {
    Matched,
    Failed,
    KnownGap,
}

#[derive(Debug, Clone)]
pub struct CaseEvaluation {
    pub case_id: String,
    pub status: EvaluationStatus,
    pub detail: String,
}

pub fn evaluate_suite(suite: &str) -> Result<Vec<CaseEvaluation>, String> {
    let manifests = load_all_manifests().map_err(|error| error.to_string())?;
    let filtered = manifests
        .into_iter()
        .filter(|manifest| manifest.suite == suite)
        .collect::<Vec<_>>();

    let mut evaluations = Vec::new();
    for manifest in &filtered {
        for case in &manifest.cases {
            evaluations.push(evaluate_case(manifest, case)?);
        }
    }
    Ok(evaluations)
}

pub fn failure_messages(evaluations: &[CaseEvaluation]) -> Vec<String> {
    evaluations
        .iter()
        .filter(|evaluation| evaluation.status == EvaluationStatus::Failed)
        .map(|evaluation| format!("[{}] {}", evaluation.case_id, evaluation.detail))
        .collect()
}

fn evaluate_case(
    _manifest: &ConformanceManifest,
    case: &ConformanceCase,
) -> Result<CaseEvaluation, String> {
    if case.expectation == Expectation::KnownGap {
        return Ok(CaseEvaluation {
            case_id: case.id.clone(),
            status: EvaluationStatus::KnownGap,
            detail: case
                .notes
                .clone()
                .unwrap_or_else(|| "classified as known gap".to_owned()),
        });
    }

    let detail_result = match case.runner.as_str() {
        "icu_parse" => evaluate_icu_parse(case),
        "icu_reject" => evaluate_icu_reject(case),
        other => Err(format!("unknown ICU runner `{other}` for case {}", case.id)),
    };

    let (status, detail) = match detail_result {
        Ok(detail) if detail.is_empty() => {
            (EvaluationStatus::Matched, "matched expectation".to_owned())
        }
        Ok(detail) => (EvaluationStatus::Failed, detail),
        Err(detail) => (EvaluationStatus::Failed, detail),
    };

    Ok(CaseEvaluation {
        case_id: case.id.clone(),
        status,
        detail,
    })
}

fn evaluate_icu_parse(case: &ConformanceCase) -> Result<String, String> {
    let input = read_fixture_text(&case.input).map_err(|error| error.to_string())?;
    let expected = match read_expected_artifact(
        case.expected
            .as_deref()
            .ok_or_else(|| format!("icu parse case {} is missing expected artifact", case.id))?,
    )
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
    let expected = match read_expected_artifact(
        case.expected
            .as_deref()
            .ok_or_else(|| format!("icu reject case {} is missing expected artifact", case.id))?,
    )
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
                    "first literal mismatch: expected {:?}, got {:?}",
                    expected_literal, other
                ));
            }
        }
    }
    if let Some(expected_name) = &expected.first_argument_name {
        match nodes.get(1).or_else(|| nodes.first()) {
            Some(IcuNode::Argument { name }) if name == expected_name => {}
            other => {
                return Err(format!(
                    "argument name mismatch: expected {:?}, got {:?}",
                    expected_name, other
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
                if plural_kind_name(kind.clone()) != expected_kind {
                    return Err(format!(
                        "first plural kind mismatch: expected {:?}, got {:?}",
                        expected_kind,
                        plural_kind_name(kind.clone())
                    ));
                }
                if let Some(expected_offset) = expected.first_plural_offset {
                    let expected_offset = u32::try_from(expected_offset)
                        .map_err(|_| format!("invalid expected offset {}", expected_offset))?;
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
            other => return Err(format!("expected first node to be plural, got {:?}", other)),
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
                if plural_kind_name((*kind).clone()) != expected_kind {
                    return Err(format!(
                        "second plural kind mismatch: expected {:?}, got {:?}",
                        expected_kind,
                        plural_kind_name((*kind).clone())
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

fn plural_kind_name(kind: IcuPluralKind) -> &'static str {
    match kind {
        IcuPluralKind::Cardinal => "cardinal",
        IcuPluralKind::Ordinal => "ordinal",
    }
}
