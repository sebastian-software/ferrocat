use std::env;
use std::process::ExitCode;
use std::time::Instant;

use ferrox_po::{SerializeOptions, parse_po, stringify_po};

const TINY_FIXTURE: &str = include_str!("../fixtures/tiny.po");
const REALISTIC_FIXTURE: &str = include_str!("../fixtures/realistic.po");
const STRESS_FIXTURE: &str = include_str!("../fixtures/stress.po");

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
        None => 5_000,
    };

    let fixture = fixture_by_name(&fixture_name)
        .ok_or_else(|| format!("unknown fixture: {fixture_name} (use tiny, realistic, stress)"))?;

    match command.as_str() {
        "parse" => bench_parse(fixture_name.as_str(), fixture, iterations),
        "stringify" => bench_stringify(fixture_name.as_str(), fixture, iterations),
        other => Err(format!("unknown command: {other} (use parse or stringify)")),
    }
}

fn fixture_by_name(name: &str) -> Option<&'static str> {
    match name {
        "tiny" => Some(TINY_FIXTURE),
        "realistic" => Some(REALISTIC_FIXTURE),
        "stress" => Some(STRESS_FIXTURE),
        _ => None,
    }
}

fn bench_parse(name: &str, fixture: &str, iterations: usize) -> Result<(), String> {
    let start = Instant::now();
    let mut items_per_iteration = 0usize;
    for _ in 0..iterations {
        let file = parse_po(fixture).map_err(|error| error.to_string())?;
        items_per_iteration = file.items.len();
        std::hint::black_box(file);
    }
    let elapsed = start.elapsed();
    report(
        "parse",
        name,
        fixture.len(),
        iterations,
        items_per_iteration,
        elapsed,
    );
    Ok(())
}

fn bench_stringify(name: &str, fixture: &str, iterations: usize) -> Result<(), String> {
    let file = parse_po(fixture).map_err(|error| error.to_string())?;
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
        name,
        bytes / iterations.max(1),
        iterations,
        file.items.len(),
        elapsed,
    );
    Ok(())
}

fn report(
    command: &str,
    fixture: &str,
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
    println!("fixture: {fixture}");
    println!("iterations: {iterations}");
    println!("items/iteration: {items_per_iteration}");
    println!("bytes/iteration: {bytes_per_iteration}");
    println!("elapsed: {:.3}s", seconds);
    println!("iter/s: {:.1}", iter_per_sec);
    println!("MiB/s: {:.2}", mib_per_sec);
}
