#[path = "../../../conformance/harness.rs"]
mod harness;

use harness::{EvaluationStatus, evaluate_suite, failure_messages};

#[test]
fn po_conformance_cases_match_current_expectations() {
    let mut failures = Vec::new();
    for suite in ["po-polib", "po-pofile", "po-babel"] {
        let evaluations = evaluate_suite(suite).expect("evaluate suite");
        failures.extend(failure_messages(
            &evaluations
                .into_iter()
                .filter(|evaluation| {
                    evaluation.capability != "merge"
                        && evaluation.status == EvaluationStatus::Failed
                })
                .collect::<Vec<_>>(),
        ));
    }

    assert!(
        failures.is_empty(),
        "PO conformance failures:\n{}",
        failures.join("\n")
    );
}
