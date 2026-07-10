//! Host, toolchain, and external-tool environment detection.

use super::*;

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct EnvironmentMetadata {
    pub(super) git_sha: String,
    pub(super) system_label: String,
    pub(super) os: String,
    pub(super) cpu_model: String,
    pub(super) memory_bytes: u64,
    pub(super) rustc_version: String,
    pub(super) node_version: String,
    pub(super) python_version: String,
    pub(super) msgmerge_version: String,
    pub(super) msgcat_version: String,
    pub(super) node_adapter_version: String,
    pub(super) python_adapter_version: String,
}

#[derive(Debug)]
pub(super) struct BenchmarkEnvironment {
    pub(super) git_sha: String,
    pub(super) system_label: String,
    pub(super) os: String,
    pub(super) cpu_model: String,
    pub(super) memory_bytes: u64,
    pub(super) rustc_version: String,
    pub(super) node_version: String,
    pub(super) python_version: String,
    pub(super) msgmerge_version: String,
    pub(super) msgcat_version: String,
    pub(super) node_adapter_version: String,
    pub(super) python_adapter_version: String,
    pub(super) python_program: PathBuf,
}

impl BenchmarkEnvironment {
    pub(super) fn detect(
        workspace: &Path,
        path_override: Option<&OsStr>,
        tool_requirement: ToolRequirement,
    ) -> Result<Self, String> {
        let mut errors = Vec::new();
        let python_program = preferred_python_program(workspace);
        let os = format!("{}-{}", env::consts::OS, env::consts::ARCH);
        let cpu_model = detect_cpu_model(path_override);
        let memory_bytes = detect_memory_bytes(path_override);

        let rustc_version =
            match read_command_version_with_path("rustc", &["--version"], path_override) {
                Ok(version) => version,
                Err(error) => {
                    errors.push(error);
                    String::new()
                }
            };
        let mut external = ExternalToolVersions::not_required();
        if tool_requirement == ToolRequirement::External {
            external = detect_external_tool_versions(
                workspace,
                path_override,
                &python_program,
                &mut errors,
            );
        }

        if !errors.is_empty() {
            return Err(format!(
                "benchmark environment verification failed:\n- {}",
                errors.join("\n- ")
            ));
        }

        Ok(Self {
            git_sha: read_git_sha(workspace),
            system_label: build_system_label(&cpu_model, memory_bytes),
            os,
            cpu_model,
            memory_bytes,
            rustc_version,
            node_version: external.node_version,
            python_version: external.python_version,
            msgmerge_version: external.msgmerge_version,
            msgcat_version: external.msgcat_version,
            node_adapter_version: external.node_adapter_version,
            python_adapter_version: external.python_adapter_version,
            python_program,
        })
    }

    pub(super) fn metadata(&self) -> EnvironmentMetadata {
        EnvironmentMetadata {
            git_sha: self.git_sha.clone(),
            system_label: self.system_label.clone(),
            os: self.os.clone(),
            cpu_model: self.cpu_model.clone(),
            memory_bytes: self.memory_bytes,
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

#[derive(Debug)]
pub(super) struct ExternalToolVersions {
    pub(super) node_version: String,
    pub(super) python_version: String,
    pub(super) msgmerge_version: String,
    pub(super) msgcat_version: String,
    pub(super) node_adapter_version: String,
    pub(super) python_adapter_version: String,
}

impl ExternalToolVersions {
    pub(super) fn not_required() -> Self {
        Self {
            node_version: "not-required".to_owned(),
            python_version: "not-required".to_owned(),
            msgmerge_version: "not-required".to_owned(),
            msgcat_version: "not-required".to_owned(),
            node_adapter_version: "not-required".to_owned(),
            python_adapter_version: "not-required".to_owned(),
        }
    }
}

pub(super) fn detect_external_tool_versions(
    workspace: &Path,
    path_override: Option<&OsStr>,
    python_program: &Path,
    errors: &mut Vec<String>,
) -> ExternalToolVersions {
    let node_version = match read_command_version_with_path("node", &["--version"], path_override) {
        Ok(version) => version,
        Err(error) => {
            errors.push(error);
            String::new()
        }
    };
    let python_version = match read_command_version_for_program(
        python_program,
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

    ExternalToolVersions {
        node_version,
        python_version,
        msgmerge_version,
        msgcat_version,
        node_adapter_version,
        python_adapter_version,
    }
}

pub(super) fn read_git_sha(workspace: &Path) -> String {
    match run_command_capture("git", &["rev-parse", "HEAD"], workspace) {
        Ok(output) => output.stdout.trim().to_owned(),
        Err(_) => "unknown".to_owned(),
    }
}

pub(super) fn detect_cpu_model(path_override: Option<&OsStr>) -> String {
    let workspace = workspace_root().unwrap_or_else(|_| PathBuf::from("."));
    if env::consts::OS == "macos" {
        if let Some(value) = read_macos_sysctl_string("machdep.cpu.brand_string") {
            return value;
        }
        if let Ok(output) =
            run_command_capture_with_path("uname", &["-m"], &workspace, path_override)
        {
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
    if let Ok(output) = run_command_capture_with_path("uname", &["-m"], &workspace, path_override) {
        let value = output.stdout.trim();
        if !value.is_empty() {
            return value.to_owned();
        }
    }
    "unknown-cpu".to_owned()
}

pub(super) fn detect_memory_bytes(path_override: Option<&OsStr>) -> u64 {
    let workspace = workspace_root().unwrap_or_else(|_| PathBuf::from("."));
    if env::consts::OS == "macos"
        && let Some(bytes) = read_macos_sysctl_u64("hw.memsize")
    {
        return bytes;
    }
    if env::consts::OS == "linux"
        && let Ok(meminfo) = fs::read_to_string("/proc/meminfo")
    {
        for line in meminfo.lines() {
            if let Some(kb) = line
                .strip_prefix("MemTotal:")
                .and_then(|value| value.split_whitespace().next())
                .and_then(|raw| raw.parse::<u64>().ok())
            {
                return kb.saturating_mul(1024);
            }
        }
    }
    if env::consts::OS == "windows"
        && let Ok(output) = run_command_capture_with_path(
            "powershell",
            &[
                "-NoProfile",
                "-Command",
                "(Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory",
            ],
            &workspace,
            path_override,
        )
        && let Ok(bytes) = output.stdout.trim().parse::<u64>()
    {
        return bytes;
    }
    0
}

pub(super) fn build_system_label(cpu_model: &str, memory_bytes: u64) -> String {
    build_system_label_with_os(cpu_model, memory_bytes, &human_os_label())
}

pub(super) fn build_system_label_with_os(cpu_model: &str, memory_bytes: u64, os: &str) -> String {
    let cpu = if cpu_model.trim().is_empty() {
        "Unknown CPU"
    } else {
        cpu_model.trim()
    };
    if memory_bytes == 0 {
        return format!("{cpu} ({os})");
    }
    format!("{cpu} ({}, {os})", format_memory_label(memory_bytes))
}

pub(super) fn human_os_label() -> String {
    let os = match env::consts::OS {
        "macos" => "macOS",
        "windows" => "Windows",
        "linux" => "Linux",
        other => other,
    };
    let arch = match env::consts::ARCH {
        "aarch64" => "arm64",
        other => other,
    };
    format!("{os} {arch}")
}

pub(super) fn format_memory_label(memory_bytes: u64) -> String {
    if memory_bytes == 0 {
        return "unknown RAM".to_owned();
    }

    let gib = f64_from_u64(memory_bytes) / 1024_f64.powi(3);
    let rounded = gib.round();
    if (gib - rounded).abs() < 0.05 {
        format!("{rounded:.0} GB RAM")
    } else {
        format!("{gib:.1} GB RAM")
    }
}

#[cfg(target_os = "macos")]
pub(super) fn read_macos_sysctl_string(name: &str) -> Option<String> {
    let name = CString::new(name).ok()?;
    let mut len = 0usize;
    // SAFETY: `name` is a valid NUL-terminated C string, the first call asks macOS
    // for the required buffer size, and the second call writes into an owned buffer
    // of that exact size.
    unsafe {
        if libc::sysctlbyname(
            name.as_ptr(),
            std::ptr::null_mut(),
            &raw mut len,
            std::ptr::null_mut(),
            0,
        ) != 0
            || len == 0
        {
            return None;
        }

        let mut buffer = vec![0u8; len];
        if libc::sysctlbyname(
            name.as_ptr(),
            buffer.as_mut_ptr().cast(),
            &raw mut len,
            std::ptr::null_mut(),
            0,
        ) != 0
        {
            return None;
        }

        if len > 0 {
            buffer.truncate(len.saturating_sub(1));
        }
        String::from_utf8(buffer)
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
    }
}

#[cfg(not(target_os = "macos"))]
pub(super) fn read_macos_sysctl_string(_name: &str) -> Option<String> {
    None
}

#[cfg(target_os = "macos")]
pub(super) fn read_macos_sysctl_u64(name: &str) -> Option<u64> {
    let name = CString::new(name).ok()?;
    let mut value = 0u64;
    let mut len = std::mem::size_of::<u64>();
    // SAFETY: `name` is a valid NUL-terminated C string and `value` points to a
    // properly sized writable `u64` buffer for `sysctlbyname`.
    unsafe {
        if libc::sysctlbyname(
            name.as_ptr(),
            (&raw mut value).cast(),
            &raw mut len,
            std::ptr::null_mut(),
            0,
        ) != 0
            || len != std::mem::size_of::<u64>()
        {
            return None;
        }
    }
    Some(value)
}

#[cfg(not(target_os = "macos"))]
pub(super) fn read_macos_sysctl_u64(_name: &str) -> Option<u64> {
    None
}

pub(super) fn read_command_version(program: &str, args: &[&str]) -> Result<String, String> {
    read_command_version_with_path(program, args, None)
}

pub(super) fn read_command_version_for_program(
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

pub(super) fn read_command_version_with_path(
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
pub(super) struct CommandCapture {
    pub(super) stdout: String,
}

pub(super) fn run_command_capture(
    program: &str,
    args: &[&str],
    cwd: &Path,
) -> Result<CommandCapture, String> {
    run_command_capture_with_path(program, args, cwd, None)
}

pub(super) fn run_command_capture_with_path(
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

pub(super) fn preferred_python_program(workspace: &Path) -> PathBuf {
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
