//! Regression comparison, classification, and human-readable reporting.

use super::*;

#[derive(Debug)]
pub(super) struct BenchmarkRegressionReport {
    pub(super) profile: String,
    pub(super) max_regression_percent: f64,
    pub(super) passed: Vec<BenchmarkRegressionScenario>,
    pub(super) failures: Vec<BenchmarkRegressionScenario>,
    pub(super) skipped_noisy: Vec<BenchmarkRegressionScenario>,
    pub(super) skipped_semantics_changed: Vec<BenchmarkSemanticsChangedScenario>,
    pub(super) missing_baseline: Vec<String>,
    pub(super) missing_current: Vec<String>,
}

impl BenchmarkRegressionReport {
    pub(super) fn has_failures(&self) -> bool {
        !self.failures.is_empty() || !self.missing_current.is_empty()
    }

    pub(super) fn render(&self) -> String {
        let mut out = String::new();
        out.push_str("Benchmark regression check\n");
        out.push_str(&format!("profile: {}\n", self.profile));
        out.push_str(&format!(
            "max-regression-percent: {:.2}\n",
            self.max_regression_percent
        ));
        out.push_str(&format!(
            "passed: {} failed: {} skipped-noisy: {} skipped-semantics-changed: {} missing-baseline: {} missing-current: {}\n",
            self.passed.len(),
            self.failures.len(),
            self.skipped_noisy.len(),
            self.skipped_semantics_changed.len(),
            self.missing_baseline.len(),
            self.missing_current.len()
        ));

        for scenario in &self.failures {
            out.push_str(&format!(
                "FAIL {}: +{:.2}% per-iteration time ({} -> {})\n",
                scenario.id,
                scenario.regression_percent,
                format_per_iteration_ns(scenario.baseline_per_iteration_ns),
                format_per_iteration_ns(scenario.current_per_iteration_ns)
            ));
        }
        for scenario in &self.skipped_noisy {
            out.push_str(&format!(
                "SKIP noisy {}: {:+.2}% per-iteration time (baseline noisy={} current noisy={})\n",
                scenario.id,
                scenario.regression_percent,
                scenario.baseline_noisy,
                scenario.current_noisy
            ));
        }
        for scenario in &self.skipped_semantics_changed {
            out.push_str(&format!(
                "SKIP semantics-changed {}: baseline digest {} -> current digest {}\n",
                scenario.id, scenario.baseline_semantic_digest, scenario.current_semantic_digest
            ));
        }
        for id in &self.missing_baseline {
            out.push_str(&format!("SKIP missing-baseline {id}\n"));
        }
        for id in &self.missing_current {
            out.push_str(&format!("FAIL missing-current {id}\n"));
        }
        if self.has_failures() {
            out.push_str("result: FAIL\n");
        } else {
            out.push_str("result: PASS\n");
        }
        out
    }
}

#[derive(Debug)]
pub(super) struct BenchmarkRegressionScenario {
    pub(super) id: String,
    pub(super) baseline_per_iteration_ns: f64,
    pub(super) current_per_iteration_ns: f64,
    pub(super) regression_percent: f64,
    pub(super) baseline_noisy: bool,
    pub(super) current_noisy: bool,
}

#[derive(Debug)]
pub(super) struct BenchmarkSemanticsChangedScenario {
    pub(super) id: String,
    pub(super) baseline_semantic_digest: String,
    pub(super) current_semantic_digest: String,
}

pub(super) fn compare_regression_reports(
    baseline: &CompareReport,
    current: &CompareReport,
    max_regression_percent: f64,
) -> Result<BenchmarkRegressionReport, String> {
    if baseline.profile != current.profile {
        return Err(format!(
            "cannot compare benchmark profiles {} and {}",
            baseline.profile, current.profile
        ));
    }

    let baseline_by_id = baseline
        .scenarios
        .iter()
        .map(|scenario| (scenario.id.as_str(), scenario))
        .collect::<BTreeMap<_, _>>();
    let current_by_id = current
        .scenarios
        .iter()
        .map(|scenario| (scenario.id.as_str(), scenario))
        .collect::<BTreeMap<_, _>>();

    let mut passed = Vec::new();
    let mut failures = Vec::new();
    let mut skipped_noisy = Vec::new();
    let mut skipped_semantics_changed = Vec::new();
    let mut missing_baseline = Vec::new();
    let mut missing_current = Vec::new();

    for scenario in &current.scenarios {
        let Some(baseline_scenario) = baseline_by_id.get(scenario.id.as_str()) else {
            missing_baseline.push(scenario.id.clone());
            continue;
        };
        if baseline_scenario.semantic_digest != scenario.semantic_digest {
            skipped_semantics_changed.push(BenchmarkSemanticsChangedScenario {
                id: scenario.id.clone(),
                baseline_semantic_digest: baseline_scenario.semantic_digest.clone(),
                current_semantic_digest: scenario.semantic_digest.clone(),
            });
            continue;
        }
        let regression = regression_scenario(baseline_scenario, scenario)?;
        if regression.baseline_noisy || regression.current_noisy {
            skipped_noisy.push(regression);
        } else if regression.regression_percent > max_regression_percent {
            failures.push(regression);
        } else {
            passed.push(regression);
        }
    }

    for scenario in &baseline.scenarios {
        if !current_by_id.contains_key(scenario.id.as_str()) {
            missing_current.push(scenario.id.clone());
        }
    }

    if passed.is_empty()
        && failures.is_empty()
        && skipped_noisy.is_empty()
        && skipped_semantics_changed.is_empty()
        && missing_baseline.is_empty()
        && missing_current.is_empty()
    {
        return Err("benchmark reports have no comparable scenarios".to_owned());
    }

    Ok(BenchmarkRegressionReport {
        profile: current.profile.clone(),
        max_regression_percent,
        passed,
        failures,
        skipped_noisy,
        skipped_semantics_changed,
        missing_baseline,
        missing_current,
    })
}

pub(super) fn regression_scenario(
    baseline: &ScenarioReport,
    current: &ScenarioReport,
) -> Result<BenchmarkRegressionScenario, String> {
    // Compare per-iteration time, not raw sample time. The harness calibrates
    // iterations_per_sample independently per run, so a faster build can pick
    // more iterations per sample and end up with a *larger* median_elapsed_ns
    // even though each operation got quicker. Normalizing by iterations keeps
    // the check measuring the work, not the calibration.
    let baseline_per_iteration = per_iteration_nanos(baseline)?;
    let current_per_iteration = per_iteration_nanos(current)?;

    let regression_percent = ((current_per_iteration / baseline_per_iteration) - 1.0) * 100.0;

    Ok(BenchmarkRegressionScenario {
        id: current.id.clone(),
        baseline_per_iteration_ns: baseline_per_iteration,
        current_per_iteration_ns: current_per_iteration,
        regression_percent,
        baseline_noisy: baseline.statistics.noisy,
        current_noisy: current.statistics.noisy,
    })
}

pub(super) fn per_iteration_nanos(report: &ScenarioReport) -> Result<f64, String> {
    if report.iterations_per_sample == 0 {
        return Err(format!(
            "scenario {} has zero iterations_per_sample",
            report.id
        ));
    }
    if report.statistics.median_elapsed_ns == 0 {
        return Err(format!(
            "scenario {} has zero median elapsed time",
            report.id
        ));
    }
    Ok(f64_from_u128(report.statistics.median_elapsed_ns) / report.iterations_per_sample as f64)
}

pub(super) fn format_per_iteration_ns(value: f64) -> String {
    format!("{:.4} ms", value / 1_000_000.0)
}
