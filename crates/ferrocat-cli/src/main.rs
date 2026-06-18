use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use ferrocat_po::{
    CatalogAuditOptions, CatalogAuditReport, CatalogAuditSummary, CatalogMode, DiagnosticSeverity,
    NormalizedParsedCatalog, ParseCatalogOptions, audit_catalogs, parse_catalog,
};

const EXIT_AUDIT_FAILED: u8 = 1;
const EXIT_USAGE_OR_RUNTIME_ERROR: u8 = 2;

const USAGE: &str = "\
Usage:
  ferrocat audit --source-locale <locale> --source <path> --target <locale=path>... [options]

Commands:
  audit    Audit source and target catalogs for release readiness.

Audit options:
  --source-locale <locale>    Locale that represents the source catalog.
  --source <path>             Source catalog path.
  --target <locale=path>      Target catalog path. Repeat for multiple locales.
  --locale <locale>           Target locale filter. Repeat to request multiple target locales.
  --storage <po|ndjson>       Catalog storage format. Defaults to po.
  --format <text|json>        Output format. Defaults to text.
  -h, --help                  Show this help text.
";

fn main() -> ExitCode {
    let mut stdout = io::stdout().lock();
    match run_with_writer(env::args().skip(1), &mut stdout) {
        Ok(exit_code) => exit_code,
        Err(error) => {
            eprintln!("{error}");
            if matches!(error, CliError::Usage(_)) {
                eprintln!();
                eprint!("{USAGE}");
            }
            ExitCode::from(EXIT_USAGE_OR_RUNTIME_ERROR)
        }
    }
}

fn run_with_writer<I, W>(args: I, stdout: &mut W) -> Result<ExitCode, CliError>
where
    I: IntoIterator<Item = String>,
    W: Write,
{
    let mut args = args.into_iter();
    let Some(command) = args.next() else {
        write_usage(stdout)?;
        return Ok(ExitCode::SUCCESS);
    };

    match command.as_str() {
        "-h" | "--help" | "help" => {
            write_usage(stdout)?;
            Ok(ExitCode::SUCCESS)
        }
        "audit" => run_audit(args, stdout),
        other => Err(CliError::usage(format!("unknown command `{other}`"))),
    }
}

fn run_audit<I, W>(args: I, stdout: &mut W) -> Result<ExitCode, CliError>
where
    I: IntoIterator<Item = String>,
    W: Write,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if args
        .iter()
        .any(|arg| matches!(arg.as_str(), "-h" | "--help"))
    {
        write_usage(stdout)?;
        return Ok(ExitCode::SUCCESS);
    }

    let config = AuditConfig::parse(args)?;
    let report = config.run()?;

    match config.format {
        OutputFormat::Text => write_text_report(&report, stdout)?,
        OutputFormat::Json => {
            serde_json::to_writer_pretty(&mut *stdout, &json_report(&report)).map_err(|error| {
                CliError::runtime(format!("failed to write JSON report: {error}"))
            })?;
            writeln!(stdout)?;
        }
    }

    if report.has_errors() {
        Ok(ExitCode::from(EXIT_AUDIT_FAILED))
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

fn write_usage<W>(stdout: &mut W) -> Result<(), CliError>
where
    W: Write,
{
    write!(stdout, "{USAGE}")?;
    Ok(())
}

fn write_text_report<W>(report: &CatalogAuditReport, stdout: &mut W) -> Result<(), CliError>
where
    W: Write,
{
    let CatalogAuditSummary {
        source_messages,
        target_locales,
        diagnostics,
        errors,
        warnings,
        infos,
    } = report.summary;

    writeln!(
        stdout,
        "Catalog audit: {source_messages} source messages, {target_locales} target locales, {diagnostics} diagnostics ({errors} errors, {warnings} warnings, {infos} info)"
    )?;

    if report.diagnostics.is_empty() {
        writeln!(stdout, "No catalog audit diagnostics.")?;
        return Ok(());
    }

    for diagnostic in &report.diagnostics {
        let severity = match diagnostic.severity {
            DiagnosticSeverity::Info => "info",
            DiagnosticSeverity::Warning => "warning",
            DiagnosticSeverity::Error => "error",
        };
        let location = diagnostic
            .source_key
            .as_ref()
            .map(format_source_key)
            .unwrap_or_else(|| "catalog".to_owned());

        if let Some(name) = &diagnostic.name {
            writeln!(
                stdout,
                "{severity} {} [{location}; {name}]: {}",
                diagnostic.code, diagnostic.message
            )?;
        } else {
            writeln!(
                stdout,
                "{severity} {} [{location}]: {}",
                diagnostic.code, diagnostic.message
            )?;
        }
    }

    Ok(())
}

fn format_source_key(source_key: &ferrocat_po::CatalogAuditMessageRef) -> String {
    let locale = source_key.locale.as_deref().unwrap_or("unknown-locale");
    match source_key.msgctxt.as_deref() {
        Some(msgctxt) => format!("{locale} {msgctxt:?} / {:?}", source_key.msgid),
        None => format!("{locale} {:?}", source_key.msgid),
    }
}

fn json_report(report: &CatalogAuditReport) -> serde_json::Value {
    serde_json::json!({
        "summary": {
            "source_messages": report.summary.source_messages,
            "target_locales": report.summary.target_locales,
            "diagnostics": report.summary.diagnostics,
            "errors": report.summary.errors,
            "warnings": report.summary.warnings,
            "infos": report.summary.infos,
        },
        "diagnostics": report
            .diagnostics
            .iter()
            .map(json_diagnostic)
            .collect::<Vec<_>>(),
    })
}

fn json_diagnostic(diagnostic: &ferrocat_po::CatalogAuditDiagnostic) -> serde_json::Value {
    let severity = match diagnostic.severity {
        DiagnosticSeverity::Info => "info",
        DiagnosticSeverity::Warning => "warning",
        DiagnosticSeverity::Error => "error",
    };
    let source_key = diagnostic
        .source_key
        .as_ref()
        .map(|source_key| {
            serde_json::json!({
                "locale": source_key.locale.as_deref(),
                "msgid": source_key.msgid.as_str(),
                "msgctxt": source_key.msgctxt.as_deref(),
            })
        })
        .unwrap_or(serde_json::Value::Null);

    serde_json::json!({
        "severity": severity,
        "code": diagnostic.code.as_str(),
        "message": diagnostic.message.as_str(),
        "source_key": source_key,
        "name": diagnostic.name.as_deref(),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Text,
    Json,
}

impl OutputFormat {
    fn parse(value: &str) -> Result<Self, CliError> {
        match value {
            "text" => Ok(Self::Text),
            "json" => Ok(Self::Json),
            other => Err(CliError::usage(format!(
                "unsupported output format `{other}`; expected `text` or `json`"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CliStorageFormat {
    Po,
    Ndjson,
}

impl CliStorageFormat {
    fn parse(value: &str) -> Result<Self, CliError> {
        match value {
            "po" => Ok(Self::Po),
            "ndjson" => Ok(Self::Ndjson),
            other => Err(CliError::usage(format!(
                "unsupported storage format `{other}`; expected `po` or `ndjson`"
            ))),
        }
    }

    const fn catalog_mode(self) -> CatalogMode {
        match self {
            Self::Po => CatalogMode::IcuPo,
            Self::Ndjson => CatalogMode::IcuNdjson,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TargetCatalog {
    locale: String,
    path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AuditConfig {
    source_locale: String,
    source_path: PathBuf,
    targets: Vec<TargetCatalog>,
    locale_filters: Vec<String>,
    storage_format: CliStorageFormat,
    format: OutputFormat,
}

impl AuditConfig {
    fn parse<I>(args: I) -> Result<Self, CliError>
    where
        I: IntoIterator<Item = String>,
    {
        let mut source_locale = None;
        let mut source_path = None;
        let mut targets = Vec::new();
        let mut locale_filters = Vec::new();
        let mut storage_format = CliStorageFormat::Po;
        let mut format = OutputFormat::Text;

        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-h" | "--help" => {
                    return Err(CliError::usage(USAGE.trim_end().to_owned()));
                }
                "--source-locale" => {
                    source_locale = Some(next_value(&mut args, "--source-locale")?);
                }
                "--source" => {
                    source_path = Some(PathBuf::from(next_value(&mut args, "--source")?));
                }
                "--target" => {
                    targets.push(parse_target(&next_value(&mut args, "--target")?)?);
                }
                "--locale" => {
                    let locale = next_value(&mut args, "--locale")?;
                    if locale.trim().is_empty() {
                        return Err(CliError::usage("--locale must not be empty"));
                    }
                    locale_filters.push(locale);
                }
                "--storage" => {
                    storage_format = CliStorageFormat::parse(&next_value(&mut args, "--storage")?)?;
                }
                "--format" => {
                    format = OutputFormat::parse(&next_value(&mut args, "--format")?)?;
                }
                other => {
                    return Err(CliError::usage(format!("unknown audit option `{other}`")));
                }
            }
        }

        let source_locale =
            source_locale.ok_or_else(|| CliError::usage("audit requires --source-locale"))?;
        if source_locale.trim().is_empty() {
            return Err(CliError::usage("--source-locale must not be empty"));
        }
        let source_path = source_path.ok_or_else(|| CliError::usage("audit requires --source"))?;
        if targets.is_empty() {
            return Err(CliError::usage("audit requires at least one --target"));
        }

        Ok(Self {
            source_locale,
            source_path,
            targets,
            locale_filters,
            storage_format,
            format,
        })
    }

    fn run(&self) -> Result<CatalogAuditReport, CliError> {
        let mut catalogs = Vec::with_capacity(self.targets.len() + 1);
        catalogs.push(load_catalog(
            &self.source_path,
            &self.source_locale,
            &self.source_locale,
            self.storage_format,
        )?);
        for target in &self.targets {
            catalogs.push(load_catalog(
                &target.path,
                &target.locale,
                &self.source_locale,
                self.storage_format,
            )?);
        }

        let catalog_refs = catalogs.iter().collect::<Vec<_>>();
        let locale_filters = self
            .locale_filters
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        let options = CatalogAuditOptions {
            source_locale: &self.source_locale,
            locales: &locale_filters,
            ..CatalogAuditOptions::default()
        };

        audit_catalogs(&catalog_refs, &options)
            .map_err(|error| CliError::runtime(format!("catalog audit failed: {error}")))
    }
}

fn load_catalog(
    path: &Path,
    locale: &str,
    source_locale: &str,
    storage_format: CliStorageFormat,
) -> Result<NormalizedParsedCatalog, CliError> {
    let content = fs::read_to_string(path).map_err(|error| {
        CliError::runtime(format!("failed to read {}: {error}", path.display()))
    })?;
    parse_catalog(ParseCatalogOptions {
        content: &content,
        locale: Some(locale),
        source_locale,
        mode: storage_format.catalog_mode(),
        ..ParseCatalogOptions::new(&content, source_locale)
    })
    .and_then(ferrocat_po::ParsedCatalog::into_normalized_view)
    .map_err(|error| CliError::runtime(format!("failed to parse {}: {error}", path.display())))
}

fn next_value<I>(args: &mut I, flag: &str) -> Result<String, CliError>
where
    I: Iterator<Item = String>,
{
    args.next()
        .ok_or_else(|| CliError::usage(format!("{flag} requires a value")))
}

fn parse_target(value: &str) -> Result<TargetCatalog, CliError> {
    let Some((locale, path)) = value.split_once('=') else {
        return Err(CliError::usage(
            "--target expects <locale=path>, for example --target de=locales/de.po",
        ));
    };
    if locale.trim().is_empty() {
        return Err(CliError::usage("--target locale must not be empty"));
    }
    if path.trim().is_empty() {
        return Err(CliError::usage("--target path must not be empty"));
    }

    Ok(TargetCatalog {
        locale: locale.to_owned(),
        path: PathBuf::from(path),
    })
}

#[derive(Debug)]
enum CliError {
    Usage(String),
    Runtime(String),
}

impl CliError {
    fn usage(message: impl Into<String>) -> Self {
        Self::Usage(message.into())
    }

    fn runtime(message: impl Into<String>) -> Self {
        Self::Runtime(message.into())
    }
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage(message) | Self::Runtime(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for CliError {}

impl From<io::Error> for CliError {
    fn from(error: io::Error) -> Self {
        Self::runtime(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{
        AuditConfig, CliStorageFormat, EXIT_AUDIT_FAILED, OutputFormat, TargetCatalog,
        run_with_writer,
    };

    #[test]
    fn audit_config_parses_required_inputs_and_repeated_targets() {
        let config = AuditConfig::parse([
            "--source-locale".to_owned(),
            "en".to_owned(),
            "--source".to_owned(),
            "locales/en.po".to_owned(),
            "--target".to_owned(),
            "de=locales/de.po".to_owned(),
            "--target".to_owned(),
            "fr=locales/fr.po".to_owned(),
            "--locale".to_owned(),
            "de".to_owned(),
            "--format".to_owned(),
            "json".to_owned(),
            "--storage".to_owned(),
            "ndjson".to_owned(),
        ])
        .expect("audit config should parse");

        assert_eq!(config.source_locale, "en");
        assert_eq!(config.targets.len(), 2);
        assert_eq!(
            config.targets[0],
            TargetCatalog {
                locale: "de".to_owned(),
                path: "locales/de.po".into(),
            }
        );
        assert_eq!(config.locale_filters, vec!["de"]);
        assert_eq!(config.format, OutputFormat::Json);
        assert_eq!(config.storage_format, CliStorageFormat::Ndjson);
    }

    #[test]
    fn audit_config_rejects_target_without_locale() {
        let error = AuditConfig::parse([
            "--source-locale".to_owned(),
            "en".to_owned(),
            "--source".to_owned(),
            "locales/en.po".to_owned(),
            "--target".to_owned(),
            "locales/de.po".to_owned(),
        ])
        .expect_err("target without locale should fail");

        assert!(error.to_string().contains("--target expects"));
    }

    #[test]
    fn audit_help_exits_successfully() {
        let mut stdout = Vec::new();
        let exit_code = run_with_writer(["audit".to_owned(), "--help".to_owned()], &mut stdout)
            .expect("help should succeed");

        assert_eq!(exit_code, std::process::ExitCode::SUCCESS);
        let output = String::from_utf8(stdout).expect("help output should be UTF-8");
        assert!(output.contains("ferrocat audit"));
    }

    #[test]
    fn audit_command_returns_failure_exit_code_for_error_diagnostics() {
        let tempdir = tempfile_dir();
        let source_path = tempdir.path().join("en.po");
        let target_path = tempdir.path().join("de.po");
        fs::write(&source_path, "msgid \"Checkout\"\nmsgstr \"Checkout\"\n")
            .expect("write source fixture");
        fs::write(&target_path, "").expect("write target fixture");

        let mut stdout = Vec::new();
        let exit_code = run_with_writer(
            [
                "audit".to_owned(),
                "--source-locale".to_owned(),
                "en".to_owned(),
                "--source".to_owned(),
                source_path.display().to_string(),
                "--target".to_owned(),
                format!("de={}", target_path.display()),
            ],
            &mut stdout,
        )
        .expect("audit command should run");

        assert_eq!(exit_code, std::process::ExitCode::from(EXIT_AUDIT_FAILED));
        let output = String::from_utf8(stdout).expect("text output should be UTF-8");
        assert!(output.contains("catalog.missing_translation"));
    }

    #[test]
    fn audit_command_writes_json_report() {
        let tempdir = tempfile_dir();
        let source_path = tempdir.path().join("en.po");
        let target_path = tempdir.path().join("de.po");
        fs::write(&source_path, "msgid \"Checkout\"\nmsgstr \"Checkout\"\n")
            .expect("write source fixture");
        fs::write(&target_path, "msgid \"Checkout\"\nmsgstr \"Zur Kasse\"\n")
            .expect("write target fixture");

        let mut stdout = Vec::new();
        let exit_code = run_with_writer(
            [
                "audit".to_owned(),
                "--source-locale".to_owned(),
                "en".to_owned(),
                "--source".to_owned(),
                source_path.display().to_string(),
                "--target".to_owned(),
                format!("de={}", target_path.display()),
                "--format".to_owned(),
                "json".to_owned(),
            ],
            &mut stdout,
        )
        .expect("audit command should run");

        assert_eq!(exit_code, std::process::ExitCode::SUCCESS);
        let json: serde_json::Value =
            serde_json::from_slice(&stdout).expect("JSON output should parse");
        assert_eq!(json["summary"]["errors"], 0);
        assert_eq!(json["summary"]["target_locales"], 1);
    }

    fn tempfile_dir() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir should be created")
    }
}
