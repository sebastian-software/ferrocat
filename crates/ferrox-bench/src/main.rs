mod fixtures;

use std::env;
use std::process::ExitCode;
use std::time::Instant;

use ferrox_po::{SerializeOptions, merge_catalog, parse_po, parse_po_borrowed, stringify_po};
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
    let iterations = match args.next() {
        Some(value) => value
            .parse::<usize>()
            .map_err(|_| format!("invalid iteration count: {value}"))?,
        None => default_iterations(&fixture_name),
    };

    match command.as_str() {
        "parse" => {
            let fixture = load_fixture(&fixture_name)?;
            bench_parse(&fixture, iterations)
        }
        "parse-borrowed" => {
            let fixture = load_fixture(&fixture_name)?;
            bench_parse_borrowed(&fixture, iterations)
        }
        "stringify" => {
            let fixture = load_fixture(&fixture_name)?;
            bench_stringify(&fixture, iterations)
        }
        "merge" => {
            let fixture = load_merge_fixture(&fixture_name)?;
            bench_merge(&fixture, iterations)
        }
        "describe" => {
            let fixture = load_fixture(&fixture_name)?;
            describe(&fixture);
            Ok(())
        }
        other => Err(format!(
            "unknown command: {other} (use parse, parse-borrowed, stringify, merge, or describe)"
        )),
    }
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

fn bench_parse(fixture: &Fixture, iterations: usize) -> Result<(), String> {
    let start = Instant::now();
    let mut items_per_iteration = 0usize;
    for _ in 0..iterations {
        let file = parse_po(fixture.content()).map_err(|error| error.to_string())?;
        items_per_iteration = file.items.len();
        std::hint::black_box(file);
    }
    let elapsed = start.elapsed();
    report(
        "parse",
        fixture,
        fixture.content().len(),
        iterations,
        items_per_iteration,
        elapsed,
    );
    Ok(())
}

fn bench_parse_borrowed(fixture: &Fixture, iterations: usize) -> Result<(), String> {
    let start = Instant::now();
    let mut items_per_iteration = 0usize;
    for _ in 0..iterations {
        let file = parse_po_borrowed(fixture.content()).map_err(|error| error.to_string())?;
        items_per_iteration = file.items.len();
        std::hint::black_box(file);
    }
    let elapsed = start.elapsed();
    report(
        "parse-borrowed",
        fixture,
        fixture.content().len(),
        iterations,
        items_per_iteration,
        elapsed,
    );
    Ok(())
}

fn bench_stringify(fixture: &Fixture, iterations: usize) -> Result<(), String> {
    let file = parse_po(fixture.content()).map_err(|error| error.to_string())?;
    let options = SerializeOptions::default();

    let start = Instant::now();
    let mut bytes = 0usize;
    for _ in 0..iterations {
        let rendered = stringify_po(&file, &options);
        bytes += rendered.len();
        std::hint::black_box(rendered);
    }
    let elapsed = start.elapsed();
    report(
        "stringify",
        fixture,
        bytes / iterations.max(1),
        iterations,
        file.items.len(),
        elapsed,
    );
    Ok(())
}

fn bench_merge(fixture: &MergeFixture, iterations: usize) -> Result<(), String> {
    let start = Instant::now();
    let mut bytes = 0usize;
    for _ in 0..iterations {
        let rendered = merge_catalog(fixture.existing_po(), fixture.extracted_messages())
            .map_err(|error| error.to_string())?;
        bytes += rendered.len();
        std::hint::black_box(rendered);
    }
    let elapsed = start.elapsed();
    report_merge(
        "merge",
        fixture,
        bytes / iterations.max(1),
        iterations,
        elapsed,
    );
    Ok(())
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
    iterations: usize,
    elapsed: std::time::Duration,
) {
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

    println!("command: {command}");
    println!("fixture: {}", fixture.name());
    println!("kind: {}", fixture.kind());
    println!("iterations: {iterations}");
    println!("existing-items: {}", fixture.existing_entries());
    println!("extracted-items: {}", fixture.extracted_entries());
    println!("bytes/iteration: {bytes_per_iteration}");
    println!("elapsed: {:.3}s", seconds);
    println!("iter/s: {:.1}", iter_per_sec);
    println!("MiB/s: {:.2}", mib_per_sec);
}

fn report(
    command: &str,
    fixture: &Fixture,
    bytes_per_iteration: usize,
    iterations: usize,
    items_per_iteration: usize,
    elapsed: std::time::Duration,
) {
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

    println!("command: {command}");
    println!("fixture: {}", fixture.name());
    println!("kind: {}", fixture.kind());
    println!("iterations: {iterations}");
    println!("items/iteration: {items_per_iteration}");
    println!("bytes/iteration: {bytes_per_iteration}");
    println!("elapsed: {:.3}s", seconds);
    println!("iter/s: {:.1}", iter_per_sec);
    println!("MiB/s: {:.2}", mib_per_sec);
}
