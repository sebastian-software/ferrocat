mod fixtures;

use std::env;
use std::fs;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use ferrox_po::{
    PluralEncoding, SerializeOptions, UpdateCatalogFileOptions, UpdateCatalogOptions,
    merge_catalog, parse_po, parse_po_borrowed, stringify_po, update_catalog, update_catalog_file,
};
use fixtures::{Fixture, MergeFixture, fixture_by_name, merge_fixture_by_name};

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
    let fixture_name = args.next().unwrap_or_else(|| "realistic".to_owned());
    let config = parse_bench_config(args, &fixture_name)?;

    match command.as_str() {
        "parse" => {
            let fixture = load_fixture(&fixture_name)?;
            bench_parse(&fixture, config)
        }
        "parse-borrowed" => {
            let fixture = load_fixture(&fixture_name)?;
            bench_parse_borrowed(&fixture, config)
        }
        "stringify" => {
            let fixture = load_fixture(&fixture_name)?;
            bench_stringify(&fixture, config)
        }
        "merge" => {
            let fixture = load_merge_fixture(&fixture_name)?;
            bench_merge(&fixture, config)
        }
        "update-catalog" => {
            let fixture = load_merge_fixture(&fixture_name)?;
            bench_update_catalog(&fixture, config)
        }
        "update-catalog-file" => {
            let fixture = load_merge_fixture(&fixture_name)?;
            bench_update_catalog_file(&fixture, config)
        }
        "describe" => {
            let fixture = load_fixture(&fixture_name)?;
            describe(&fixture);
            Ok(())
        }
        other => Err(format!(
            "unknown command: {other} (use parse, parse-borrowed, stringify, merge, update-catalog, update-catalog-file, or describe)"
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
    let mut args = args.peekable();

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
            "unknown fixture: {fixture_name} (use tiny, realistic, stress, mixed-1000, mixed-10000)"
        )
    })
}

fn load_merge_fixture(fixture_name: &str) -> Result<MergeFixture, String> {
    merge_fixture_by_name(fixture_name).ok_or_else(|| {
        format!("unknown merge fixture: {fixture_name} (use mixed-1000 or mixed-10000)")
    })
}

fn default_iterations(fixture_name: &str) -> usize {
    match fixture_name {
        "tiny" => 20_000,
        "mixed-10000" => 100,
        "stress" => 1_000,
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
                locale: Some("de".to_owned()),
                source_locale: "en".to_owned(),
                extracted: fixture.api_extracted_messages().to_vec(),
                existing: Some(fixture.existing_po().to_owned()),
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
        "ferrox-bench-update-catalog-file-{}",
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
                target_path: path.clone(),
                locale: Some("de".to_owned()),
                source_locale: "en".to_owned(),
                extracted: fixture.api_extracted_messages().to_vec(),
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

impl BenchSample {
    fn new(elapsed: Duration, iterations: usize, bytes_per_iteration: usize) -> Self {
        let seconds = elapsed.as_secs_f64();
        let iter_per_sec = if seconds > 0.0 {
            iterations as f64 / seconds
        } else {
            f64::INFINITY
        };
        let mib_per_sec = if seconds > 0.0 {
            (bytes_per_iteration as f64 * iterations as f64) / (1024.0 * 1024.0 * seconds)
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
