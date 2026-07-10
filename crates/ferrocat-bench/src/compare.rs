mod adapters;
mod environment;
mod operations;
mod profile;
mod regression;
mod report;
mod semantic_digest;

use adapters::*;
use environment::*;
use operations::*;
use profile::*;
use regression::*;
use report::*;
use semantic_digest::*;

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
#[cfg(target_os = "macos")]
use std::ffi::CString;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use ferrocat_icu::{IcuMessage, IcuNode, IcuOption, IcuPluralKind, parse_icu};
use ferrocat_po::{
    BorrowedMsgStr, BorrowedPoFile, CatalogMessage, CatalogMode, CatalogOrigin, ExtractedMessage,
    ExtractedPluralMessage, ExtractedSingularMessage, MergeMessageInput, MsgStr,
    ParseCatalogOptions, ParsedCatalog, PluralSource, PoFile, SerializeOptions, TranslationShape,
    UpdateCatalogOptions, merge_catalog, parse_catalog, parse_po, parse_po_borrowed, stringify_po,
    update_catalog,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tempfile::TempDir;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::fixtures::{
    Fixture, IcuFixture, MergeFixture, fixture_by_name, icu_fixture_by_name, merge_fixture_by_name,
    parse_origin,
};

const INTERNAL_TOOL_VERSION: &str = concat!("ferrocat@", env!("CARGO_PKG_VERSION"));
const DEFAULT_MIN_SAMPLE_MILLIS: u64 = 250;
const DEFAULT_MAX_REGRESSION_PERCENT: f64 = 20.0;
const CALIBRATION_PROBE_RUNS: usize = 3;
const NOISE_CV_WARNING_THRESHOLD: f64 = 0.05;
const NOISE_RELATIVE_SPAN_WARNING_THRESHOLD: f64 = 10.0;

pub fn run_verify_benchmark_env() -> Result<(), String> {
    let workspace = workspace_root()?;
    let detected = BenchmarkEnvironment::detect(&workspace, None, ToolRequirement::External)?;

    println!("benchmark-root: {}", workspace.display());
    println!("system: {}", detected.system_label);
    println!("rustc: {}", detected.rustc_version);
    println!("node: {}", detected.node_version);
    println!(
        "python: {} [{}]",
        detected.python_version,
        detected.python_program.display()
    );
    println!("gettext-msgmerge: {}", detected.msgmerge_version);
    println!("gettext-msgcat: {}", detected.msgcat_version);
    println!("node-packages: {}", detected.node_adapter_version);
    println!("python-packages: {}", detected.python_adapter_version);
    println!("git-sha: {}", detected.git_sha);
    println!("os: {}", detected.os);
    println!("cpu: {}", detected.cpu_model);
    println!(
        "memory: {} ({} bytes)",
        format_memory_label(detected.memory_bytes),
        detected.memory_bytes
    );
    Ok(())
}

pub fn run_compare_command(
    profile_name: &str,
    args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let workspace = workspace_root()?;
    let options = CompareCliOptions::parse(args)?;
    let profile = BenchmarkProfile::load(&workspace, profile_name)?;
    let environment = BenchmarkEnvironment::detect(&workspace, None, profile.tool_requirement())?;
    let report = run_profile(&workspace, &environment, &profile)?;

    if let Some(parent) = options.out.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create report directory {}: {error}",
                parent.display()
            )
        })?;
    }

    let rendered = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("failed to serialize compare report: {error}"))?;
    fs::write(&options.out, rendered).map_err(|error| {
        format!(
            "failed to write compare report {}: {error}",
            options.out.display()
        )
    })?;

    println!("profile: {}", report.profile);
    println!("report: {}", options.out.display());
    println!("generated-at: {}", report.generated_at);
    println!("scenarios: {}", report.scenarios.len());
    for scenario in &report.scenarios {
        println!(
            "scenario: {} implementation={} fixture={} median-ms={:.3} cv={:.2}% span={:.2}% noisy={} samples={}",
            scenario.id,
            scenario.implementation,
            scenario.fixture,
            nanos_to_millis(scenario.statistics.median_elapsed_ns),
            scenario.statistics.coefficient_of_variation * 100.0,
            scenario.statistics.relative_span_percent,
            scenario.statistics.noisy,
            scenario.samples.len()
        );
    }

    Ok(())
}

pub fn run_regression_check_command(args: impl Iterator<Item = String>) -> Result<(), String> {
    let options = RegressionCheckCliOptions::parse(args)?;
    let baseline = load_compare_report(&options.baseline)?;
    let current = load_compare_report(&options.current)?;
    let report = compare_regression_reports(&baseline, &current, options.max_regression_percent)?;

    print!("{}", report.render());
    if report.has_failures() {
        return Err(format!(
            "benchmark regression check failed: {} scenario(s) exceeded {:.2}%",
            report.failures.len(),
            options.max_regression_percent
        ));
    }

    Ok(())
}

fn load_compare_report(path: &Path) -> Result<CompareReport, String> {
    let content = fs::read_to_string(path).map_err(|error| {
        format!(
            "failed to read benchmark report {}: {error}",
            path.display()
        )
    })?;
    serde_json::from_str(&content).map_err(|error| {
        format!(
            "failed to parse benchmark report {}: {error}",
            path.display()
        )
    })
}

#[expect(
    clippy::too_many_lines,
    reason = "Benchmark scheduling is kept in one routine so execution order and validation stay easy to audit."
)]
fn run_profile(
    workspace: &Path,
    environment: &BenchmarkEnvironment,
    profile: &BenchmarkProfile,
) -> Result<CompareReport, String> {
    // Keep runs interleaved within each comparison group so scheduler drift,
    // caches, and thermal behavior are less likely to bias one implementation.
    let mut grouped = BTreeMap::<String, Vec<BenchmarkScenario>>::new();
    for scenario in &profile.scenarios {
        grouped
            .entry(scenario.comparison_group.clone())
            .or_default()
            .push(scenario.clone());
    }

    let mut reports = Vec::with_capacity(profile.scenarios.len());
    for scenarios in grouped.into_values() {
        let prepared = PreparedScenario::prepare(workspace, &scenarios)?;
        let mut expected_digest = None::<String>;
        let mut plans = Vec::with_capacity(scenarios.len());

        for scenario in scenarios {
            let validation = execute_scenario(workspace, &prepared, &scenario, 1, true)?;
            let validated_digest = prepared.validate(&validation)?;
            if validation.reported_digest != validated_digest {
                return Err(format!(
                    "scenario {} reported digest {} but validated as {}",
                    scenario.id, validation.reported_digest, validated_digest
                ));
            }

            match &expected_digest {
                Some(reference) if reference != &validated_digest => {
                    return Err(format!(
                        "scenario {} produced digest {} but comparison group {} expects {}",
                        scenario.id, validated_digest, scenario.comparison_group, reference
                    ));
                }
                None => expected_digest = Some(validated_digest.clone()),
                _ => {}
            }

            let mut calibration_elapsed_ns = Vec::with_capacity(CALIBRATION_PROBE_RUNS);
            calibration_elapsed_ns.push(validation.elapsed_ns);
            for _ in 1..CALIBRATION_PROBE_RUNS {
                let probe = execute_scenario(workspace, &prepared, &scenario, 1, false)?;
                if probe.reported_digest != validated_digest {
                    return Err(format!(
                        "calibration digest mismatch for scenario {}: expected {}, got {}",
                        scenario.id, validated_digest, probe.reported_digest
                    ));
                }
                calibration_elapsed_ns.push(probe.elapsed_ns);
            }

            let iterations = calibrate_iterations(
                scenario
                    .minimum_sample_millis
                    .unwrap_or(profile.minimum_sample_millis),
                &calibration_elapsed_ns,
            );
            let cli_baseline = PreparedScenario::prepare_cli_baseline(workspace, &scenario)?;
            plans.push(ScenarioExecutionPlan {
                scenario,
                tool_version: validation.tool_version,
                validated_digest,
                iterations,
                cli_baseline,
            });
        }

        for index in round_robin_schedule(
            &plans
                .iter()
                .map(|plan| plan.scenario.warmup_runs)
                .collect::<Vec<_>>(),
        ) {
            let plan = &plans[index];
            let warmup =
                execute_scenario(workspace, &prepared, &plan.scenario, plan.iterations, false)?;
            if warmup.reported_digest != plan.validated_digest {
                return Err(format!(
                    "warmup digest mismatch for scenario {}: expected {}, got {}",
                    plan.scenario.id, plan.validated_digest, warmup.reported_digest
                ));
            }
            if let Some(baseline) = &plan.cli_baseline {
                execute_scenario(
                    workspace,
                    &baseline.prepared,
                    &plan.scenario,
                    plan.iterations,
                    false,
                )?;
            }
        }

        let mut samples_by_plan = plans
            .iter()
            .map(|plan| Vec::with_capacity(plan.scenario.measured_runs))
            .collect::<Vec<_>>();
        for index in round_robin_schedule(
            &plans
                .iter()
                .map(|plan| plan.scenario.measured_runs)
                .collect::<Vec<_>>(),
        ) {
            let plan = &plans[index];
            let mut sample =
                execute_scenario(workspace, &prepared, &plan.scenario, plan.iterations, false)?;
            if sample.reported_digest != plan.validated_digest {
                return Err(format!(
                    "measured digest mismatch for scenario {}: expected {}, got {}",
                    plan.scenario.id, plan.validated_digest, sample.reported_digest
                ));
            }
            if let Some(baseline) = &plan.cli_baseline {
                let baseline_sample = execute_scenario(
                    workspace,
                    &baseline.prepared,
                    &plan.scenario,
                    plan.iterations,
                    false,
                )?;
                sample.baseline_elapsed_ns = Some(baseline_sample.elapsed_ns);
            }
            samples_by_plan[index].push(sample);
        }

        for (plan, samples) in plans.into_iter().zip(samples_by_plan) {
            let statistics = ScenarioStatistics::from_samples(&samples);
            reports.push(ScenarioReport {
                id: plan.scenario.id.clone(),
                comparison_group: plan.scenario.comparison_group.clone(),
                workload: plan.scenario.workload.clone(),
                operation: plan.scenario.operation.clone(),
                fixture: plan.scenario.fixture.clone(),
                implementation: plan.scenario.implementation.clone(),
                tool_version: plan.tool_version,
                iterations_per_sample: plan.iterations,
                warmup_runs: plan.scenario.warmup_runs,
                measured_runs: plan.scenario.measured_runs,
                semantic_digest: plan.validated_digest,
                baseline_strategy: plan
                    .cli_baseline
                    .as_ref()
                    .map(|_| "empty-cli-run".to_owned()),
                baseline_fixture: plan
                    .cli_baseline
                    .as_ref()
                    .map(|baseline| baseline.label.clone()),
                statistics,
                samples: samples
                    .iter()
                    .map(ScenarioSampleReport::from_execution)
                    .collect(),
            });
        }
    }

    Ok(CompareReport {
        profile: profile.name.clone(),
        generated_at: OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .map_err(|error| format!("failed to format current time: {error}"))?,
        reference_host_policy: "single-reference-host".to_owned(),
        environment: environment.metadata(),
        scenarios: reports,
    })
}

fn execute_scenario(
    workspace: &Path,
    prepared: &PreparedScenario,
    scenario: &BenchmarkScenario,
    iterations: usize,
    capture_artifacts: bool,
) -> Result<ExecutionResult, String> {
    match scenario.implementation.as_str() {
        "ferrocat-parse" => prepared.run_internal_parse(iterations, false),
        "ferrocat-parse-borrowed" => prepared.run_internal_parse(iterations, true),
        "ferrocat-parse-catalog-po" => prepared.run_internal_parse_catalog(iterations),
        "ferrocat-parse-catalog-fcl" => prepared.run_internal_parse_catalog_fcl(iterations),
        "ferrocat-stringify" => prepared.run_internal_stringify(iterations, capture_artifacts),
        "ferrocat-merge" => prepared.run_internal_merge(iterations, capture_artifacts),
        "ferrocat-update-catalog" => {
            prepared.run_internal_update_catalog(iterations, capture_artifacts)
        }
        "ferrocat-update-catalog-file" => {
            prepared.run_internal_update_catalog_file(iterations, capture_artifacts)
        }
        "ferrocat-parse-icu" => prepared.run_internal_parse_icu(iterations, capture_artifacts),
        "pofile"
        | "pofile-ts"
        | "gettext-parser"
        | "formatjs-icu-parser"
        | "messageformat-parser" => {
            prepared.run_node_adapter(workspace, scenario, iterations, capture_artifacts)
        }
        "polib" | "babel" => {
            prepared.run_python_adapter(workspace, scenario, iterations, capture_artifacts)
        }
        "php-gettext" => {
            prepared.run_php_adapter(workspace, scenario, iterations, capture_artifacts)
        }
        "msgcat" => prepared.run_msgcat(iterations, capture_artifacts),
        "msgmerge" => prepared.run_msgmerge(iterations, capture_artifacts),
        other => Err(format!("unsupported benchmark implementation: {other}")),
    }
}

fn calibrate_iterations(minimum_sample_millis: u64, probe_elapsed_ns: &[u128]) -> usize {
    let Some(single_elapsed_ns) = median_u128(probe_elapsed_ns.to_vec()) else {
        return 1;
    };
    if single_elapsed_ns == 0 {
        return 1;
    }
    let target_ns = u128::from(minimum_sample_millis.max(1)) * 1_000_000;
    let iterations = target_ns.div_ceil(single_elapsed_ns);
    iterations.clamp(1, 1_000_000) as usize
}

fn workspace_root() -> Result<PathBuf, String> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| "failed to resolve workspace root from manifest directory".to_owned())
}

#[derive(Debug)]
struct CompareCliOptions {
    out: PathBuf,
}

impl CompareCliOptions {
    fn parse(mut args: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut out = None;
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--out" => {
                    let value = args
                        .next()
                        .ok_or_else(|| "--out requires a path value".to_owned())?;
                    out = Some(PathBuf::from(value));
                }
                value => return Err(format!("unknown compare flag: {value}")),
            }
        }

        Ok(Self {
            out: out.ok_or_else(|| "compare requires --out <json-path>".to_owned())?,
        })
    }
}

#[derive(Debug)]
struct RegressionCheckCliOptions {
    baseline: PathBuf,
    current: PathBuf,
    max_regression_percent: f64,
}

impl RegressionCheckCliOptions {
    fn parse(mut args: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut baseline = None;
        let mut current = None;
        let mut max_regression_percent = DEFAULT_MAX_REGRESSION_PERCENT;

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--baseline" => {
                    let value = args
                        .next()
                        .ok_or_else(|| "--baseline requires a path value".to_owned())?;
                    baseline = Some(PathBuf::from(value));
                }
                "--current" => {
                    let value = args
                        .next()
                        .ok_or_else(|| "--current requires a path value".to_owned())?;
                    current = Some(PathBuf::from(value));
                }
                "--max-regression-percent" => {
                    let value = args.next().ok_or_else(|| {
                        "--max-regression-percent requires a numeric value".to_owned()
                    })?;
                    max_regression_percent =
                        parse_positive_f64("--max-regression-percent", &value)?;
                }
                value => return Err(format!("unknown regression-check flag: {value}")),
            }
        }

        Ok(Self {
            baseline: baseline
                .ok_or_else(|| "regression-check requires --baseline <json-path>".to_owned())?,
            current: current
                .ok_or_else(|| "regression-check requires --current <json-path>".to_owned())?,
            max_regression_percent,
        })
    }
}

fn parse_positive_f64(label: &str, value: &str) -> Result<f64, String> {
    let parsed = value
        .parse::<f64>()
        .map_err(|_| format!("invalid {label} value: {value}"))?;
    if !parsed.is_finite() || parsed <= 0.0 {
        return Err(format!("{label} must be a positive finite number"));
    }
    Ok(parsed)
}

include!("compare/tests.rs");
