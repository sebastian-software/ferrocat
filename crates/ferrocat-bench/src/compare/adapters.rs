//! External benchmark adapter request and response handling.

use super::*;

#[derive(Debug, Serialize)]
pub(super) struct AdapterRequest {
    pub(super) scenario_id: String,
    pub(super) implementation: String,
    pub(super) workload: String,
    pub(super) operation: String,
    pub(super) fixture: String,
    pub(super) iterations: usize,
    pub(super) capture_artifacts: bool,
    pub(super) po_input_path: Option<String>,
    pub(super) existing_po_path: Option<String>,
    pub(super) pot_path: Option<String>,
    pub(super) icu_messages_path: Option<String>,
    pub(super) po_output_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct AdapterResponse {
    pub(super) implementation: String,
    pub(super) workload: String,
    pub(super) fixture: String,
    pub(super) success: bool,
    pub(super) semantic_digest: String,
    pub(super) elapsed_ns: u128,
    pub(super) bytes_processed: u64,
    pub(super) items_processed: Option<u64>,
    pub(super) messages_processed: Option<u64>,
    pub(super) tool_version: String,
    pub(super) po_summary: Option<PoSemanticSummary>,
    pub(super) icu_summary: Option<IcuFixtureSummary>,
    pub(super) po_output_path: Option<String>,
}

pub(super) fn run_external_adapter(
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
        baseline_elapsed_ns: None,
        bytes_processed: response.bytes_processed,
        items_processed: response.items_processed,
        messages_processed: response.messages_processed,
        artifact,
    })
}
