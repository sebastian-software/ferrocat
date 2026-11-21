use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Arg, ArgAction, ArgMatches, Command, error::ErrorKind};

use ferrocat_po::{
    CatalogAuditOptions, CatalogAuditReport, CatalogAuditSummary, CatalogMode, DiagnosticSeverity,
    NormalizedParsedCatalog, ParseCatalogOptions, audit_catalogs, parse_catalog_for_review,
};

const EXIT_AUDIT_FAILED: u8 = 1;
const EXIT_USAGE_OR_RUNTIME_ERROR: u8 = 2;

fn command() -> Command {
    let scalar = |name: &'static str, help: &'static str| {
        Arg::new(name)
            .long(name)
            .help(help)
            .action(ArgAction::Set)
            .allow_hyphen_values(true)
    };
    Command::new("ferrocat")
        .about("Audit source and target catalogs for release readiness.")
        .subcommand(
            Command::new("audit")
                .about("Audit source and target catalogs for release readiness.")
                .args_override_self(true)
                .arg(
                    scalar(
                        "source-locale",
                        "Locale that represents the source catalog.",
                    )
                    .required(true)
                    .value_name("locale"),
                )
                .arg(
                    scalar("source", "Source catalog path.")
                        .required(true)
                        .value_name("path"),
                )
                .arg(
                    Arg::new("target")
                        .long("target")
                        .help("Target catalog path. Repeat for multiple locales.")
                        .required(true)
                        .action(ArgAction::Append)
                        .allow_hyphen_values(true)
                        .value_name("locale=path"),
                )
                .arg(
                    Arg::new("locale")
                        .long("locale")
                        .help("Target locale filter. Repeat for multiple locales.")
                        .action(ArgAction::Append)
                        .allow_hyphen_values(true)
                        .value_name("locale"),
                )
                .arg(
                    scalar("storage", "Catalog storage format.")
                        .value_parser(["po", "fcl"])
                        .default_value("po")
                        .value_name("po|fcl"),
                )
                .arg(
                    scalar("format", "Output format.")
                        .value_parser(["text", "json"])
                        .default_value("text")
                        .value_name("text|json"),
                )
                .arg(
                    Arg::new("json")
                        .long("json")
                        .help("Output JSON (takes precedence over --format).")
                        .action(ArgAction::SetTrue),
                ),
        )
}

fn main() -> ExitCode {
    let mut stdout = io::stdout().lock();
    let mut stderr = io::stderr().lock();
    run_cli(env::args().skip(1), &mut stdout, &mut stderr)
}

fn run_cli<I, W, E>(args: I, stdout: &mut W, stderr: &mut E) -> ExitCode
where
    I: IntoIterator<Item = String>,
    W: Write,
    E: Write,
{
    match run_with_writer(args, stdout) {
        Ok(exit_code) => exit_code,
        Err(error) => {
            let _ = writeln!(stderr, "{error}");
            if let CliError::Usage(message) = &error
                && !message.contains("Usage:")
            {
                let _ = writeln!(stderr, "\n{}", command().render_usage());
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
    let args: Vec<String> = args.into_iter().collect();
    if args.is_empty() {
        command().write_long_help(stdout)?;
        writeln!(stdout)?;
        return Ok(ExitCode::SUCCESS);
    }
    // Preserve audit help priority even if a preceding option expects a value.
    if args.first().is_some_and(|arg| arg == "audit")
        && args
            .iter()
            .skip(1)
            .any(|arg| matches!(arg.as_str(), "-h" | "--help"))
    {
        let help = command()
            .try_get_matches_from(["ferrocat", "audit", "--help"])
            .expect_err("clap renders audit help");
        write!(stdout, "{help}")?;
        return Ok(ExitCode::SUCCESS);
    }
    let matches =
        match command().try_get_matches_from(std::iter::once("ferrocat".to_owned()).chain(args)) {
            Ok(matches) => matches,
            Err(error) if error.kind() == ErrorKind::DisplayHelp => {
                write!(stdout, "{error}")?;
                return Ok(ExitCode::SUCCESS);
            }
            Err(error) => return Err(CliError::usage(error.to_string())),
        };
    let Some(("audit", matches)) = matches.subcommand() else {
        command().write_long_help(stdout)?;
        writeln!(stdout)?;
        return Ok(ExitCode::SUCCESS);
    };
    let config = AuditConfig::from_matches(matches)?;
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
    Ok(if report.has_errors() {
        ExitCode::from(EXIT_AUDIT_FAILED)
    } else {
        ExitCode::SUCCESS
    })
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
    Fcl,
}

impl CliStorageFormat {
    fn parse(value: &str) -> Result<Self, CliError> {
        match value {
            "po" => Ok(Self::Po),
            "fcl" => Ok(Self::Fcl),
            other => Err(CliError::usage(format!(
                "unsupported storage format `{other}`; expected `po` or `fcl`"
            ))),
        }
    }

    const fn catalog_mode(self) -> CatalogMode {
        match self {
            Self::Po => CatalogMode::IcuPo,
            Self::Fcl => CatalogMode::IcuFcl,
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
    #[cfg(test)]
    fn parse<I: IntoIterator<Item = String>>(args: I) -> Result<Self, CliError> {
        let matches = command()
            .try_get_matches_from(
                ["ferrocat".to_owned(), "audit".to_owned()]
                    .into_iter()
                    .chain(args),
            )
            .map_err(|error| CliError::usage(error.to_string()))?;
        Self::from_matches(
            matches
                .subcommand_matches("audit")
                .expect("audit subcommand"),
        )
    }

    fn from_matches(matches: &ArgMatches) -> Result<Self, CliError> {
        let source_locale = matches
            .get_one::<String>("source-locale")
            .expect("required")
            .clone();
        if source_locale.trim().is_empty() {
            return Err(CliError::usage("--source-locale must not be empty"));
        }
        let locale_filters = matches
            .get_many::<String>("locale")
            .into_iter()
            .flatten()
            .cloned()
            .collect::<Vec<_>>();
        if locale_filters.iter().any(|locale| locale.trim().is_empty()) {
            return Err(CliError::usage("--locale must not be empty"));
        }
        if locale_filters.iter().any(|locale| locale == &source_locale) {
            return Err(CliError::usage(
                "--locale selects target locales and must not equal --source-locale",
            ));
        }
        let targets = matches
            .get_many::<String>("target")
            .expect("required")
            .map(|value| parse_target(value))
            .collect::<Result<Vec<_>, _>>()?;
        let source_path = PathBuf::from(matches.get_one::<String>("source").expect("required"));
        let storage_format =
            CliStorageFormat::parse(matches.get_one::<String>("storage").expect("default"))?;
        let format = if matches.get_flag("json") {
            OutputFormat::Json
        } else {
            OutputFormat::parse(matches.get_one::<String>("format").expect("default"))?
        };
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
        // Validate the entire declared inventory even when only some catalogs are read.
        let mut locales = std::collections::BTreeSet::new();
        locales.insert(self.source_locale.as_str());
        for target in &self.targets {
            if !locales.insert(target.locale.as_str()) {
                return Err(CliError::runtime(format!(
                    "catalog audit failed: audit_catalogs received duplicate catalog locale {:?}",
                    target.locale
                )));
            }
        }
        for target in &self.targets {
            if !self.locale_filters.is_empty() && !self.locale_filters.contains(&target.locale) {
                continue;
            }
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
        let options = CatalogAuditOptions::new(&self.source_locale).with_locales(&locale_filters);

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
    parse_catalog_for_review(
        ParseCatalogOptions::new(&content, source_locale)
            .with_locale(locale)
            .with_mode(storage_format.catalog_mode()),
    )
    .map_err(|error| CliError::runtime(format!("failed to parse {}: {error}", path.display())))
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
        AuditConfig, CliStorageFormat, EXIT_AUDIT_FAILED, EXIT_USAGE_OR_RUNTIME_ERROR,
        OutputFormat, TargetCatalog, run_cli, run_with_writer,
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
            "fcl".to_owned(),
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
        assert_eq!(config.storage_format, CliStorageFormat::Fcl);
    }

    #[test]
    fn audit_rejects_source_locale_as_target_filter() {
        for filters in [vec!["en"], vec!["de", "en"]] {
            let mut args = vec![
                "audit",
                "--source-locale",
                "en",
                "--source",
                "en.po",
                "--target",
                "de=de.po",
            ];
            for locale in filters {
                args.extend(["--locale", locale]);
            }
            let error = run_with_writer(args.into_iter().map(str::to_owned), &mut Vec::new())
                .expect_err("source locale is not a target filter");
            assert!(error.to_string().contains("must not equal --source-locale"));
        }
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

    #[test]
    fn audit_command_reports_fuzzy_translations() {
        let tempdir = tempfile_dir();
        let source_path = tempdir.path().join("en.po");
        let target_path = tempdir.path().join("de.po");
        fs::write(&source_path, "msgid \"Checkout\"\nmsgstr \"Checkout\"\n")
            .expect("write source fixture");
        fs::write(
            &target_path,
            "#, fuzzy\nmsgid \"Checkout\"\nmsgstr \"Zur Kasse\"\n",
        )
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
        assert_eq!(json["summary"]["infos"], 1);
        assert_eq!(json["diagnostics"][0]["code"], "catalog.fuzzy_flag");
    }

    #[test]
    fn cli_returns_usage_exit_code_for_unknown_command() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let exit_code = run_cli(["unknown".to_owned()], &mut stdout, &mut stderr);

        assert_eq!(
            exit_code,
            std::process::ExitCode::from(EXIT_USAGE_OR_RUNTIME_ERROR)
        );
        assert!(stdout.is_empty());
        let error = String::from_utf8(stderr).expect("stderr should be UTF-8");
        assert!(error.contains("unknown"));
        assert!(error.contains("Usage:"));
    }

    #[test]
    fn cli_returns_usage_exit_code_for_missing_flag_value() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let exit_code = run_cli(
            ["audit".to_owned(), "--source-locale".to_owned()],
            &mut stdout,
            &mut stderr,
        );

        assert_eq!(
            exit_code,
            std::process::ExitCode::from(EXIT_USAGE_OR_RUNTIME_ERROR)
        );
        let error = String::from_utf8(stderr).expect("stderr should be UTF-8");
        assert!(error.contains("--source-locale"));
        assert!(error.contains("Usage:"));
    }

    #[test]
    fn cli_returns_runtime_exit_code_for_missing_source_file() {
        let tempdir = tempfile_dir();
        let target_path = tempdir.path().join("de.po");
        fs::write(&target_path, "msgid \"Checkout\"\nmsgstr \"Zur Kasse\"\n")
            .expect("write target fixture");

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let exit_code = run_cli(
            [
                "audit".to_owned(),
                "--source-locale".to_owned(),
                "en".to_owned(),
                "--source".to_owned(),
                tempdir.path().join("missing.po").display().to_string(),
                "--target".to_owned(),
                format!("de={}", target_path.display()),
            ],
            &mut stdout,
            &mut stderr,
        );

        assert_eq!(
            exit_code,
            std::process::ExitCode::from(EXIT_USAGE_OR_RUNTIME_ERROR)
        );
        let error = String::from_utf8(stderr).expect("stderr should be UTF-8");
        assert!(error.contains("failed to read"));
        assert!(!error.contains("Usage:"));
    }

    #[test]
    fn audit_command_filters_requested_target_locale() {
        let tempdir = tempfile_dir();
        let source_path = tempdir.path().join("en.po");
        let de_path = tempdir.path().join("de.po");
        let fr_path = tempdir.path().join("fr.po");
        fs::write(&source_path, "msgid \"Checkout\"\nmsgstr \"Checkout\"\n")
            .expect("write source fixture");
        // The unselected German catalog does not exist.
        fs::write(&fr_path, "msgid \"Checkout\"\nmsgstr \"Paiement\"\n")
            .expect("write French fixture");

        let mut stdout = Vec::new();
        let exit_code = run_with_writer(
            [
                "audit".to_owned(),
                "--source-locale".to_owned(),
                "en".to_owned(),
                "--source".to_owned(),
                source_path.display().to_string(),
                "--target".to_owned(),
                format!("de={}", de_path.display()),
                "--target".to_owned(),
                format!("fr={}", fr_path.display()),
                "--locale".to_owned(),
                "fr".to_owned(),
            ],
            &mut stdout,
        )
        .expect("filtered audit should run");

        assert_eq!(exit_code, std::process::ExitCode::SUCCESS);
        let output = String::from_utf8(stdout).expect("text output should be UTF-8");
        assert!(output.contains("1 target locales"));
        assert!(!output.contains("catalog.missing_translation"));
    }

    #[test]
    fn audit_inventory_preserves_errors_and_filter_semantics() {
        let tempdir = tempfile_dir();
        let source = tempdir.path().join("en.po");
        let french = tempdir.path().join("fr.po");
        let missing = tempdir.path().join("missing.po");
        fs::write(&source, "msgid \"Checkout\"\nmsgstr \"Checkout\"\n")
            .expect("write source fixture");
        fs::write(&french, "msgid \"Checkout\"\nmsgstr \"Paiement\"\n")
            .expect("write target fixture");
        let mut config = AuditConfig {
            source_locale: "en".into(),
            source_path: source,
            targets: vec![
                TargetCatalog {
                    locale: "de".into(),
                    path: missing.clone(),
                },
                TargetCatalog {
                    locale: "fr".into(),
                    path: french,
                },
            ],
            locale_filters: vec!["fr".into(), "fr".into(), "en".into(), "es".into()],
            storage_format: CliStorageFormat::Po,
            format: OutputFormat::Text,
        };
        let report = config.run().expect("unselected target should not be read");
        assert_eq!(report.summary.target_locales, 1);
        assert!(
            report
                .diagnostics
                .iter()
                .any(|d| d.message.contains("requested locale `es`"))
        );

        config.locale_filters.clear();
        assert!(
            config
                .run()
                .expect_err("audit should fail")
                .to_string()
                .contains("failed to read")
        );
        config.locale_filters = vec!["de".into()];
        assert!(
            config
                .run()
                .expect_err("audit should fail")
                .to_string()
                .contains("failed to read")
        );
        config.locale_filters = vec!["fr".into()];
        config.targets.push(TargetCatalog {
            locale: "de".into(),
            path: missing.clone(),
        });
        assert!(
            config
                .run()
                .expect_err("audit should fail")
                .to_string()
                .contains("duplicate catalog locale")
        );
        config.targets.pop();
        config.targets.push(TargetCatalog {
            locale: "en".into(),
            path: missing.clone(),
        });
        assert!(
            config
                .run()
                .expect_err("audit should fail")
                .to_string()
                .contains("duplicate catalog locale")
        );
        config.targets.pop();
        config.source_path = missing;
        assert!(
            config
                .run()
                .expect_err("audit should fail")
                .to_string()
                .contains("failed to read")
        );
    }

    #[test]
    fn audit_command_reads_fcl_catalogs() {
        let tempdir = tempfile_dir();
        let source_path = tempdir.path().join("en.fcl");
        let target_path = tempdir.path().join("de.fcl");
        fs::write(&source_path, fcl_catalog("en", "Checkout")).expect("write source fixture");
        fs::write(&target_path, fcl_catalog("de", "Zur Kasse")).expect("write target fixture");

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
                "--storage".to_owned(),
                "fcl".to_owned(),
                "--format".to_owned(),
                "json".to_owned(),
            ],
            &mut stdout,
        )
        .expect("FCL audit should run");

        assert_eq!(exit_code, std::process::ExitCode::SUCCESS);
        let json: serde_json::Value =
            serde_json::from_slice(&stdout).expect("JSON output should parse");
        assert_eq!(json["summary"]["errors"], 0);
    }

    #[test]
    fn audit_command_reports_fcl_parse_errors() {
        let tempdir = tempfile_dir();
        let source_path = tempdir.path().join("en.fcl");
        let target_path = tempdir.path().join("de.fcl");
        fs::write(&source_path, "not a valid fcl catalog\n")
            .expect("write malformed source fixture");
        fs::write(&target_path, fcl_catalog("de", "Zur Kasse")).expect("write target fixture");

        let mut stdout = Vec::new();
        let error = run_with_writer(
            [
                "audit".to_owned(),
                "--source-locale".to_owned(),
                "en".to_owned(),
                "--source".to_owned(),
                source_path.display().to_string(),
                "--target".to_owned(),
                format!("de={}", target_path.display()),
                "--storage".to_owned(),
                "fcl".to_owned(),
            ],
            &mut stdout,
        )
        .expect_err("malformed FCL should fail");

        assert!(error.to_string().contains("failed to parse"));
    }

    fn tempfile_dir() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir should be created")
    }

    fn fcl_catalog(locale: &str, translation: &str) -> String {
        format!("%FCL1\tsource=en\tlocale={locale}\nCheckout\t\t{translation}\n")
    }
}
