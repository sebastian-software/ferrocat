use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use ferrocat_icu::{IcuMessage, IcuNode, IcuOption, IcuPluralKind, parse_icu};
use ferrocat_po::{
    ExtractedMessage, MergeExtractedMessage, MsgStr, PluralEncoding, PoFile, SerializeOptions,
    UpdateCatalogOptions, merge_catalog, parse_po, parse_po_borrowed, stringify_po, update_catalog,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tempfile::TempDir;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::fixtures::{
    Fixture, IcuFixture, MergeFixture, fixture_by_name, icu_fixture_by_name, merge_fixture_by_name,
};

const INTERNAL_TOOL_VERSION: &str = concat!("ferrocat@", env!("CARGO_PKG_VERSION"));
const DEFAULT_MIN_SAMPLE_MILLIS: u64 = 250;

pub fn run_verify_benchmark_env() -> Result<(), String> {
    let workspace = workspace_root()?;
    let detected = BenchmarkEnvironment::detect(&workspace, None)?;

    println!("benchmark-root: {}", workspace.display());
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
    println!("host: {}", detected.host_identifier);
    println!("os: {}", detected.os);
    println!("cpu: {}", detected.cpu_model);
    Ok(())
}

pub fn run_compare_command(
    profile_name: &str,
    args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let workspace = workspace_root()?;
    let options = CompareCliOptions::parse(args)?;
    let environment = BenchmarkEnvironment::detect(&workspace, None)?;
    let profile = BenchmarkProfile::load(&workspace, profile_name)?;
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
            "scenario: {} implementation={} fixture={} median-ms={:.3} samples={}",
            scenario.id,
            scenario.implementation,
            scenario.fixture,
            scenario.statistics.median_elapsed_ns as f64 / 1_000_000.0,
            scenario.samples.len()
        );
    }

    Ok(())
}

fn run_profile(
    workspace: &Path,
    environment: &BenchmarkEnvironment,
    profile: &BenchmarkProfile,
) -> Result<CompareReport, String> {
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

            let iterations = calibrate_iterations(
                scenario
                    .minimum_sample_millis
                    .unwrap_or(profile.minimum_sample_millis),
                validation.elapsed_ns,
            );

            for _ in 0..scenario.warmup_runs {
                let warmup = execute_scenario(workspace, &prepared, &scenario, iterations, false)?;
                if warmup.reported_digest != validated_digest {
                    return Err(format!(
                        "warmup digest mismatch for scenario {}: expected {}, got {}",
                        scenario.id, validated_digest, warmup.reported_digest
                    ));
                }
            }

            let mut samples = Vec::with_capacity(scenario.measured_runs);
            for _ in 0..scenario.measured_runs {
                let sample = execute_scenario(workspace, &prepared, &scenario, iterations, false)?;
                if sample.reported_digest != validated_digest {
                    return Err(format!(
                        "measured digest mismatch for scenario {}: expected {}, got {}",
                        scenario.id, validated_digest, sample.reported_digest
                    ));
                }
                samples.push(sample);
            }

            let statistics = ScenarioStatistics::from_samples(&samples);
            reports.push(ScenarioReport {
                id: scenario.id.clone(),
                comparison_group: scenario.comparison_group.clone(),
                workload: scenario.workload.clone(),
                operation: scenario.operation.clone(),
                fixture: scenario.fixture.clone(),
                implementation: scenario.implementation.clone(),
                tool_version: validation.tool_version,
                iterations_per_sample: iterations,
                warmup_runs: scenario.warmup_runs,
                measured_runs: scenario.measured_runs,
                semantic_digest: validated_digest,
                statistics,
                samples: samples
                    .into_iter()
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
        "ferrocat-stringify" => prepared.run_internal_stringify(iterations, capture_artifacts),
        "ferrocat-merge" => prepared.run_internal_merge(iterations, capture_artifacts),
        "ferrocat-update-catalog" => {
            prepared.run_internal_update_catalog(iterations, capture_artifacts)
        }
        "ferrocat-parse-icu" => prepared.run_internal_parse_icu(iterations, capture_artifacts),
        "pofile" | "formatjs-icu-parser" | "messageformat-parser" => {
            prepared.run_node_adapter(workspace, scenario, iterations, capture_artifacts)
        }
        "polib" => prepared.run_python_adapter(workspace, scenario, iterations, capture_artifacts),
        "msgcat" => prepared.run_msgcat(iterations, capture_artifacts),
        "msgmerge" => prepared.run_msgmerge(iterations, capture_artifacts),
        other => Err(format!("unsupported benchmark implementation: {other}")),
    }
}

fn calibrate_iterations(minimum_sample_millis: u64, single_elapsed_ns: u128) -> usize {
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

#[derive(Debug, Clone, Deserialize)]
struct BenchmarkProfile {
    name: String,
    #[serde(default = "default_minimum_sample_millis")]
    minimum_sample_millis: u64,
    scenarios: Vec<BenchmarkScenario>,
}

impl BenchmarkProfile {
    fn load(workspace: &Path, profile_name: &str) -> Result<Self, String> {
        let path = workspace
            .join("benchmark")
            .join("profiles")
            .join(format!("{profile_name}.json"));
        let content = fs::read_to_string(&path).map_err(|error| {
            format!(
                "failed to read benchmark profile {}: {error}",
                path.display()
            )
        })?;
        let profile: BenchmarkProfile = serde_json::from_str(&content).map_err(|error| {
            format!(
                "failed to parse benchmark profile {}: {error}",
                path.display()
            )
        })?;
        if profile.name != profile_name {
            return Err(format!(
                "benchmark profile {} declares mismatched name {}",
                path.display(),
                profile.name
            ));
        }
        if profile.scenarios.is_empty() {
            return Err(format!(
                "benchmark profile {} defines no scenarios",
                path.display()
            ));
        }
        Ok(profile)
    }
}

fn default_minimum_sample_millis() -> u64 {
    DEFAULT_MIN_SAMPLE_MILLIS
}

#[derive(Debug, Clone, Deserialize)]
struct BenchmarkScenario {
    id: String,
    comparison_group: String,
    workload: String,
    operation: String,
    fixture: String,
    implementation: String,
    warmup_runs: usize,
    measured_runs: usize,
    minimum_sample_millis: Option<u64>,
}

#[derive(Debug)]
struct PreparedScenario {
    operation: String,
    fixture: String,
    _tempdir: TempDir,
    po_input_path: Option<PathBuf>,
    icu_messages_path: Option<PathBuf>,
    existing_po_path: Option<PathBuf>,
    pot_path: Option<PathBuf>,
    po_content: Option<String>,
    po_file: Option<PoFile>,
    merge_fixture: Option<OwnedMergeFixture>,
    icu_messages: Option<Vec<String>>,
}

impl PreparedScenario {
    fn prepare(workspace: &Path, scenarios: &[BenchmarkScenario]) -> Result<Self, String> {
        let Some(first) = scenarios.first() else {
            return Err("cannot prepare empty benchmark scenario group".to_owned());
        };
        for scenario in scenarios {
            if scenario.operation != first.operation || scenario.fixture != first.fixture {
                return Err(format!(
                    "comparison group {} mixes incompatible operations or fixtures",
                    first.comparison_group
                ));
            }
        }

        let tempdir = tempfile::Builder::new()
            .prefix("ferrocat-compare-")
            .tempdir_in(workspace.join("target"))
            .map_err(|error| format!("failed to create compare tempdir: {error}"))?;

        match first.operation.as_str() {
            "parse" | "stringify" => {
                let fixture = load_fixture(&first.fixture)?;
                let input_path = tempdir.path().join("input.po");
                fs::write(&input_path, fixture.content()).map_err(|error| {
                    format!(
                        "failed to write fixture input {}: {error}",
                        input_path.display()
                    )
                })?;
                let po_file = parse_po(fixture.content())
                    .map_err(|error| format!("failed to parse fixture: {error}"))?;
                Ok(Self {
                    operation: first.operation.clone(),
                    fixture: first.fixture.clone(),
                    _tempdir: tempdir,
                    po_input_path: Some(input_path),
                    icu_messages_path: None,
                    existing_po_path: None,
                    pot_path: None,
                    po_content: Some(fixture.content().to_owned()),
                    po_file: Some(po_file),
                    merge_fixture: None,
                    icu_messages: None,
                })
            }
            "merge" | "update-catalog" => {
                let fixture = load_merge_fixture(&first.fixture)?;
                let existing_po_path = tempdir.path().join("existing.po");
                fs::write(&existing_po_path, fixture.existing_po()).map_err(|error| {
                    format!(
                        "failed to write merge fixture input {}: {error}",
                        existing_po_path.display()
                    )
                })?;
                let pot_path = if first.operation == "merge"
                    || scenarios
                        .iter()
                        .any(|scenario| scenario.implementation == "msgmerge")
                {
                    let pot_path = tempdir.path().join("template.pot");
                    let pot = build_merge_pot(&fixture);
                    fs::write(&pot_path, pot).map_err(|error| {
                        format!(
                            "failed to write merge template {}: {error}",
                            pot_path.display()
                        )
                    })?;
                    Some(pot_path)
                } else {
                    None
                };
                Ok(Self {
                    operation: first.operation.clone(),
                    fixture: first.fixture.clone(),
                    _tempdir: tempdir,
                    po_input_path: None,
                    icu_messages_path: None,
                    existing_po_path: Some(existing_po_path),
                    pot_path,
                    po_content: None,
                    po_file: None,
                    merge_fixture: Some(OwnedMergeFixture::from_fixture(&fixture)),
                    icu_messages: None,
                })
            }
            "parse-icu" => {
                let fixture = load_icu_fixture(&first.fixture)?;
                let messages_path = tempdir.path().join("messages.json");
                let messages = fixture.messages().to_vec();
                let serialized = serde_json::to_string_pretty(&messages)
                    .map_err(|error| format!("failed to serialize ICU messages: {error}"))?;
                fs::write(&messages_path, serialized).map_err(|error| {
                    format!(
                        "failed to write ICU fixture input {}: {error}",
                        messages_path.display()
                    )
                })?;
                Ok(Self {
                    operation: first.operation.clone(),
                    fixture: first.fixture.clone(),
                    _tempdir: tempdir,
                    po_input_path: None,
                    icu_messages_path: Some(messages_path),
                    existing_po_path: None,
                    pot_path: None,
                    po_content: None,
                    po_file: None,
                    merge_fixture: None,
                    icu_messages: Some(messages),
                })
            }
            other => Err(format!("unsupported benchmark operation: {other}")),
        }
    }

    fn validate(&self, result: &ExecutionResult) -> Result<String, String> {
        let digest = match self.operation.as_str() {
            "parse" => match result.artifact.as_ref() {
                Some(ExecutionArtifact::PoSummary(summary)) => digest_summary(summary)?,
                _ => {
                    return Err(format!(
                        "scenario {} expected PO summary artifact",
                        self.fixture
                    ));
                }
            },
            "stringify" | "merge" | "update-catalog" => {
                let content = match result.artifact.as_ref() {
                    Some(ExecutionArtifact::RenderedPo(content)) => Cow::Borrowed(content.as_str()),
                    Some(ExecutionArtifact::RenderedPoPath(path)) => {
                        Cow::Owned(fs::read_to_string(path).map_err(|error| {
                            format!(
                                "failed to read rendered PO output {}: {error}",
                                path.display()
                            )
                        })?)
                    }
                    _ => {
                        return Err(format!(
                            "scenario {} expected rendered PO artifact for {}",
                            self.fixture, self.operation
                        ));
                    }
                };
                let parsed = parse_po(&content)
                    .map_err(|error| format!("rendered output did not parse as PO: {error}"))?;
                digest_summary(&PoSemanticSummary::from_po_file(&parsed))?
            }
            "parse-icu" => match result.artifact.as_ref() {
                Some(ExecutionArtifact::IcuSummary(summary)) => digest_summary(summary)?,
                _ => {
                    return Err(format!(
                        "scenario {} expected ICU summary artifact",
                        self.fixture
                    ));
                }
            },
            other => return Err(format!("unsupported validation operation: {other}")),
        };
        Ok(digest)
    }

    fn run_internal_parse(
        &self,
        iterations: usize,
        borrowed: bool,
    ) -> Result<ExecutionResult, String> {
        let input = self
            .po_content
            .as_deref()
            .ok_or_else(|| "internal parse requires PO content".to_owned())?;
        let mut last_summary = None;
        let start = Instant::now();
        for _ in 0..iterations {
            let summary = if borrowed {
                let parsed = parse_po_borrowed(input)
                    .map_err(|error| format!("borrowed parse failed: {error}"))?;
                PoSemanticSummary::from_po_file(&parsed.into_owned())
            } else {
                let parsed =
                    parse_po(input).map_err(|error| format!("owned parse failed: {error}"))?;
                PoSemanticSummary::from_po_file(&parsed)
            };
            last_summary = Some(summary);
        }
        let elapsed = start.elapsed();
        let summary =
            last_summary.ok_or_else(|| "internal parse produced no summary".to_owned())?;
        let digest = digest_summary(&summary)?;
        Ok(ExecutionResult {
            tool_version: INTERNAL_TOOL_VERSION.to_owned(),
            reported_digest: digest,
            elapsed_ns: elapsed.as_nanos(),
            bytes_processed: (input.len() * iterations) as u64,
            items_processed: summary
                .items
                .len()
                .checked_mul(iterations)
                .map(|value| value as u64),
            messages_processed: None,
            artifact: Some(ExecutionArtifact::PoSummary(summary)),
        })
    }

    fn run_internal_stringify(
        &self,
        iterations: usize,
        capture_artifacts: bool,
    ) -> Result<ExecutionResult, String> {
        let po_file = self
            .po_file
            .as_ref()
            .ok_or_else(|| "internal stringify requires parsed PO file".to_owned())?;
        let mut last_rendered = None;
        let start = Instant::now();
        let mut bytes_processed = 0usize;
        for _ in 0..iterations {
            let rendered = stringify_po(po_file, &SerializeOptions::default());
            bytes_processed += rendered.len();
            last_rendered = Some(rendered);
        }
        let elapsed = start.elapsed();
        let rendered = last_rendered
            .ok_or_else(|| "internal stringify produced no rendered content".to_owned())?;
        let summary = {
            let parsed = parse_po(&rendered).map_err(|error| {
                format!("stringify validation parse failed for rendered output: {error}")
            })?;
            PoSemanticSummary::from_po_file(&parsed)
        };
        let digest = digest_summary(&summary)?;
        Ok(ExecutionResult {
            tool_version: INTERNAL_TOOL_VERSION.to_owned(),
            reported_digest: digest,
            elapsed_ns: elapsed.as_nanos(),
            bytes_processed: bytes_processed as u64,
            items_processed: summary
                .items
                .len()
                .checked_mul(iterations)
                .map(|value| value as u64),
            messages_processed: None,
            artifact: capture_artifacts.then_some(ExecutionArtifact::RenderedPo(rendered)),
        })
    }

    fn run_internal_merge(
        &self,
        iterations: usize,
        capture_artifacts: bool,
    ) -> Result<ExecutionResult, String> {
        let fixture = self
            .merge_fixture
            .as_ref()
            .ok_or_else(|| "internal merge requires merge fixture".to_owned())?;
        let mut last_rendered = None;
        let start = Instant::now();
        let mut bytes_processed = 0usize;
        for _ in 0..iterations {
            let rendered = merge_catalog(&fixture.existing_po, &fixture.merge_messages)
                .map_err(|error| format!("merge_catalog failed: {error}"))?;
            bytes_processed += rendered.len();
            last_rendered = Some(rendered);
        }
        let elapsed = start.elapsed();
        let rendered = last_rendered
            .ok_or_else(|| "internal merge produced no rendered content".to_owned())?;
        let summary = {
            let parsed = parse_po(&rendered)
                .map_err(|error| format!("merge output did not parse: {error}"))?;
            PoSemanticSummary::from_po_file(&parsed)
        };
        let digest = digest_summary(&summary)?;
        Ok(ExecutionResult {
            tool_version: INTERNAL_TOOL_VERSION.to_owned(),
            reported_digest: digest,
            elapsed_ns: elapsed.as_nanos(),
            bytes_processed: bytes_processed as u64,
            items_processed: summary
                .items
                .len()
                .checked_mul(iterations)
                .map(|value| value as u64),
            messages_processed: None,
            artifact: capture_artifacts.then_some(ExecutionArtifact::RenderedPo(rendered)),
        })
    }

    fn run_internal_update_catalog(
        &self,
        iterations: usize,
        capture_artifacts: bool,
    ) -> Result<ExecutionResult, String> {
        let fixture = self
            .merge_fixture
            .as_ref()
            .ok_or_else(|| "internal update-catalog requires merge fixture".to_owned())?;
        let mut last_rendered = None;
        let start = Instant::now();
        let mut bytes_processed = 0usize;
        let locale = fixture_locale(&self.fixture);
        let plural_encoding = fixture_plural_encoding(&self.fixture);
        for _ in 0..iterations {
            let updated = update_catalog(UpdateCatalogOptions {
                locale: locale.clone(),
                source_locale: "en".to_owned(),
                extracted: fixture.api_messages.clone(),
                existing: Some(fixture.existing_po.clone()),
                plural_encoding,
                ..UpdateCatalogOptions::default()
            })
            .map_err(|error| format!("update_catalog failed: {error}"))?;
            bytes_processed += updated.content.len();
            last_rendered = Some(updated.content);
        }
        let elapsed = start.elapsed();
        let rendered = last_rendered
            .ok_or_else(|| "internal update_catalog produced no rendered content".to_owned())?;
        let summary = {
            let parsed = parse_po(&rendered)
                .map_err(|error| format!("update_catalog output did not parse: {error}"))?;
            PoSemanticSummary::from_po_file(&parsed)
        };
        let digest = digest_summary(&summary)?;
        Ok(ExecutionResult {
            tool_version: INTERNAL_TOOL_VERSION.to_owned(),
            reported_digest: digest,
            elapsed_ns: elapsed.as_nanos(),
            bytes_processed: bytes_processed as u64,
            items_processed: summary
                .items
                .len()
                .checked_mul(iterations)
                .map(|value| value as u64),
            messages_processed: None,
            artifact: capture_artifacts.then_some(ExecutionArtifact::RenderedPo(rendered)),
        })
    }

    fn run_internal_parse_icu(
        &self,
        iterations: usize,
        capture_artifacts: bool,
    ) -> Result<ExecutionResult, String> {
        let messages = self
            .icu_messages
            .as_ref()
            .ok_or_else(|| "internal parse-icu requires ICU messages".to_owned())?;
        let mut last_summary = None;
        let total_bytes = messages.iter().map(String::len).sum::<usize>();
        let start = Instant::now();
        for _ in 0..iterations {
            let summary = IcuFixtureSummary::from_messages(messages)?;
            last_summary = Some(summary);
        }
        let elapsed = start.elapsed();
        let summary =
            last_summary.ok_or_else(|| "internal parse-icu produced no summary".to_owned())?;
        let digest = digest_summary(&summary)?;
        Ok(ExecutionResult {
            tool_version: INTERNAL_TOOL_VERSION.to_owned(),
            reported_digest: digest,
            elapsed_ns: elapsed.as_nanos(),
            bytes_processed: (total_bytes * iterations) as u64,
            items_processed: None,
            messages_processed: Some((messages.len() * iterations) as u64),
            artifact: capture_artifacts.then_some(ExecutionArtifact::IcuSummary(summary)),
        })
    }

    fn run_node_adapter(
        &self,
        workspace: &Path,
        scenario: &BenchmarkScenario,
        iterations: usize,
        capture_artifacts: bool,
    ) -> Result<ExecutionResult, String> {
        let request = self.adapter_request(scenario, iterations, capture_artifacts)?;
        let script = workspace.join("benchmark").join("node").join("adapter.cjs");
        run_external_adapter(
            "node",
            &["--no-warnings", script.to_string_lossy().as_ref()],
            workspace,
            &request,
        )
    }

    fn run_python_adapter(
        &self,
        workspace: &Path,
        scenario: &BenchmarkScenario,
        iterations: usize,
        capture_artifacts: bool,
    ) -> Result<ExecutionResult, String> {
        let request = self.adapter_request(scenario, iterations, capture_artifacts)?;
        let script = workspace
            .join("benchmark")
            .join("python")
            .join("adapter.py");
        let python = preferred_python_program(workspace);
        let args = vec![script.into_os_string()];
        run_external_adapter(python.as_os_str(), &args, workspace, &request)
    }

    fn run_msgcat(
        &self,
        iterations: usize,
        capture_artifacts: bool,
    ) -> Result<ExecutionResult, String> {
        let input = self
            .po_input_path
            .as_ref()
            .ok_or_else(|| "msgcat requires PO input path".to_owned())?;
        let capture_path = self._tempdir.path().join("msgcat-output.po");
        let start = Instant::now();
        let mut last_stdout = Vec::new();
        let mut bytes_processed = 0usize;
        for _ in 0..iterations {
            let output = Command::new("msgcat")
                .arg("--no-wrap")
                .arg(input)
                .output()
                .map_err(|error| format!("failed to launch msgcat: {error}"))?;
            if !output.status.success() {
                return Err(format!(
                    "msgcat failed: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                ));
            }
            bytes_processed += output.stdout.len();
            last_stdout = output.stdout;
        }
        let elapsed = start.elapsed();
        let rendered = String::from_utf8(last_stdout)
            .map_err(|error| format!("msgcat output was not valid UTF-8: {error}"))?;
        if capture_artifacts {
            fs::write(&capture_path, &rendered).map_err(|error| {
                format!(
                    "failed to persist msgcat output {}: {error}",
                    capture_path.display()
                )
            })?;
        }
        let summary = PoSemanticSummary::from_po_file(
            &parse_po(&rendered).map_err(|error| format!("msgcat output parse failed: {error}"))?,
        );
        let digest = digest_summary(&summary)?;
        Ok(ExecutionResult {
            tool_version: read_command_version("msgcat", &["--version"])?
                .lines()
                .next()
                .unwrap_or("msgcat")
                .to_owned(),
            reported_digest: digest,
            elapsed_ns: elapsed.as_nanos(),
            bytes_processed: bytes_processed as u64,
            items_processed: summary
                .items
                .len()
                .checked_mul(iterations)
                .map(|value| value as u64),
            messages_processed: None,
            artifact: capture_artifacts.then_some(ExecutionArtifact::RenderedPoPath(capture_path)),
        })
    }

    fn run_msgmerge(
        &self,
        iterations: usize,
        capture_artifacts: bool,
    ) -> Result<ExecutionResult, String> {
        let existing = self
            .existing_po_path
            .as_ref()
            .ok_or_else(|| "msgmerge requires existing PO input".to_owned())?;
        let pot = self
            .pot_path
            .as_ref()
            .ok_or_else(|| "msgmerge requires a POT template path".to_owned())?;
        let capture_path = self._tempdir.path().join("msgmerge-output.po");
        let start = Instant::now();
        let mut last_stdout = Vec::new();
        let mut bytes_processed = 0usize;
        for _ in 0..iterations {
            let output = Command::new("msgmerge")
                .arg("--no-wrap")
                .arg("--quiet")
                .arg(existing)
                .arg(pot)
                .output()
                .map_err(|error| format!("failed to launch msgmerge: {error}"))?;
            if !output.status.success() {
                return Err(format!(
                    "msgmerge failed: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                ));
            }
            bytes_processed += output.stdout.len();
            last_stdout = output.stdout;
        }
        let elapsed = start.elapsed();
        let rendered = String::from_utf8(last_stdout)
            .map_err(|error| format!("msgmerge output was not valid UTF-8: {error}"))?;
        if capture_artifacts {
            fs::write(&capture_path, &rendered).map_err(|error| {
                format!(
                    "failed to persist msgmerge output {}: {error}",
                    capture_path.display()
                )
            })?;
        }
        let summary = PoSemanticSummary::from_po_file(
            &parse_po(&rendered)
                .map_err(|error| format!("msgmerge output parse failed: {error}"))?,
        );
        let digest = digest_summary(&summary)?;
        Ok(ExecutionResult {
            tool_version: read_command_version("msgmerge", &["--version"])?
                .lines()
                .next()
                .unwrap_or("msgmerge")
                .to_owned(),
            reported_digest: digest,
            elapsed_ns: elapsed.as_nanos(),
            bytes_processed: bytes_processed as u64,
            items_processed: summary
                .items
                .len()
                .checked_mul(iterations)
                .map(|value| value as u64),
            messages_processed: None,
            artifact: capture_artifacts.then_some(ExecutionArtifact::RenderedPoPath(capture_path)),
        })
    }

    fn adapter_request(
        &self,
        scenario: &BenchmarkScenario,
        iterations: usize,
        capture_artifacts: bool,
    ) -> Result<AdapterRequest, String> {
        Ok(AdapterRequest {
            scenario_id: scenario.id.clone(),
            implementation: scenario.implementation.clone(),
            workload: scenario.workload.clone(),
            operation: scenario.operation.clone(),
            fixture: scenario.fixture.clone(),
            iterations,
            capture_artifacts,
            po_input_path: self
                .po_input_path
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned()),
            icu_messages_path: self
                .icu_messages_path
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned()),
            po_output_path: capture_artifacts.then(|| {
                self._tempdir
                    .path()
                    .join(format!("{}-output.po", scenario.implementation))
                    .to_string_lossy()
                    .into_owned()
            }),
        })
    }
}

#[derive(Debug, Clone)]
struct OwnedMergeFixture {
    existing_po: String,
    merge_messages: Vec<MergeExtractedMessage<'static>>,
    api_messages: Vec<ExtractedMessage>,
}

impl OwnedMergeFixture {
    fn from_fixture(fixture: &MergeFixture) -> Self {
        Self {
            existing_po: fixture.existing_po().to_owned(),
            merge_messages: fixture.extracted_messages().to_vec(),
            api_messages: fixture.api_extracted_messages().to_vec(),
        }
    }
}

fn build_merge_pot(fixture: &MergeFixture) -> String {
    let mut out = String::new();
    out.push_str("msgid \"\"\n");
    out.push_str("msgstr \"\"\n");
    out.push_str("\"Project-Id-Version: ferrocat benchmark template\\n\"\n");
    out.push_str("\"Content-Type: text/plain; charset=UTF-8\\n\"\n\n");

    for message in fixture.extracted_messages() {
        for comment in &message.extracted_comments {
            out.push_str("#. ");
            out.push_str(comment);
            out.push('\n');
        }
        if !message.references.is_empty() {
            out.push_str("#: ");
            let mut first = true;
            for reference in &message.references {
                if !first {
                    out.push(' ');
                }
                first = false;
                out.push_str(reference);
            }
            out.push('\n');
        }
        if !message.flags.is_empty() {
            out.push_str("#, ");
            let mut first = true;
            for flag in &message.flags {
                if !first {
                    out.push_str(", ");
                }
                first = false;
                out.push_str(flag);
            }
            out.push('\n');
        }
        if let Some(context) = &message.msgctxt {
            push_po_keyword(&mut out, "msgctxt", context);
        }
        push_po_keyword(&mut out, "msgid", &message.msgid);
        if let Some(plural) = &message.msgid_plural {
            push_po_keyword(&mut out, "msgid_plural", plural);
            out.push_str("msgstr[0] \"\"\n");
            out.push_str("msgstr[1] \"\"\n");
        } else {
            out.push_str("msgstr \"\"\n");
        }
        out.push('\n');
    }

    out
}

fn push_po_keyword(out: &mut String, keyword: &str, value: &str) {
    if !value.contains('\n') {
        out.push_str(keyword);
        out.push_str(" \"");
        out.push_str(&escape_po_text(value));
        out.push_str("\"\n");
        return;
    }

    out.push_str(keyword);
    out.push_str(" \"\"\n");
    let mut lines = value.split('\n').peekable();
    while let Some(line) = lines.next() {
        out.push('"');
        out.push_str(&escape_po_text(line));
        if lines.peek().is_some() {
            out.push_str("\\n");
        }
        out.push_str("\"\n");
    }
}

fn escape_po_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            _ => out.push(ch),
        }
    }
    out
}

fn load_fixture(name: &str) -> Result<Fixture, String> {
    fixture_by_name(name).ok_or_else(|| format!("unknown benchmark fixture: {name}"))
}

fn load_icu_fixture(name: &str) -> Result<IcuFixture, String> {
    icu_fixture_by_name(name).ok_or_else(|| format!("unknown ICU fixture: {name}"))
}

fn load_merge_fixture(name: &str) -> Result<MergeFixture, String> {
    merge_fixture_by_name(name).ok_or_else(|| format!("unknown merge fixture: {name}"))
}

fn fixture_locale(name: &str) -> Option<String> {
    if !name.starts_with("gettext-") {
        return Some("de".to_owned());
    }

    let mut parts = name.split('-');
    let _prefix = parts.next()?;
    let _family = parts.next()?;
    let locale = parts.next()?;
    Some(locale.to_owned())
}

fn fixture_plural_encoding(name: &str) -> PluralEncoding {
    if name.starts_with("gettext-") {
        PluralEncoding::Gettext
    } else {
        PluralEncoding::Icu
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct PoSemanticSummary {
    headers: Vec<PoHeaderSummary>,
    items: Vec<PoItemSummary>,
}

impl PoSemanticSummary {
    fn from_po_file(file: &PoFile) -> Self {
        let headers = file
            .headers
            .iter()
            .map(|header| PoHeaderSummary {
                key: header.key.clone(),
                value: header.value.clone(),
            })
            .collect::<Vec<_>>();
        let items = file
            .items
            .iter()
            .map(|item| PoItemSummary {
                msgctxt: item.msgctxt.clone(),
                msgid: item.msgid.clone(),
                msgid_plural: item.msgid_plural.clone(),
                msgstr: match &item.msgstr {
                    MsgStr::None => Vec::new(),
                    MsgStr::Singular(value) => vec![value.clone()],
                    MsgStr::Plural(values) => values.clone(),
                },
                obsolete: item.obsolete,
            })
            .collect::<Vec<_>>();
        Self { headers, items }.normalized()
    }

    fn normalized(mut self) -> Self {
        self.headers.retain(|header| {
            !header.value.is_empty()
                && !matches!(header.key.as_str(), "MIME-Version" | "X-Generator")
        });
        self.headers.sort();
        self.items.iter_mut().for_each(PoItemSummary::normalize);
        self.items.sort();
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
struct PoHeaderSummary {
    key: String,
    value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
struct PoItemSummary {
    msgctxt: Option<String>,
    msgid: String,
    msgid_plural: Option<String>,
    msgstr: Vec<String>,
    obsolete: bool,
}

impl PoItemSummary {
    fn normalize(&mut self) {
        if self.msgctxt.as_deref() == Some("") {
            self.msgctxt = None;
        }
        if self.msgid_plural.as_deref() == Some("") {
            self.msgid_plural = None;
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct IcuFixtureSummary {
    messages: Vec<IcuMessageSummary>,
}

impl IcuFixtureSummary {
    fn from_messages(messages: &[String]) -> Result<Self, String> {
        let mut summary = Vec::with_capacity(messages.len());
        for message in messages {
            let parsed = parse_icu(message)
                .map_err(|error| format!("failed to parse ICU benchmark message: {error}"))?;
            summary.push(IcuMessageSummary::from_message(&parsed));
        }
        Ok(Self { messages: summary })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct IcuMessageSummary {
    variable_names: Vec<String>,
    selector_kinds: Vec<String>,
    selectors: Vec<String>,
    plural_categories: Vec<String>,
    tag_names: Vec<String>,
    formatter_kinds: Vec<String>,
    literal_segments: usize,
    argument_count: usize,
    pound_count: usize,
    max_depth: usize,
}

impl IcuMessageSummary {
    fn from_message(message: &IcuMessage) -> Self {
        let mut collector = IcuCollector::default();
        collector.visit_nodes(&message.nodes, 1);
        Self {
            variable_names: collector.variable_names.into_iter().collect(),
            selector_kinds: collector.selector_kinds.into_iter().collect(),
            selectors: collector.selectors.into_iter().collect(),
            plural_categories: collector.plural_categories.into_iter().collect(),
            tag_names: collector.tag_names.into_iter().collect(),
            formatter_kinds: collector.formatter_kinds.into_iter().collect(),
            literal_segments: collector.literal_segments,
            argument_count: collector.argument_count,
            pound_count: collector.pound_count,
            max_depth: collector.max_depth,
        }
    }
}

#[derive(Default)]
struct IcuCollector {
    variable_names: BTreeSet<String>,
    selector_kinds: BTreeSet<String>,
    selectors: BTreeSet<String>,
    plural_categories: BTreeSet<String>,
    tag_names: BTreeSet<String>,
    formatter_kinds: BTreeSet<String>,
    literal_segments: usize,
    argument_count: usize,
    pound_count: usize,
    max_depth: usize,
}

impl IcuCollector {
    fn visit_nodes(&mut self, nodes: &[IcuNode], depth: usize) {
        self.max_depth = self.max_depth.max(depth);
        for node in nodes {
            self.visit_node(node, depth);
        }
    }

    fn visit_node(&mut self, node: &IcuNode, depth: usize) {
        self.max_depth = self.max_depth.max(depth);
        match node {
            IcuNode::Literal(_) => self.literal_segments += 1,
            IcuNode::Argument { name } => {
                self.argument_count += 1;
                self.variable_names.insert(name.clone());
            }
            IcuNode::Number { name, .. } => self.push_formatter("number", name),
            IcuNode::Date { name, .. } => self.push_formatter("date", name),
            IcuNode::Time { name, .. } => self.push_formatter("time", name),
            IcuNode::List { name, .. } => self.push_formatter("list", name),
            IcuNode::Duration { name, .. } => self.push_formatter("duration", name),
            IcuNode::Ago { name, .. } => self.push_formatter("ago", name),
            IcuNode::Name { name, .. } => self.push_formatter("name", name),
            IcuNode::Select { name, options } => {
                self.argument_count += 1;
                self.variable_names.insert(name.clone());
                self.selector_kinds.insert("select".to_owned());
                self.visit_options(options, depth + 1, false);
            }
            IcuNode::Plural {
                name,
                kind,
                options,
                ..
            } => {
                self.argument_count += 1;
                self.variable_names.insert(name.clone());
                self.selector_kinds.insert(match kind {
                    IcuPluralKind::Cardinal => "plural".to_owned(),
                    IcuPluralKind::Ordinal => "selectordinal".to_owned(),
                });
                self.visit_options(options, depth + 1, true);
            }
            IcuNode::Pound => self.pound_count += 1,
            IcuNode::Tag { name, children } => {
                self.tag_names.insert(name.clone());
                self.visit_nodes(children, depth + 1);
            }
        }
    }

    fn visit_options(&mut self, options: &[IcuOption], depth: usize, plural: bool) {
        for option in options {
            self.selectors.insert(option.selector.clone());
            if plural {
                self.plural_categories.insert(option.selector.clone());
            }
            self.visit_nodes(&option.value, depth);
        }
    }

    fn push_formatter(&mut self, kind: &str, name: &str) {
        self.argument_count += 1;
        self.variable_names.insert(name.to_owned());
        self.formatter_kinds.insert(kind.to_owned());
    }
}

fn digest_summary<T: Serialize>(value: &T) -> Result<String, String> {
    let canonical = canonical_json_string(value)?;
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    Ok(format!("{:x}", hasher.finalize()))
}

fn canonical_json_string<T: Serialize>(value: &T) -> Result<String, String> {
    let value = serde_json::to_value(value)
        .map_err(|error| format!("failed to build canonical JSON value: {error}"))?;
    let sorted = sort_json_value(value);
    serde_json::to_string(&sorted)
        .map_err(|error| format!("failed to render canonical JSON: {error}"))
}

fn sort_json_value(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Array(values) => {
            serde_json::Value::Array(values.into_iter().map(sort_json_value).collect::<Vec<_>>())
        }
        serde_json::Value::Object(values) => {
            let mut sorted = serde_json::Map::new();
            let mut keys = values.into_iter().collect::<Vec<_>>();
            keys.sort_by(|left, right| left.0.cmp(&right.0));
            for (key, value) in keys {
                sorted.insert(key, sort_json_value(value));
            }
            serde_json::Value::Object(sorted)
        }
        other => other,
    }
}

#[derive(Debug, Serialize)]
struct CompareReport {
    profile: String,
    generated_at: String,
    reference_host_policy: String,
    environment: EnvironmentMetadata,
    scenarios: Vec<ScenarioReport>,
}

#[derive(Debug, Serialize)]
struct ScenarioReport {
    id: String,
    comparison_group: String,
    workload: String,
    operation: String,
    fixture: String,
    implementation: String,
    tool_version: String,
    iterations_per_sample: usize,
    warmup_runs: usize,
    measured_runs: usize,
    semantic_digest: String,
    statistics: ScenarioStatistics,
    samples: Vec<ScenarioSampleReport>,
}

#[derive(Debug, Serialize)]
struct ScenarioSampleReport {
    elapsed_ns: u128,
    bytes_processed: u64,
    items_processed: Option<u64>,
    messages_processed: Option<u64>,
    mib_per_sec: f64,
    units_per_sec: f64,
}

impl ScenarioSampleReport {
    fn from_execution(sample: ExecutionResult) -> Self {
        let elapsed_seconds = nanos_to_seconds(sample.elapsed_ns);
        let units = sample
            .items_processed
            .or(sample.messages_processed)
            .unwrap_or(0) as f64;
        Self {
            elapsed_ns: sample.elapsed_ns,
            bytes_processed: sample.bytes_processed,
            items_processed: sample.items_processed,
            messages_processed: sample.messages_processed,
            mib_per_sec: throughput_mib(sample.bytes_processed, elapsed_seconds),
            units_per_sec: throughput_units(units, elapsed_seconds),
        }
    }
}

#[derive(Debug, Serialize)]
struct ScenarioStatistics {
    median_elapsed_ns: u128,
    min_elapsed_ns: u128,
    max_elapsed_ns: u128,
    stddev_elapsed_ns: f64,
    median_mib_per_sec: f64,
    median_units_per_sec: f64,
}

impl ScenarioStatistics {
    fn from_samples(samples: &[ExecutionResult]) -> Self {
        let mut elapsed = samples
            .iter()
            .map(|sample| sample.elapsed_ns)
            .collect::<Vec<_>>();
        elapsed.sort_unstable();
        let median_elapsed_ns = elapsed[elapsed.len() / 2];
        let min_elapsed_ns = *elapsed.first().unwrap_or(&0);
        let max_elapsed_ns = *elapsed.last().unwrap_or(&0);
        let mean = elapsed.iter().map(|value| *value as f64).sum::<f64>() / elapsed.len() as f64;
        let variance = elapsed
            .iter()
            .map(|value| {
                let delta = *value as f64 - mean;
                delta * delta
            })
            .sum::<f64>()
            / elapsed.len() as f64;

        let mut sample_reports = samples
            .iter()
            .map(|sample| {
                let seconds = nanos_to_seconds(sample.elapsed_ns);
                (
                    throughput_mib(sample.bytes_processed, seconds),
                    throughput_units(
                        sample
                            .items_processed
                            .or(sample.messages_processed)
                            .unwrap_or(0) as f64,
                        seconds,
                    ),
                )
            })
            .collect::<Vec<_>>();
        sample_reports.sort_by(|left, right| left.0.total_cmp(&right.0));
        let median_mib_per_sec = sample_reports
            .get(sample_reports.len() / 2)
            .map(|entry| entry.0)
            .unwrap_or(0.0);
        sample_reports.sort_by(|left, right| left.1.total_cmp(&right.1));
        let median_units_per_sec = sample_reports
            .get(sample_reports.len() / 2)
            .map(|entry| entry.1)
            .unwrap_or(0.0);

        Self {
            median_elapsed_ns,
            min_elapsed_ns,
            max_elapsed_ns,
            stddev_elapsed_ns: variance.sqrt(),
            median_mib_per_sec,
            median_units_per_sec,
        }
    }
}

fn nanos_to_seconds(value: u128) -> f64 {
    value as f64 / 1_000_000_000.0
}

fn throughput_mib(bytes: u64, seconds: f64) -> f64 {
    if seconds <= 0.0 {
        return f64::INFINITY;
    }
    bytes as f64 / (1024.0 * 1024.0 * seconds)
}

fn throughput_units(units: f64, seconds: f64) -> f64 {
    if seconds <= 0.0 {
        return f64::INFINITY;
    }
    units / seconds
}

#[derive(Debug, Clone)]
struct ExecutionResult {
    tool_version: String,
    reported_digest: String,
    elapsed_ns: u128,
    bytes_processed: u64,
    items_processed: Option<u64>,
    messages_processed: Option<u64>,
    artifact: Option<ExecutionArtifact>,
}

#[derive(Debug, Clone)]
enum ExecutionArtifact {
    PoSummary(PoSemanticSummary),
    IcuSummary(IcuFixtureSummary),
    RenderedPo(String),
    RenderedPoPath(PathBuf),
}

#[derive(Debug, Serialize)]
struct EnvironmentMetadata {
    git_sha: String,
    host_identifier: String,
    os: String,
    cpu_model: String,
    rustc_version: String,
    node_version: String,
    python_version: String,
    msgmerge_version: String,
    msgcat_version: String,
    node_adapter_version: String,
    python_adapter_version: String,
}

#[derive(Debug)]
struct BenchmarkEnvironment {
    git_sha: String,
    host_identifier: String,
    os: String,
    cpu_model: String,
    rustc_version: String,
    node_version: String,
    python_version: String,
    msgmerge_version: String,
    msgcat_version: String,
    node_adapter_version: String,
    python_adapter_version: String,
    python_program: PathBuf,
}

impl BenchmarkEnvironment {
    fn detect(workspace: &Path, path_override: Option<&OsStr>) -> Result<Self, String> {
        let mut errors = Vec::new();
        let python_program = preferred_python_program(workspace);

        let rustc_version =
            match read_command_version_with_path("rustc", &["--version"], path_override) {
                Ok(version) => version,
                Err(error) => {
                    errors.push(error);
                    String::new()
                }
            };
        let node_version =
            match read_command_version_with_path("node", &["--version"], path_override) {
                Ok(version) => version,
                Err(error) => {
                    errors.push(error);
                    String::new()
                }
            };
        let python_version = match read_command_version_for_program(
            &python_program,
            &["--version"],
            workspace,
            path_override,
        ) {
            Ok(version) => version,
            Err(error) => {
                errors.push(error);
                String::new()
            }
        };
        let msgmerge_version =
            match read_command_version_with_path("msgmerge", &["--version"], path_override) {
                Ok(version) => version,
                Err(error) => {
                    errors.push(error);
                    String::new()
                }
            };
        let msgcat_version =
            match read_command_version_with_path("msgcat", &["--version"], path_override) {
                Ok(version) => version,
                Err(error) => {
                    errors.push(error);
                    String::new()
                }
            };
        let node_adapter_version = match run_command_capture_with_path(
            "node",
            &[
                OsString::from("--no-warnings"),
                workspace
                    .join("benchmark")
                    .join("node")
                    .join("adapter.cjs")
                    .into_os_string(),
                OsString::from("--check"),
            ],
            workspace,
            path_override,
        ) {
            Ok(output) => output.stdout.trim().to_owned(),
            Err(error) => {
                errors.push(error);
                String::new()
            }
        };
        let python_adapter_version = match run_command_capture_with_path(
            python_program.as_os_str(),
            &[
                workspace
                    .join("benchmark")
                    .join("python")
                    .join("adapter.py")
                    .into_os_string(),
                OsString::from("--check"),
            ],
            workspace,
            path_override,
        ) {
            Ok(output) => output.stdout.trim().to_owned(),
            Err(error) => {
                errors.push(error);
                String::new()
            }
        };

        if !errors.is_empty() {
            return Err(format!(
                "benchmark environment verification failed:\n- {}",
                errors.join("\n- ")
            ));
        }

        Ok(Self {
            git_sha: read_git_sha(workspace),
            host_identifier: detect_host_identifier(path_override),
            os: format!("{}-{}", env::consts::OS, env::consts::ARCH),
            cpu_model: detect_cpu_model(path_override),
            rustc_version,
            node_version,
            python_version,
            msgmerge_version,
            msgcat_version,
            node_adapter_version,
            python_adapter_version,
            python_program,
        })
    }

    fn metadata(&self) -> EnvironmentMetadata {
        EnvironmentMetadata {
            git_sha: self.git_sha.clone(),
            host_identifier: self.host_identifier.clone(),
            os: self.os.clone(),
            cpu_model: self.cpu_model.clone(),
            rustc_version: self.rustc_version.clone(),
            node_version: self.node_version.clone(),
            python_version: self.python_version.clone(),
            msgmerge_version: self.msgmerge_version.clone(),
            msgcat_version: self.msgcat_version.clone(),
            node_adapter_version: self.node_adapter_version.clone(),
            python_adapter_version: self.python_adapter_version.clone(),
        }
    }
}

fn read_git_sha(workspace: &Path) -> String {
    match run_command_capture("git", &["rev-parse", "HEAD"], workspace) {
        Ok(output) => output.stdout.trim().to_owned(),
        Err(_) => "unknown".to_owned(),
    }
}

fn detect_host_identifier(path_override: Option<&OsStr>) -> String {
    if let Ok(hostname) = env::var("HOSTNAME") {
        if !hostname.trim().is_empty() {
            return hostname;
        }
    }
    run_command_capture_with_path(
        "hostname",
        &[] as &[&str],
        &workspace_root().unwrap_or_else(|_| PathBuf::from(".")),
        path_override,
    )
    .map(|output| output.stdout.trim().to_owned())
    .unwrap_or_else(|_| "unknown-host".to_owned())
}

fn detect_cpu_model(path_override: Option<&OsStr>) -> String {
    let workspace = workspace_root().unwrap_or_else(|_| PathBuf::from("."));
    if env::consts::OS == "macos" {
        if let Ok(output) = run_command_capture_with_path(
            "sysctl",
            &["-n", "machdep.cpu.brand_string"],
            &workspace,
            path_override,
        ) {
            let value = output.stdout.trim();
            if !value.is_empty() {
                return value.to_owned();
            }
        }
    }
    if let Ok(output) =
        run_command_capture_with_path("lscpu", &[] as &[&str], &workspace, path_override)
    {
        for line in output.stdout.lines() {
            if let Some(value) = line.strip_prefix("Model name:") {
                let trimmed = value.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_owned();
                }
            }
        }
    }
    "unknown-cpu".to_owned()
}

fn read_command_version(program: &str, args: &[&str]) -> Result<String, String> {
    read_command_version_with_path(program, args, None)
}

fn read_command_version_for_program(
    program: &Path,
    args: &[&str],
    cwd: &Path,
    path_override: Option<&OsStr>,
) -> Result<String, String> {
    let output = run_command_capture_with_path(program.as_os_str(), args, cwd, path_override)?;
    let version = output.stdout.trim();
    if version.is_empty() {
        return Err(format!("{} produced no version output", program.display()));
    }
    Ok(version.to_owned())
}

fn read_command_version_with_path(
    program: &str,
    args: &[&str],
    path_override: Option<&OsStr>,
) -> Result<String, String> {
    let workspace = workspace_root()?;
    let output = run_command_capture_with_path(program, args, &workspace, path_override)?;
    let version = output.stdout.trim();
    if version.is_empty() {
        return Err(format!("{program} produced no version output"));
    }
    Ok(version.to_owned())
}

#[derive(Debug)]
struct CommandCapture {
    stdout: String,
}

fn run_command_capture(program: &str, args: &[&str], cwd: &Path) -> Result<CommandCapture, String> {
    run_command_capture_with_path(program, args, cwd, None)
}

fn run_command_capture_with_path(
    program: impl AsRef<OsStr>,
    args: &[impl AsRef<OsStr>],
    cwd: &Path,
    path_override: Option<&OsStr>,
) -> Result<CommandCapture, String> {
    let program = program.as_ref();
    let program_label = program.to_string_lossy().into_owned();
    let mut command = Command::new(program);
    command.current_dir(cwd);
    if let Some(path_override) = path_override {
        command.env("PATH", path_override);
    }
    for arg in args {
        command.arg(arg);
    }

    let output = command
        .output()
        .map_err(|error| format!("failed to launch {program_label}: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "{program_label} exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(CommandCapture {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
    })
}

fn preferred_python_program(workspace: &Path) -> PathBuf {
    let candidates = [
        workspace
            .join("benchmark")
            .join("python")
            .join(".venv")
            .join("bin")
            .join("python3"),
        workspace
            .join("benchmark")
            .join("python")
            .join(".venv")
            .join("bin")
            .join("python"),
        workspace
            .join("benchmark")
            .join("python")
            .join(".venv")
            .join("Scripts")
            .join("python.exe"),
    ];

    for candidate in candidates {
        if candidate.is_file() {
            return candidate;
        }
    }

    PathBuf::from("python3")
}

#[derive(Debug, Serialize)]
struct AdapterRequest {
    scenario_id: String,
    implementation: String,
    workload: String,
    operation: String,
    fixture: String,
    iterations: usize,
    capture_artifacts: bool,
    po_input_path: Option<String>,
    icu_messages_path: Option<String>,
    po_output_path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AdapterResponse {
    implementation: String,
    workload: String,
    fixture: String,
    success: bool,
    semantic_digest: String,
    elapsed_ns: u128,
    bytes_processed: u64,
    items_processed: Option<u64>,
    messages_processed: Option<u64>,
    tool_version: String,
    po_summary: Option<PoSemanticSummary>,
    icu_summary: Option<IcuFixtureSummary>,
    po_output_path: Option<String>,
}

fn run_external_adapter(
    program: impl AsRef<OsStr>,
    args: &[impl AsRef<OsStr>],
    workspace: &Path,
    request: &AdapterRequest,
) -> Result<ExecutionResult, String> {
    let input = serde_json::to_vec(request)
        .map_err(|error| format!("failed to serialize adapter request: {error}"))?;
    let program = program.as_ref();
    let program_label = program.to_string_lossy().into_owned();
    let mut command = Command::new(program);
    command.current_dir(workspace);
    for arg in args {
        command.arg(arg);
    }
    command.stdin(std::process::Stdio::piped());
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|error| format!("failed to launch {program_label}: {error}"))?;
    let Some(mut stdin) = child.stdin.take() else {
        return Err(format!("failed to open stdin for {program_label}"));
    };
    use std::io::Write;
    stdin
        .write_all(&input)
        .map_err(|error| format!("failed to write adapter request: {error}"))?;
    drop(stdin);

    let output = child
        .wait_with_output()
        .map_err(|error| format!("failed to wait for {program_label}: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "{program_label} adapter failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let response: AdapterResponse = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("failed to parse adapter response: {error}"))?;
    if !response.success {
        return Err(format!(
            "{program_label} adapter reported unsuccessful execution for scenario {}",
            request.scenario_id
        ));
    }
    if response.implementation != request.implementation
        || response.workload != request.workload
        || response.fixture != request.fixture
    {
        return Err(format!(
            "{program_label} adapter response metadata mismatch for scenario {}",
            request.scenario_id
        ));
    }

    let artifact = if let Some(summary) = response.po_summary {
        Some(ExecutionArtifact::PoSummary(summary.normalized()))
    } else if let Some(summary) = response.icu_summary {
        Some(ExecutionArtifact::IcuSummary(summary))
    } else {
        response
            .po_output_path
            .map(|path| ExecutionArtifact::RenderedPoPath(PathBuf::from(path)))
    };

    Ok(ExecutionResult {
        tool_version: response.tool_version,
        reported_digest: response.semantic_digest,
        elapsed_ns: response.elapsed_ns,
        bytes_processed: response.bytes_processed,
        items_processed: response.items_processed,
        messages_processed: response.messages_processed,
        artifact,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn first_po_summary_difference(left: &PoSemanticSummary, right: &PoSemanticSummary) -> String {
        if left.headers != right.headers {
            return format!(
                "headers differ\nleft={}\nright={}",
                canonical_json_string(&left.headers)
                    .unwrap_or_else(|_| "<left-json-error>".to_owned()),
                canonical_json_string(&right.headers)
                    .unwrap_or_else(|_| "<right-json-error>".to_owned())
            );
        }

        if left.items.len() != right.items.len() {
            return format!(
                "item count differs: left={} right={}",
                left.items.len(),
                right.items.len()
            );
        }

        for (index, (left_item, right_item)) in left.items.iter().zip(&right.items).enumerate() {
            if left_item != right_item {
                return format!(
                    "item {} differs\nleft={}\nright={}",
                    index,
                    canonical_json_string(left_item)
                        .unwrap_or_else(|_| "<left-json-error>".to_owned()),
                    canonical_json_string(right_item)
                        .unwrap_or_else(|_| "<right-json-error>".to_owned())
                );
            }
        }

        "no summary difference found".to_owned()
    }

    #[test]
    #[ignore = "manual compatibility probe for external adapters"]
    fn debug_polib_gettext_ui_de_compatibility() {
        let workspace = workspace_root().expect("workspace");
        let scenarios = vec![
            BenchmarkScenario {
                id: "po-parse/gettext-ui-de-1000/ferrocat-owned".to_owned(),
                comparison_group: "po-parse/gettext-ui-de-1000".to_owned(),
                workload: "po-parse".to_owned(),
                operation: "parse".to_owned(),
                fixture: "gettext-ui-de-1000".to_owned(),
                implementation: "ferrocat-parse".to_owned(),
                warmup_runs: 0,
                measured_runs: 1,
                minimum_sample_millis: Some(1),
            },
            BenchmarkScenario {
                id: "po-parse/gettext-ui-de-1000/polib".to_owned(),
                comparison_group: "po-parse/gettext-ui-de-1000".to_owned(),
                workload: "po-parse".to_owned(),
                operation: "parse".to_owned(),
                fixture: "gettext-ui-de-1000".to_owned(),
                implementation: "polib".to_owned(),
                warmup_runs: 0,
                measured_runs: 1,
                minimum_sample_millis: Some(1),
            },
        ];

        let prepared = PreparedScenario::prepare(&workspace, &scenarios).expect("prepared");
        let internal = execute_scenario(&workspace, &prepared, &scenarios[0], 1, true)
            .expect("internal parse");
        let polib =
            execute_scenario(&workspace, &prepared, &scenarios[1], 1, true).expect("polib parse");

        let ExecutionArtifact::PoSummary(internal_summary) =
            internal.artifact.expect("internal artifact")
        else {
            panic!("internal scenario did not return a po summary");
        };
        let ExecutionArtifact::PoSummary(polib_summary) = polib.artifact.expect("polib artifact")
        else {
            panic!("polib scenario did not return a po summary");
        };

        assert_eq!(
            internal_summary,
            polib_summary,
            "{}",
            first_po_summary_difference(&internal_summary, &polib_summary)
        );
    }

    #[test]
    #[ignore = "manual compatibility probe for external adapters"]
    fn debug_pofile_gettext_ui_de_compatibility() {
        let workspace = workspace_root().expect("workspace");
        let scenarios = vec![
            BenchmarkScenario {
                id: "po-parse/gettext-ui-de-1000/ferrocat-owned".to_owned(),
                comparison_group: "po-parse/gettext-ui-de-1000".to_owned(),
                workload: "po-parse".to_owned(),
                operation: "parse".to_owned(),
                fixture: "gettext-ui-de-1000".to_owned(),
                implementation: "ferrocat-parse".to_owned(),
                warmup_runs: 0,
                measured_runs: 1,
                minimum_sample_millis: Some(1),
            },
            BenchmarkScenario {
                id: "po-parse/gettext-ui-de-1000/pofile".to_owned(),
                comparison_group: "po-parse/gettext-ui-de-1000".to_owned(),
                workload: "po-parse".to_owned(),
                operation: "parse".to_owned(),
                fixture: "gettext-ui-de-1000".to_owned(),
                implementation: "pofile".to_owned(),
                warmup_runs: 0,
                measured_runs: 1,
                minimum_sample_millis: Some(1),
            },
        ];

        let prepared = PreparedScenario::prepare(&workspace, &scenarios).expect("prepared");
        let internal = execute_scenario(&workspace, &prepared, &scenarios[0], 1, true)
            .expect("internal parse");
        let pofile =
            execute_scenario(&workspace, &prepared, &scenarios[1], 1, true).expect("pofile parse");

        let ExecutionArtifact::PoSummary(internal_summary) =
            internal.artifact.expect("internal artifact")
        else {
            panic!("internal scenario did not return a po summary");
        };
        let ExecutionArtifact::PoSummary(pofile_summary) =
            pofile.artifact.expect("pofile artifact")
        else {
            panic!("pofile scenario did not return a po summary");
        };

        assert_eq!(
            internal_summary,
            pofile_summary,
            "{}",
            first_po_summary_difference(&internal_summary, &pofile_summary)
        );
    }

    #[test]
    #[ignore = "manual compatibility probe for external adapters"]
    fn debug_msgmerge_gettext_ui_de_merge_compatibility() {
        let workspace = workspace_root().expect("workspace");
        let scenarios = vec![
            BenchmarkScenario {
                id: "po-merge/gettext-ui-de-1000/ferrocat".to_owned(),
                comparison_group: "po-merge/gettext-ui-de-1000".to_owned(),
                workload: "po-merge-update".to_owned(),
                operation: "merge".to_owned(),
                fixture: "gettext-ui-de-1000".to_owned(),
                implementation: "ferrocat-merge".to_owned(),
                warmup_runs: 0,
                measured_runs: 1,
                minimum_sample_millis: Some(1),
            },
            BenchmarkScenario {
                id: "po-merge/gettext-ui-de-1000/msgmerge".to_owned(),
                comparison_group: "po-merge/gettext-ui-de-1000".to_owned(),
                workload: "po-merge-update".to_owned(),
                operation: "merge".to_owned(),
                fixture: "gettext-ui-de-1000".to_owned(),
                implementation: "msgmerge".to_owned(),
                warmup_runs: 0,
                measured_runs: 1,
                minimum_sample_millis: Some(1),
            },
        ];

        let prepared = PreparedScenario::prepare(&workspace, &scenarios).expect("prepared");
        let internal = execute_scenario(&workspace, &prepared, &scenarios[0], 1, true)
            .expect("internal merge");
        let external =
            execute_scenario(&workspace, &prepared, &scenarios[1], 1, true).expect("msgmerge");

        let internal_rendered = match internal.artifact.expect("internal artifact") {
            ExecutionArtifact::RenderedPo(content) => content,
            ExecutionArtifact::RenderedPoPath(path) => {
                std::fs::read_to_string(path).expect("read internal rendered output")
            }
            other => panic!("unexpected internal artifact: {other:?}"),
        };
        let external_rendered = match external.artifact.expect("external artifact") {
            ExecutionArtifact::RenderedPo(content) => content,
            ExecutionArtifact::RenderedPoPath(path) => {
                std::fs::read_to_string(path).expect("read external rendered output")
            }
            other => panic!("unexpected external artifact: {other:?}"),
        };

        let internal_summary =
            PoSemanticSummary::from_po_file(&parse_po(&internal_rendered).expect("parse internal"));
        let external_summary =
            PoSemanticSummary::from_po_file(&parse_po(&external_rendered).expect("parse external"));

        assert_eq!(
            internal_summary,
            external_summary,
            "{}",
            first_po_summary_difference(&internal_summary, &external_summary)
        );
    }

    #[test]
    #[ignore = "manual compatibility probe for external adapters"]
    fn debug_msgmerge_gettext_ui_de_update_compatibility() {
        let workspace = workspace_root().expect("workspace");
        let scenarios = vec![
            BenchmarkScenario {
                id: "po-update/gettext-ui-de-1000/ferrocat".to_owned(),
                comparison_group: "po-update/gettext-ui-de-1000".to_owned(),
                workload: "po-merge-update".to_owned(),
                operation: "update-catalog".to_owned(),
                fixture: "gettext-ui-de-1000".to_owned(),
                implementation: "ferrocat-update-catalog".to_owned(),
                warmup_runs: 0,
                measured_runs: 1,
                minimum_sample_millis: Some(1),
            },
            BenchmarkScenario {
                id: "po-update/gettext-ui-de-1000/msgmerge".to_owned(),
                comparison_group: "po-update/gettext-ui-de-1000".to_owned(),
                workload: "po-merge-update".to_owned(),
                operation: "update-catalog".to_owned(),
                fixture: "gettext-ui-de-1000".to_owned(),
                implementation: "msgmerge".to_owned(),
                warmup_runs: 0,
                measured_runs: 1,
                minimum_sample_millis: Some(1),
            },
        ];

        let prepared = PreparedScenario::prepare(&workspace, &scenarios).expect("prepared");
        let internal = execute_scenario(&workspace, &prepared, &scenarios[0], 1, true)
            .expect("internal update");
        let external =
            execute_scenario(&workspace, &prepared, &scenarios[1], 1, true).expect("msgmerge");

        let internal_rendered = match internal.artifact.expect("internal artifact") {
            ExecutionArtifact::RenderedPo(content) => content,
            ExecutionArtifact::RenderedPoPath(path) => {
                std::fs::read_to_string(path).expect("read internal rendered output")
            }
            other => panic!("unexpected internal artifact: {other:?}"),
        };
        let external_rendered = match external.artifact.expect("external artifact") {
            ExecutionArtifact::RenderedPo(content) => content,
            ExecutionArtifact::RenderedPoPath(path) => {
                std::fs::read_to_string(path).expect("read external rendered output")
            }
            other => panic!("unexpected external artifact: {other:?}"),
        };

        let internal_summary =
            PoSemanticSummary::from_po_file(&parse_po(&internal_rendered).expect("parse internal"));
        let external_summary =
            PoSemanticSummary::from_po_file(&parse_po(&external_rendered).expect("parse external"));

        assert_eq!(
            internal_summary,
            external_summary,
            "{}",
            first_po_summary_difference(&internal_summary, &external_summary)
        );
    }

    #[test]
    fn canonical_po_summary_ignores_item_order() {
        let first = PoSemanticSummary::from_po_file(&PoFile {
            headers: vec![ferrocat_po::Header {
                key: "Language".to_owned(),
                value: "de".to_owned(),
            }],
            items: vec![
                ferrocat_po::PoItem {
                    msgid: "b".to_owned(),
                    msgstr: MsgStr::Singular("B".to_owned()),
                    ..ferrocat_po::PoItem::default()
                },
                ferrocat_po::PoItem {
                    msgid: "a".to_owned(),
                    msgstr: MsgStr::Singular("A".to_owned()),
                    ..ferrocat_po::PoItem::default()
                },
            ],
            ..PoFile::default()
        });
        let second = PoSemanticSummary::from_po_file(&PoFile {
            headers: vec![ferrocat_po::Header {
                key: "Language".to_owned(),
                value: "de".to_owned(),
            }],
            items: vec![
                ferrocat_po::PoItem {
                    msgid: "a".to_owned(),
                    msgstr: MsgStr::Singular("A".to_owned()),
                    ..ferrocat_po::PoItem::default()
                },
                ferrocat_po::PoItem {
                    msgid: "b".to_owned(),
                    msgstr: MsgStr::Singular("B".to_owned()),
                    ..ferrocat_po::PoItem::default()
                },
            ],
            ..PoFile::default()
        });

        assert_eq!(
            digest_summary(&first).expect("digest"),
            digest_summary(&second).expect("digest")
        );
    }

    #[test]
    fn statistics_use_elapsed_distribution() {
        let stats = ScenarioStatistics::from_samples(&[
            ExecutionResult {
                tool_version: "tool".to_owned(),
                reported_digest: "a".to_owned(),
                elapsed_ns: 10,
                bytes_processed: 1024,
                items_processed: Some(10),
                messages_processed: None,
                artifact: None,
            },
            ExecutionResult {
                tool_version: "tool".to_owned(),
                reported_digest: "a".to_owned(),
                elapsed_ns: 30,
                bytes_processed: 1024,
                items_processed: Some(10),
                messages_processed: None,
                artifact: None,
            },
            ExecutionResult {
                tool_version: "tool".to_owned(),
                reported_digest: "a".to_owned(),
                elapsed_ns: 20,
                bytes_processed: 1024,
                items_processed: Some(10),
                messages_processed: None,
                artifact: None,
            },
        ]);

        assert_eq!(stats.median_elapsed_ns, 20);
        assert_eq!(stats.min_elapsed_ns, 10);
        assert_eq!(stats.max_elapsed_ns, 30);
        assert!(stats.stddev_elapsed_ns > 0.0);
    }

    #[test]
    fn profile_loads_serious_v1() {
        let workspace = workspace_root().expect("workspace");
        let profile = BenchmarkProfile::load(&workspace, "serious-v1").expect("profile");
        assert_eq!(profile.name, "serious-v1");
        assert!(!profile.scenarios.is_empty());
    }

    #[test]
    fn profile_loads_gettext_compat_v1() {
        let workspace = workspace_root().expect("workspace");
        let profile = BenchmarkProfile::load(&workspace, "gettext-compat-v1").expect("profile");
        assert_eq!(profile.name, "gettext-compat-v1");
        assert!(!profile.scenarios.is_empty());
    }

    #[test]
    fn profile_loads_gettext_workflows_v1() {
        let workspace = workspace_root().expect("workspace");
        let profile = BenchmarkProfile::load(&workspace, "gettext-workflows-v1").expect("profile");
        assert_eq!(profile.name, "gettext-workflows-v1");
        assert!(!profile.scenarios.is_empty());
    }

    #[test]
    fn adapter_response_schema_accepts_optional_artifacts() {
        let response = serde_json::from_str::<AdapterResponse>(
            r#"{
                "implementation":"polib",
                "workload":"po-parse",
                "fixture":"mixed-1000",
                "success":true,
                "semantic_digest":"abc",
                "elapsed_ns":123,
                "bytes_processed":456,
                "items_processed":10,
                "messages_processed":null,
                "tool_version":"polib 1.0",
                "po_summary":{"headers":[],"items":[]},
                "icu_summary":null,
                "po_output_path":null
            }"#,
        )
        .expect("schema");

        assert_eq!(response.implementation, "polib");
        assert!(response.po_summary.is_some());
    }

    #[test]
    fn benchmark_environment_reports_missing_tools() {
        let workspace = workspace_root().expect("workspace");
        let error =
            BenchmarkEnvironment::detect(&workspace, Some(OsStr::new("/definitely-missing")))
                .expect_err("expected failure");
        assert!(error.contains("benchmark environment verification failed"));
    }

    #[test]
    fn run_profile_supports_internal_compare_groups() {
        let workspace = workspace_root().expect("workspace");
        let profile = BenchmarkProfile {
            name: "test-internal".to_owned(),
            minimum_sample_millis: 1,
            scenarios: vec![
                BenchmarkScenario {
                    id: "po-parse/mixed-1000/owned".to_owned(),
                    comparison_group: "po-parse/mixed-1000".to_owned(),
                    workload: "po-parse".to_owned(),
                    operation: "parse".to_owned(),
                    fixture: "mixed-1000".to_owned(),
                    implementation: "ferrocat-parse".to_owned(),
                    warmup_runs: 1,
                    measured_runs: 2,
                    minimum_sample_millis: Some(1),
                },
                BenchmarkScenario {
                    id: "po-parse/mixed-1000/borrowed".to_owned(),
                    comparison_group: "po-parse/mixed-1000".to_owned(),
                    workload: "po-parse".to_owned(),
                    operation: "parse".to_owned(),
                    fixture: "mixed-1000".to_owned(),
                    implementation: "ferrocat-parse-borrowed".to_owned(),
                    warmup_runs: 1,
                    measured_runs: 2,
                    minimum_sample_millis: Some(1),
                },
            ],
        };
        let environment = BenchmarkEnvironment {
            git_sha: "test-sha".to_owned(),
            host_identifier: "test-host".to_owned(),
            os: "test-os".to_owned(),
            cpu_model: "test-cpu".to_owned(),
            rustc_version: "rustc test".to_owned(),
            node_version: "node test".to_owned(),
            python_version: "python test".to_owned(),
            msgmerge_version: "msgmerge test".to_owned(),
            msgcat_version: "msgcat test".to_owned(),
            node_adapter_version: "node adapters".to_owned(),
            python_adapter_version: "python adapters".to_owned(),
            python_program: PathBuf::from("python3"),
        };

        let report = run_profile(&workspace, &environment, &profile).expect("report");
        assert_eq!(report.profile, "test-internal");
        assert_eq!(report.scenarios.len(), 2);
        assert_eq!(
            report.scenarios[0].semantic_digest,
            report.scenarios[1].semantic_digest
        );
    }

    #[test]
    fn owned_and_borrowed_match_on_gettext_plural_fixture() {
        let fixture =
            crate::fixtures::fixture_by_name("gettext-commerce-pl-1000").expect("fixture");
        let owned = ferrocat_po::parse_po(fixture.content()).expect("owned parse");
        let borrowed = ferrocat_po::parse_po_borrowed(fixture.content())
            .expect("borrowed parse")
            .into_owned();

        let owned_summary = PoSemanticSummary::from_po_file(&owned);
        let borrowed_summary = PoSemanticSummary::from_po_file(&borrowed);

        assert_eq!(
            canonical_json_string(&owned_summary).expect("owned json"),
            canonical_json_string(&borrowed_summary).expect("borrowed json")
        );
    }
}
