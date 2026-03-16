#[path = "../../../conformance/icu_harness.rs"]
mod harness;

use harness::{EvaluationStatus, evaluate_suite, failure_messages};

#[test]
fn icu_conformance_cases_match_current_expectations() {
    let evaluations = evaluate_suite("icu-official").expect("evaluate icu suite");
    let failures = failure_messages(
        &evaluations
            .into_iter()
            .filter(|evaluation| evaluation.status == EvaluationStatus::Failed)
            .collect::<Vec<_>>(),
    );

    if !failures.is_empty() {
        panic!("ICU conformance failures:\n{}", failures.join("\n"));
    }
}
