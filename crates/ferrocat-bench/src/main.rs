mod compare;
#[path = "../../../conformance/harness.rs"]
mod conformance_harness;
mod fixtures;

use std::env;
use std::fs;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use conformance_harness::{evaluate_all_cases, summarize_evaluations};
use ferrocat_conformance::{ConformanceCase, Expectation, ExpectedArtifact, load_all_manifests};
use ferrocat_icu::{extract_variables, parse_icu, validate_icu};
use ferrocat_po::{
    CatalogMessage, CatalogMessageExtra, CatalogStorageFormat, ParseCatalogOptions, ParsedCatalog,
    PluralEncoding, SerializeOptions, TranslationShape, UpdateCatalogFileOptions,
    UpdateCatalogOptions, merge_catalog, parse_catalog, parse_po, parse_po_borrowed, stringify_po,
    update_catalog, update_catalog_file,
};
use fixtures::{
    Fixture, IcuFixture, MergeFixture, fixture_by_name, icu_fixture_by_name, merge_fixture_by_name,
};
use serde::Serialize;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "parse".to_owned());

    match command.as_str() {
        "verify-benchmark-env" => compare::run_verify_benchmark_env(),
        "compare" => {
            let profile_name = args
                .next()
                .ok_or_else(|| "compare requires a profile name".to_owned())?;
            compare::run_compare_command(&profile_name, args)
        }
        "parse" => {
            let fixture_name = args.next().unwrap_or_else(|| "realistic".to_owned());
            let config = parse_bench_config(args, &fixture_name)?;
            let fixture = load_fixture(&fixture_name)?;
            bench_parse(&fixture, config)
        }
        "parse-borrowed" => {
            let fixture_name = args.next().unwrap_or_else(|| "realistic".to_owned());
            let config = parse_bench_config(args, &fixture_name)?;
            let fixture = load_fixture(&fixture_name)?;
            bench_parse_borrowed(&fixture, config)
        }
        "parse-ndjson" => {
            let fixture_name = args.next().unwrap_or_else(|| "realistic".to_owned());
            let config = parse_bench_config(args, &fixture_name)?;
            let fixture = load_fixture(&fixture_name)?;
            bench_parse_ndjson(&fixture, config)
        }
        "parse-icu" => {
            let fixture_name = args.next().unwrap_or_else(|| "realistic".to_owned());
            let config = parse_bench_config(args, &fixture_name)?;
            let fixture = load_icu_fixture(&fixture_name)?;
            bench_parse_icu(&fixture, config)
        }
        "validate-icu" => {
            let fixture_name = args.next().unwrap_or_else(|| "realistic".to_owned());
            let config = parse_bench_config(args, &fixture_name)?;
            let fixture = load_icu_fixture(&fixture_name)?;
            bench_validate_icu(&fixture, config)
        }
        "extract-icu-variables" => {
            let fixture_name = args.next().unwrap_or_else(|| "realistic".to_owned());
            let config = parse_bench_config(args, &fixture_name)?;
            let fixture = load_icu_fixture(&fixture_name)?;
            bench_extract_icu_variables(&fixture, config)
        }
        "stringify" => {
            let fixture_name = args.next().unwrap_or_else(|| "realistic".to_owned());
            let config = parse_bench_config(args, &fixture_name)?;
            let fixture = load_fixture(&fixture_name)?;
            bench_stringify(&fixture, config)
        }
        "stringify-ndjson" => {
            let fixture_name = args.next().unwrap_or_else(|| "realistic".to_owned());
            let config = parse_bench_config(args, &fixture_name)?;
            let fixture = load_fixture(&fixture_name)?;
            bench_stringify_ndjson(&fixture, config)
        }
        "merge" => {
            let fixture_name = args.next().unwrap_or_else(|| "realistic".to_owned());
            let config = parse_bench_config(args, &fixture_name)?;
            let fixture = load_merge_fixture(&fixture_name)?;
            bench_merge(&fixture, config)
        }
        "update-catalog" => {
            let fixture_name = args.next().unwrap_or_else(|| "realistic".to_owned());
            let config = parse_bench_config(args, &fixture_name)?;
            let fixture = load_merge_fixture(&fixture_name)?;
            bench_update_catalog(&fixture, config)
        }
        "update-catalog-file" => {
            let fixture_name = args.next().unwrap_or_else(|| "realistic".to_owned());
            let config = parse_bench_config(args, &fixture_name)?;
            let fixture = load_merge_fixture(&fixture_name)?;
            bench_update_catalog_file(&fixture, config)
        }
        "update-catalog-file-ndjson" => {
            let fixture_name = args.next().unwrap_or_else(|| "realistic".to_owned());
            let config = parse_bench_config(args, &fixture_name)?;
            let fixture = load_merge_fixture(&fixture_name)?;
            bench_update_catalog_file_ndjson(&fixture, config)
        }
        "describe" => {
            let fixture_name = args.next().unwrap_or_else(|| "realistic".to_owned());
            let fixture = load_fixture(&fixture_name)?;
            describe(&fixture);
            Ok(())
        }
        "conformance-report" => {
            conformance_report();
            Ok(())
        }
        other => Err(format!(
            "unknown command: {other} (use verify-benchmark-env, compare, parse, parse-borrowed, parse-ndjson, parse-icu, validate-icu, extract-icu-variables, stringify, stringify-ndjson, merge, update-catalog, update-catalog-file, update-catalog-file-ndjson, describe, or conformance-report)"
        )),
    }
}

#[derive(Debug, Clone, Copy)]
struct BenchConfig {
    iterations: usize,
    runs: usize,
    warmup_runs: usize,
}

#[derive(Debug, Clone, Copy)]
struct BenchSample {
    elapsed: Duration,
    iter_per_sec: f64,
    mib_per_sec: f64,
}

fn parse_bench_config(
    args: impl Iterator<Item = String>,
    fixture_name: &str,
) -> Result<BenchConfig, String> {
    let mut iterations = None;
    let mut runs = 1usize;
    let mut warmup_runs = 0usize;
    let mut args = args;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--runs" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--runs requires a value".to_owned())?;
                runs = parse_positive_usize("--runs", &value)?;
            }
            "--warmup" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--warmup requires a value".to_owned())?;
                warmup_runs = value
                    .parse::<usize>()
                    .map_err(|_| format!("invalid --warmup value: {value}"))?;
            }
            value if value.starts_with("--") => {
                return Err(format!("unknown flag: {value}"));
            }
            value => {
                if iterations.is_some() {
                    return Err(format!("unexpected extra argument: {value}"));
                }
                iterations = Some(parse_positive_usize("iterations", value)?);
            }
        }
    }

    Ok(BenchConfig {
        iterations: iterations.unwrap_or_else(|| default_iterations(fixture_name)),
        runs,
        warmup_runs,
    })
}

fn parse_positive_usize(label: &str, value: &str) -> Result<usize, String> {
    let parsed = value
        .parse::<usize>()
        .map_err(|_| format!("invalid {label} value: {value}"))?;
    if parsed == 0 {
        return Err(format!("{label} must be greater than zero"));
    }
    Ok(parsed)
}

fn load_fixture(fixture_name: &str) -> Result<Fixture, String> {
    fixture_by_name(fixture_name).ok_or_else(|| {
        format!(
            "unknown fixture: {fixture_name} (use tiny, realistic, stress, mixed-1000, mixed-10000, or gettext-<ui|commerce|saas|content>-<de|fr|pl|ar>-<count>)"
        )
    })
}

fn load_icu_fixture(fixture_name: &str) -> Result<IcuFixture, String> {
    icu_fixture_by_name(fixture_name).ok_or_else(|| {
        format!(
            "unknown icu fixture: {fixture_name} (use icu-literal-1000, icu-literal-10000, icu-args-1000, icu-args-10000, icu-formatters-1000, icu-formatters-10000, icu-plural-1000, icu-plural-10000, icu-select-1000, icu-select-10000, icu-nested-1000, icu-nested-10000, icu-tags-1000, or icu-tags-10000)"
        )
    })
}

fn load_merge_fixture(fixture_name: &str) -> Result<MergeFixture, String> {
    merge_fixture_by_name(fixture_name).ok_or_else(|| {
        format!(
            "unknown merge fixture: {fixture_name} (use mixed-1000, mixed-10000, gettext-<ui|commerce|saas|content>-<de|fr|pl|ar>-<count>, catalog-icu-light, catalog-icu-heavy, catalog-icu-projectable, or catalog-icu-unsupported)"
        )
    })
}

fn default_iterations(fixture_name: &str) -> usize {
    match fixture_name {
        "tiny" => 20_000,
        "mixed-10000" => 100,
        "catalog-icu-heavy" => 25,
        "catalog-icu-projectable" | "catalog-icu-unsupported" => 50,
        "stress" => 1_000,
        name if name.starts_with("gettext-") && name.ends_with("-10000") => 100,
        name if name.starts_with("gettext-") => 400,
        name if name.starts_with("icu-") && name.ends_with("-10000") => 50,
        name if name.starts_with("icu-") => 200,
        _ => 5_000,
    }
}

fn bench_parse(fixture: &Fixture, config: BenchConfig) -> Result<(), String> {
    let mut items_per_iteration = 0usize;
    let samples = run_bench(config, || {
        let start = Instant::now();
        for _ in 0..config.iterations {
            let file = parse_po(fixture.content()).map_err(|error| error.to_string())?;
            items_per_iteration = file.items.len();
            std::hint::black_box(file);
        }
        Ok(BenchSample::new(
            start.elapsed(),
            config.iterations,
            fixture.content().len(),
        ))
    })?;
    report(
        "parse",
        fixture,
        fixture.content().len(),
        items_per_iteration,
        config,
        &samples,
    );
    Ok(())
}

fn bench_parse_borrowed(fixture: &Fixture, config: BenchConfig) -> Result<(), String> {
    let mut items_per_iteration = 0usize;
    let samples = run_bench(config, || {
        let start = Instant::now();
        for _ in 0..config.iterations {
            let file = parse_po_borrowed(fixture.content()).map_err(|error| error.to_string())?;
            items_per_iteration = file.items.len();
            std::hint::black_box(file);
        }
        Ok(BenchSample::new(
            start.elapsed(),
            config.iterations,
            fixture.content().len(),
        ))
    })?;
    report(
        "parse-borrowed",
        fixture,
        fixture.content().len(),
        items_per_iteration,
        config,
        &samples,
    );
    Ok(())
}

fn bench_parse_ndjson(fixture: &Fixture, config: BenchConfig) -> Result<(), String> {
    let (content, locale, items_per_iteration) = fixture_ndjson_content(fixture)?;
    let mut parsed_items = 0usize;
    let samples = run_bench(config, || {
        let start = Instant::now();
        for _ in 0..config.iterations {
            let parsed = parse_catalog(ParseCatalogOptions {
                content: &content,
                locale,
                source_locale: "en",
                storage_format: CatalogStorageFormat::Ndjson,
                plural_encoding: PluralEncoding::Icu,
                strict: false,
            })
            .map_err(|error| error.to_string())?;
            parsed_items = parsed.messages.len();
            std::hint::black_box(parsed);
        }
        Ok(BenchSample::new(
            start.elapsed(),
            config.iterations,
            content.len(),
        ))
    })?;
    report(
        "parse-ndjson",
        fixture,
        content.len(),
        parsed_items.max(items_per_iteration),
        config,
        &samples,
    );
    Ok(())
}

fn bench_parse_icu(fixture: &IcuFixture, config: BenchConfig) -> Result<(), String> {
    let samples = run_bench(config, || {
        let start = Instant::now();
        for _ in 0..config.iterations {
            for message in fixture.messages() {
                let parsed = parse_icu(message).map_err(|error| error.to_string())?;
                std::hint::black_box(parsed);
            }
        }
        Ok(BenchSample::new(
            start.elapsed(),
            config.iterations,
            fixture.total_bytes(),
        ))
    })?;
    report_icu("parse-icu", fixture, config, &samples);
    Ok(())
}

fn bench_validate_icu(fixture: &IcuFixture, config: BenchConfig) -> Result<(), String> {
    let samples = run_bench(config, || {
        let start = Instant::now();
        for _ in 0..config.iterations {
            for message in fixture.messages() {
                validate_icu(message).map_err(|error| error.to_string())?;
            }
        }
        Ok(BenchSample::new(
            start.elapsed(),
            config.iterations,
            fixture.total_bytes(),
        ))
    })?;
    report_icu("validate-icu", fixture, config, &samples);
    Ok(())
}

fn bench_extract_icu_variables(fixture: &IcuFixture, config: BenchConfig) -> Result<(), String> {
    let parsed = fixture
        .messages()
        .iter()
        .map(|message| parse_icu(message).map_err(|error| error.to_string()))
        .collect::<Result<Vec<_>, _>>()?;

    let samples = run_bench(config, || {
        let start = Instant::now();
        for _ in 0..config.iterations {
            for message in &parsed {
                let variables = extract_variables(message);
                std::hint::black_box(variables);
            }
        }
        Ok(BenchSample::new(
            start.elapsed(),
            config.iterations,
            fixture.total_bytes(),
        ))
    })?;
    report_icu("extract-icu-variables", fixture, config, &samples);
    Ok(())
}

fn bench_stringify(fixture: &Fixture, config: BenchConfig) -> Result<(), String> {
    let file = parse_po(fixture.content()).map_err(|error| error.to_string())?;
    let options = SerializeOptions::default();

    let mut bytes_per_iteration = 0usize;
    let samples = run_bench(config, || {
        let start = Instant::now();
        let mut bytes = 0usize;
        for _ in 0..config.iterations {
            let rendered = stringify_po(&file, &options);
            bytes += rendered.len();
            std::hint::black_box(rendered);
        }
        bytes_per_iteration = bytes / config.iterations;
        Ok(BenchSample::new(
            start.elapsed(),
            config.iterations,
            bytes_per_iteration,
        ))
    })?;
    report(
        "stringify",
        fixture,
        bytes_per_iteration,
        file.items.len(),
        config,
        &samples,
    );
    Ok(())
}

fn bench_stringify_ndjson(fixture: &Fixture, config: BenchConfig) -> Result<(), String> {
    let parsed = fixture_parsed_catalog(fixture)?;
    let mut bytes_per_iteration = 0usize;
    let samples = run_bench(config, || {
        let start = Instant::now();
        let mut bytes = 0usize;
        for _ in 0..config.iterations {
            let rendered = render_ndjson_catalog(&parsed);
            bytes += rendered.len();
            std::hint::black_box(rendered);
        }
        bytes_per_iteration = bytes / config.iterations;
        Ok(BenchSample::new(
            start.elapsed(),
            config.iterations,
            bytes_per_iteration,
        ))
    })?;
    report(
        "stringify-ndjson",
        fixture,
        bytes_per_iteration,
        parsed.messages.len(),
        config,
        &samples,
    );
    Ok(())
}

fn bench_merge(fixture: &MergeFixture, config: BenchConfig) -> Result<(), String> {
    let mut bytes_per_iteration = 0usize;
    let samples = run_bench(config, || {
        let start = Instant::now();
        let mut bytes = 0usize;
        for _ in 0..config.iterations {
            let rendered = merge_catalog(fixture.existing_po(), fixture.extracted_messages())
                .map_err(|error| error.to_string())?;
            bytes += rendered.len();
            std::hint::black_box(rendered);
        }
        bytes_per_iteration = bytes / config.iterations;
        Ok(BenchSample::new(
            start.elapsed(),
            config.iterations,
            bytes_per_iteration,
        ))
    })?;
    report_merge("merge", fixture, bytes_per_iteration, config, &samples);
    Ok(())
}

fn bench_update_catalog(fixture: &MergeFixture, config: BenchConfig) -> Result<(), String> {
    let mut bytes_per_iteration = 0usize;
    let samples = run_bench(config, || {
        let start = Instant::now();
        let mut bytes = 0usize;
        for _ in 0..config.iterations {
            let rendered = update_catalog(UpdateCatalogOptions {
                locale: Some("de"),
                source_locale: "en",
                input: fixture.api_extracted_messages().to_vec().into(),
                existing: Some(fixture.existing_po()),
                plural_encoding: PluralEncoding::Icu,
                ..UpdateCatalogOptions::default()
            })
            .map_err(|error| error.to_string())?;
            bytes += rendered.content.len();
            std::hint::black_box(rendered);
        }
        bytes_per_iteration = bytes / config.iterations;
        Ok(BenchSample::new(
            start.elapsed(),
            config.iterations,
            bytes_per_iteration,
        ))
    })?;
    report_merge(
        "update-catalog",
        fixture,
        bytes_per_iteration,
        config,
        &samples,
    );
    Ok(())
}

fn bench_update_catalog_file(fixture: &MergeFixture, config: BenchConfig) -> Result<(), String> {
    let mut bytes_per_iteration = 0usize;
    let temp_root = std::env::temp_dir().join(format!(
        "ferrocat-bench-update-catalog-file-{}",
        std::process::id()
    ));
    fs::create_dir_all(&temp_root).map_err(|error| error.to_string())?;
    let path = temp_root.join("messages.po");

    let samples = run_bench(config, || {
        let start = Instant::now();
        let mut bytes = 0usize;
        for _ in 0..config.iterations {
            fs::write(&path, fixture.existing_po()).map_err(|error| error.to_string())?;
            let rendered = update_catalog_file(UpdateCatalogFileOptions {
                target_path: &path,
                locale: Some("de"),
                source_locale: "en",
                input: fixture.api_extracted_messages().to_vec().into(),
                plural_encoding: PluralEncoding::Icu,
                ..UpdateCatalogFileOptions::default()
            })
            .map_err(|error| error.to_string())?;
            bytes += rendered.content.len();
            std::hint::black_box(rendered);
        }
        bytes_per_iteration = bytes / config.iterations;
        Ok(BenchSample::new(
            start.elapsed(),
            config.iterations,
            bytes_per_iteration,
        ))
    })?;
    let _ = fs::remove_file(&path);
    let _ = fs::remove_dir(&temp_root);

    report_merge(
        "update-catalog-file",
        fixture,
        bytes_per_iteration,
        config,
        &samples,
    );
    Ok(())
}

fn bench_update_catalog_file_ndjson(
    fixture: &MergeFixture,
    config: BenchConfig,
) -> Result<(), String> {
    let mut bytes_per_iteration = 0usize;
    let temp_root = std::env::temp_dir().join(format!(
        "ferrocat-bench-update-catalog-file-ndjson-{}",
        std::process::id()
    ));
    fs::create_dir_all(&temp_root).map_err(|error| error.to_string())?;
    let path = temp_root.join("messages.fcat.ndjson");
    let existing_ndjson = merge_fixture_existing_ndjson(fixture)?;

    let samples = run_bench(config, || {
        let start = Instant::now();
        let mut bytes = 0usize;
        for _ in 0..config.iterations {
            fs::write(&path, &existing_ndjson).map_err(|error| error.to_string())?;
            let rendered = update_catalog_file(UpdateCatalogFileOptions {
                target_path: &path,
                locale: Some("de"),
                source_locale: "en",
                input: fixture.api_extracted_messages().to_vec().into(),
                storage_format: CatalogStorageFormat::Ndjson,
                plural_encoding: PluralEncoding::Icu,
                ..UpdateCatalogFileOptions::default()
            })
            .map_err(|error| error.to_string())?;
            bytes += rendered.content.len();
            std::hint::black_box(rendered);
        }
        bytes_per_iteration = bytes / config.iterations;
        Ok(BenchSample::new(
            start.elapsed(),
            config.iterations,
            bytes_per_iteration,
        ))
    })?;
    let _ = fs::remove_file(&path);
    let _ = fs::remove_dir(&temp_root);

    report_merge(
        "update-catalog-file-ndjson",
        fixture,
        bytes_per_iteration,
        config,
        &samples,
    );
    Ok(())
}

fn run_bench(
    config: BenchConfig,
    mut run_once: impl FnMut() -> Result<BenchSample, String>,
) -> Result<Vec<BenchSample>, String> {
    for _ in 0..config.warmup_runs {
        std::hint::black_box(run_once()?);
    }

    let mut samples = Vec::with_capacity(config.runs);
    for _ in 0..config.runs {
        samples.push(run_once()?);
    }
    Ok(samples)
}

fn describe(fixture: &Fixture) {
    println!("fixture: {}", fixture.name());
    println!("kind: {}", fixture.kind());
    println!("bytes: {}", fixture.content().len());
    println!("items: {}", fixture.stats().entries);
    println!("plural-items: {}", fixture.stats().plural_entries);
    println!(
        "translator-comments: {}",
        fixture.stats().translator_comments
    );
    println!("extracted-comments: {}", fixture.stats().extracted_comments);
    println!("references: {}", fixture.stats().references);
    println!("contexts: {}", fixture.stats().contexts);
    println!("metadata-comments: {}", fixture.stats().metadata_comments);
    println!("obsolete-items: {}", fixture.stats().obsolete_entries);
    println!("multiline-items: {}", fixture.stats().multiline_entries);
    println!("escaped-items: {}", fixture.stats().escaped_entries);
}

fn conformance_report() {
    let evaluations = match evaluate_all_cases() {
        Ok(evaluations) => evaluations,
        Err(error) => {
            eprintln!("failed to evaluate conformance cases: {error}");
            return;
        }
    };
    let assertion_counts = match load_assertion_counts() {
        Ok(counts) => counts,
        Err(error) => {
            eprintln!("failed to load conformance metadata: {error}");
            return;
        }
    };

    let summary = summarize_evaluations(&evaluations);
    let total_assertions = evaluations
        .iter()
        .map(|evaluation| *assertion_counts.get(&evaluation.case_id).unwrap_or(&1))
        .sum::<usize>();
    println!("command: conformance-report");
    println!("total-cases: {}", summary.total);
    println!("total-assertions: {total_assertions}");
    println!("expected-pass: {}", summary.pass);
    println!("expected-reject: {}", summary.reject);
    println!("known-gap: {}", summary.known_gap);
    println!("failed-cases: {}", summary.failures.len());

    let mut by_suite = std::collections::BTreeMap::<String, Vec<_>>::new();
    for evaluation in &evaluations {
        by_suite
            .entry(evaluation.suite.clone())
            .or_default()
            .push(evaluation);
    }

    for (suite, entries) in by_suite {
        let suite_assertions = entries
            .iter()
            .map(|entry| *assertion_counts.get(&entry.case_id).unwrap_or(&1))
            .sum::<usize>();
        println!();
        println!("suite: {suite}");
        println!("cases: {}", entries.len());
        println!("assertions: {suite_assertions}");

        let mut by_capability =
            std::collections::BTreeMap::<String, (usize, usize, usize, usize)>::new();
        for entry in &entries {
            let counts = by_capability
                .entry(entry.capability.clone())
                .or_insert((0, 0, 0, 0));
            match entry.expectation {
                Expectation::Pass => counts.0 += 1,
                Expectation::Reject => counts.1 += 1,
                Expectation::KnownGap => counts.2 += 1,
            }
            counts.3 += *assertion_counts.get(&entry.case_id).unwrap_or(&1);
        }

        for (capability, (pass, reject, known_gap, assertions)) in by_capability {
            println!(
                "capability: {capability} pass={pass} reject={reject} known_gap={known_gap} assertions={assertions}"
            );
        }

        for failure in entries
            .iter()
            .filter(|entry| entry.status == conformance_harness::EvaluationStatus::Failed)
        {
            println!("failure: {} {}", failure.case_id, failure.detail);
        }
    }
}

fn load_assertion_counts() -> Result<std::collections::BTreeMap<String, usize>, String> {
    let manifests = load_all_manifests().map_err(|error| error.to_string())?;
    let mut counts = std::collections::BTreeMap::new();
    for manifest in manifests {
        for case in manifest.cases {
            counts.insert(case.id.clone(), count_case_assertions(&case));
        }
    }
    Ok(counts)
}

fn count_case_assertions(case: &ConformanceCase) -> usize {
    match case.runner.as_str() {
        "po_parse" => match case.expected_artifact() {
            Ok(ExpectedArtifact::PoParse(expected)) => {
                let mut count = 0usize;
                count += usize::from(expected.item_count.is_some());
                count += usize::from(expected.header_count.is_some());
                count += expected.headers.len();
                count += expected.items.len() * 9;
                count.max(1)
            }
            Ok(_) | Err(_) => 1,
        },
        "po_plural_header" => match case.expected_artifact() {
            Ok(ExpectedArtifact::PoPluralHeader(expected)) => {
                let count = usize::from(expected.raw_value.is_some())
                    + usize::from(expected.nplurals.is_some())
                    + usize::from(expected.plural_expression.is_some())
                    + usize::from(expected.first_item_msgstr_len.is_some())
                    + usize::from(case.locale.is_some());
                count.max(1)
            }
            Ok(_) | Err(_) => 1,
        },
        "icu_parse" => match case.expected_artifact() {
            Ok(ExpectedArtifact::IcuParse(expected)) => {
                let count = usize::from(!expected.node_kinds.is_empty())
                    + usize::from(expected.top_level_count.is_some())
                    + usize::from(expected.first_literal.is_some())
                    + usize::from(expected.first_argument_name.is_some())
                    + usize::from(expected.first_plural_kind.is_some())
                    + usize::from(expected.first_plural_offset.is_some())
                    + usize::from(expected.first_plural_option_count.is_some())
                    + usize::from(expected.second_plural_kind.is_some())
                    + usize::from(expected.second_plural_option_count.is_some());
                count.max(1)
            }
            Ok(_) | Err(_) => 1,
        },
        "icu_reject" => match case.expected_artifact() {
            Ok(ExpectedArtifact::IcuReject(expected)) => {
                1 + usize::from(expected.line.is_some())
                    + usize::from(expected.min_column.is_some())
            }
            Ok(_) | Err(_) => 1,
        },
        _ => 1,
    }
}

fn report_merge(
    command: &str,
    fixture: &MergeFixture,
    bytes_per_iteration: usize,
    config: BenchConfig,
    samples: &[BenchSample],
) {
    let summary = summarize(samples);

    println!("command: {command}");
    println!("fixture: {}", fixture.name());
    println!("kind: {}", fixture.kind());
    println!("iterations/run: {}", config.iterations);
    println!("measured-runs: {}", config.runs);
    println!("warmup-runs: {}", config.warmup_runs);
    println!("existing-items: {}", fixture.existing_entries());
    println!("extracted-items: {}", fixture.extracted_entries());
    println!("bytes/iteration: {bytes_per_iteration}");
    println!(
        "median-elapsed: {:.3}s",
        summary.median.elapsed.as_secs_f64()
    );
    println!("median-iter/s: {:.1}", summary.median.iter_per_sec);
    println!("median-MiB/s: {:.2}", summary.median.mib_per_sec);
    println!(
        "iter/s-range: {:.1}..{:.1}",
        summary.min_iter_per_sec, summary.max_iter_per_sec
    );
}

fn report_icu(command: &str, fixture: &IcuFixture, config: BenchConfig, samples: &[BenchSample]) {
    let summary = summarize(samples);

    println!("command: {command}");
    println!("fixture: {}", fixture.name());
    println!("kind: {}", fixture.kind());
    println!("iterations/run: {}", config.iterations);
    println!("measured-runs: {}", config.runs);
    println!("warmup-runs: {}", config.warmup_runs);
    println!("messages/iteration: {}", fixture.entries());
    println!("bytes/iteration: {}", fixture.total_bytes());
    println!(
        "median-elapsed: {:.3}s",
        summary.median.elapsed.as_secs_f64()
    );
    println!("median-iter/s: {:.1}", summary.median.iter_per_sec);
    println!("median-MiB/s: {:.2}", summary.median.mib_per_sec);
    println!(
        "iter/s-range: {:.1}..{:.1}",
        summary.min_iter_per_sec, summary.max_iter_per_sec
    );
}

fn report(
    command: &str,
    fixture: &Fixture,
    bytes_per_iteration: usize,
    items_per_iteration: usize,
    config: BenchConfig,
    samples: &[BenchSample],
) {
    let summary = summarize(samples);

    println!("command: {command}");
    println!("fixture: {}", fixture.name());
    println!("kind: {}", fixture.kind());
    println!("iterations/run: {}", config.iterations);
    println!("measured-runs: {}", config.runs);
    println!("warmup-runs: {}", config.warmup_runs);
    println!("items/iteration: {items_per_iteration}");
    println!("bytes/iteration: {bytes_per_iteration}");
    println!(
        "median-elapsed: {:.3}s",
        summary.median.elapsed.as_secs_f64()
    );
    println!("median-iter/s: {:.1}", summary.median.iter_per_sec);
    println!("median-MiB/s: {:.2}", summary.median.mib_per_sec);
    println!(
        "iter/s-range: {:.1}..{:.1}",
        summary.min_iter_per_sec, summary.max_iter_per_sec
    );
}

#[derive(Debug, Serialize)]
struct BenchNdjsonRecord<'a> {
    id: String,
    str: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    ctx: Option<&'a str>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    comments: Vec<&'a str>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    origin: Vec<BenchNdjsonOrigin<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    extra: Option<BenchNdjsonExtra<'a>>,
    #[serde(skip_serializing_if = "is_false")]
    obsolete: bool,
}

#[derive(Debug, Serialize)]
struct BenchNdjsonOrigin<'a> {
    file: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<u32>,
}

#[derive(Debug, Serialize)]
struct BenchNdjsonExtra<'a> {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    translator_comments: Vec<&'a str>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    flags: Vec<&'a str>,
}

fn fixture_ndjson_content(
    fixture: &Fixture,
) -> Result<(String, Option<&'static str>, usize), String> {
    let parsed = fixture_parsed_catalog(fixture)?;
    let rendered = render_ndjson_catalog(&parsed);
    Ok((
        rendered,
        inferred_fixture_locale(fixture.name()),
        parsed.messages.len(),
    ))
}

fn merge_fixture_existing_ndjson(fixture: &MergeFixture) -> Result<String, String> {
    let locale = inferred_fixture_locale(fixture.name());
    let parsed = parse_catalog(ParseCatalogOptions {
        content: fixture.existing_po(),
        locale,
        source_locale: "en",
        storage_format: CatalogStorageFormat::Po,
        plural_encoding: PluralEncoding::Icu,
        strict: false,
    })
    .map_err(|error| error.to_string())?;
    Ok(render_ndjson_catalog(&parsed))
}

fn fixture_parsed_catalog(fixture: &Fixture) -> Result<ParsedCatalog, String> {
    parse_catalog(ParseCatalogOptions {
        content: fixture.content(),
        locale: inferred_fixture_locale(fixture.name()),
        source_locale: "en",
        storage_format: CatalogStorageFormat::Po,
        plural_encoding: PluralEncoding::Icu,
        strict: false,
    })
    .map_err(|error| error.to_string())
}

fn render_ndjson_catalog(parsed: &ParsedCatalog) -> String {
    let mut rendered = String::from("---\nformat: ferrocat.ndjson.v1\n");
    if let Some(locale) = &parsed.locale {
        rendered.push_str("locale: ");
        rendered.push_str(locale);
        rendered.push('\n');
    }
    rendered.push_str("source_locale: en\n---\n");

    for message in &parsed.messages {
        let record = BenchNdjsonRecord {
            id: render_ndjson_id(message),
            str: render_ndjson_translation(message),
            ctx: message.msgctxt.as_deref(),
            comments: message.comments.iter().map(String::as_str).collect(),
            origin: message
                .origin
                .iter()
                .map(|origin| BenchNdjsonOrigin {
                    file: &origin.file,
                    line: origin.line,
                })
                .collect(),
            extra: render_ndjson_extra(message.extra.as_ref()),
            obsolete: message.obsolete,
        };
        rendered.push_str(
            &serde_json::to_string(&record).expect("benchmark ndjson record must serialize"),
        );
        rendered.push('\n');
    }

    rendered
}

fn render_ndjson_id(message: &CatalogMessage) -> String {
    match &message.translation {
        TranslationShape::Singular { .. } => message.msgid.clone(),
        TranslationShape::Plural {
            source, variable, ..
        } => synthesize_icu_plural(variable, source.one.as_deref(), &source.other),
    }
}

fn render_ndjson_translation(message: &CatalogMessage) -> String {
    match &message.translation {
        TranslationShape::Singular { value } => value.clone(),
        TranslationShape::Plural {
            translation,
            variable,
            ..
        } => synthesize_icu_plural_map(variable, translation),
    }
}

fn render_ndjson_extra(extra: Option<&CatalogMessageExtra>) -> Option<BenchNdjsonExtra<'_>> {
    let extra = extra?;
    if extra.translator_comments.is_empty() && extra.flags.is_empty() {
        None
    } else {
        Some(BenchNdjsonExtra {
            translator_comments: extra
                .translator_comments
                .iter()
                .map(String::as_str)
                .collect(),
            flags: extra.flags.iter().map(String::as_str).collect(),
        })
    }
}

fn synthesize_icu_plural_map(
    variable: &str,
    forms: &std::collections::BTreeMap<String, String>,
) -> String {
    let mut rendered = format!("{{{variable}, plural");
    for (category, value) in forms {
        rendered.push(' ');
        rendered.push_str(category);
        rendered.push_str(" {");
        rendered.push_str(value);
        rendered.push('}');
    }
    rendered.push('}');
    rendered
}

fn synthesize_icu_plural(variable: &str, one: Option<&str>, other: &str) -> String {
    let mut rendered = format!("{{{variable}, plural");
    if let Some(one) = one {
        rendered.push_str(" one {");
        rendered.push_str(one);
        rendered.push('}');
    }
    rendered.push_str(" other {");
    rendered.push_str(other);
    rendered.push_str("}}");
    rendered
}

const fn is_false(value: &bool) -> bool {
    !*value
}

fn inferred_fixture_locale(name: &str) -> Option<&'static str> {
    let parts = name.split('-').collect::<Vec<_>>();
    if parts.len() >= 4 && parts.first() == Some(&"gettext") {
        match parts[2] {
            "de" => Some("de"),
            "fr" => Some("fr"),
            "pl" => Some("pl"),
            "ar" => Some("ar"),
            _ => None,
        }
    } else {
        None
    }
}

impl BenchSample {
    fn new(elapsed: Duration, iterations: usize, bytes_per_iteration: usize) -> Self {
        let seconds = elapsed.as_secs_f64();
        let iter_per_sec = if seconds > 0.0 {
            f64_from_usize(iterations) / seconds
        } else {
            f64::INFINITY
        };
        let mib_per_sec = if seconds > 0.0 {
            (f64_from_usize(bytes_per_iteration) * f64_from_usize(iterations))
                / (1024.0 * 1024.0 * seconds)
        } else {
            f64::INFINITY
        };

        Self {
            elapsed,
            iter_per_sec,
            mib_per_sec,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct BenchSummary {
    median: BenchSample,
    min_iter_per_sec: f64,
    max_iter_per_sec: f64,
}

fn summarize(samples: &[BenchSample]) -> BenchSummary {
    let mut sorted = samples.to_vec();
    sorted.sort_by(|left, right| left.iter_per_sec.total_cmp(&right.iter_per_sec));
    let median = sorted[sorted.len() / 2];
    let min_iter_per_sec = sorted.first().map_or(0.0, |sample| sample.iter_per_sec);
    let max_iter_per_sec = sorted.last().map_or(0.0, |sample| sample.iter_per_sec);

    BenchSummary {
        median,
        min_iter_per_sec,
        max_iter_per_sec,
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "Benchmark throughput output is an approximate display metric."
)]
const fn f64_from_usize(value: usize) -> f64 {
    value as f64
}
