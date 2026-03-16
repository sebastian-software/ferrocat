#[path = "../../../conformance/harness.rs"]
mod harness;

use harness::{EvaluationStatus, evaluate_suite, failure_messages};

#[test]
fn merge_conformance_cases_match_current_expectations() {
    let evaluations = evaluate_suite("po-polib").expect("evaluate po-polib");
    let failures = failure_messages(
        &evaluations
            .into_iter()
            .filter(|evaluation| {
                evaluation.capability == "merge" && evaluation.status == EvaluationStatus::Failed
            })
            .collect::<Vec<_>>(),
    );

    if !failures.is_empty() {
        panic!("Merge conformance failures:\n{}", failures.join("\n"));
    }
}
