//! Benchmark profile schema, loading, and prepared execution state.

use super::*;

#[derive(Debug, Clone, Deserialize)]
pub(super) struct BenchmarkProfile {
    pub(super) name: String,
    #[serde(default = "default_minimum_sample_millis")]
    pub(super) minimum_sample_millis: u64,
    pub(super) scenarios: Vec<BenchmarkScenario>,
}

impl BenchmarkProfile {
    pub(super) fn load(workspace: &Path, profile_name: &str) -> Result<Self, String> {
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
        let profile: Self = serde_json::from_str(&content).map_err(|error| {
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

    pub(super) fn tool_requirement(&self) -> ToolRequirement {
        if self
            .scenarios
            .iter()
            .any(|scenario| !scenario.implementation.starts_with("ferrocat-"))
        {
            ToolRequirement::External
        } else {
            ToolRequirement::RustOnly
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ToolRequirement {
    RustOnly,
    External,
}

pub(super) const fn default_minimum_sample_millis() -> u64 {
    DEFAULT_MIN_SAMPLE_MILLIS
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct BenchmarkScenario {
    pub(super) id: String,
    pub(super) comparison_group: String,
    pub(super) workload: String,
    pub(super) operation: String,
    pub(super) fixture: String,
    pub(super) implementation: String,
    pub(super) warmup_runs: usize,
    pub(super) measured_runs: usize,
    pub(super) minimum_sample_millis: Option<u64>,
}

#[derive(Debug)]
pub(super) struct PreparedScenario {
    pub(super) operation: String,
    pub(super) fixture: String,
    pub(super) tempdir: TempDir,
    pub(super) po_input_path: Option<PathBuf>,
    pub(super) icu_messages_path: Option<PathBuf>,
    pub(super) existing_po_path: Option<PathBuf>,
    pub(super) pot_path: Option<PathBuf>,
    pub(super) po_content: Option<String>,
    pub(super) catalog_fcl_content: Option<String>,
    pub(super) po_file: Option<PoFile>,
    pub(super) merge_fixture: Option<OwnedMergeFixture>,
    pub(super) icu_messages: Option<Vec<String>>,
    pub(super) catalog_workflow: Option<CatalogWorkflowFixture>,
}

#[derive(Debug)]
pub(super) struct CliBaselineScenario {
    pub(super) label: String,
    pub(super) prepared: PreparedScenario,
}

#[derive(Debug)]
pub(super) struct ScenarioExecutionPlan {
    pub(super) scenario: BenchmarkScenario,
    pub(super) tool_version: String,
    pub(super) validated_digest: String,
    pub(super) iterations: usize,
    pub(super) cli_baseline: Option<CliBaselineScenario>,
}
