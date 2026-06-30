mod compare;
#[path = "../../../conformance/harness.rs"]
mod conformance_harness;
mod fixtures;

/// Counting global allocator used only under the `count-alloc` feature so the
/// `alloc-stats` command can report allocation counts without perturbing the
/// timing benchmarks, which run against the unmodified system allocator.
#[cfg(feature = "count-alloc")]
mod counting_alloc {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::sync::atomic::{AtomicUsize, Ordering};

    pub static ALLOCS: AtomicUsize = AtomicUsize::new(0);
    pub static BYTES: AtomicUsize = AtomicUsize::new(0);

    pub struct Counting;

    // SAFETY: every method forwards to the system allocator with the same
    // arguments; the atomics only observe sizes and never alter allocation.
    unsafe impl GlobalAlloc for Counting {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            ALLOCS.fetch_add(1, Ordering::Relaxed);
            BYTES.fetch_add(layout.size(), Ordering::Relaxed);
            unsafe { System.alloc(layout) }
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            unsafe { System.dealloc(ptr, layout) }
        }

        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            ALLOCS.fetch_add(1, Ordering::Relaxed);
            BYTES.fetch_add(new_size.saturating_sub(layout.size()), Ordering::Relaxed);
            unsafe { System.realloc(ptr, layout, new_size) }
        }
    }
}

#[cfg(feature = "count-alloc")]
#[global_allocator]
static GLOBAL_ALLOC: counting_alloc::Counting = counting_alloc::Counting;

use std::env;
use std::fs;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use conformance_harness::{evaluate_all_cases, summarize_evaluations};
use ferrocat_conformance::{ConformanceCase, Expectation, ExpectedArtifact, load_all_manifests};
use ferrocat_icu::{extract_variables, parse_icu, validate_icu};
use ferrocat_po::{
    CatalogCombineInput, CatalogMessage, CatalogMode, CombineCatalogOptions, Header, MsgStr,
    ParseCatalogOptions, ParsedCatalog, PoFile, PoItem, SerializeOptions, TranslationShape,
    UpdateCatalogFileOptions, UpdateCatalogOptions, combine_catalogs, merge_catalog, parse_catalog,
    parse_po, parse_po_borrowed, stringify_po, update_catalog, update_catalog_file,
};
use fixtures::{
    Fixture, IcuFixture, MergeFixture, fixture_by_name, icu_fixture_by_name, merge_fixture_by_name,
};

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
        "regression-check" => compare::run_regression_check_command(args),
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
        "alloc-stats" => {
            let fixture_name = args.next().unwrap_or_else(|| "realistic".to_owned());
            let fixture = load_fixture(&fixture_name)?;
            run_alloc_stats(&fixture)
        }
        "string-stats" => {
            let fixture_name = args.next().unwrap_or_else(|| "realistic".to_owned());
            let fixture = load_fixture(&fixture_name)?;
            run_string_stats(&fixture)
        }
        "parse-catalog-po" => {
            let fixture_name = args
                .next()
                .unwrap_or_else(|| "catalog-modern-de-1000".to_owned());
            let config = parse_bench_config(args, &fixture_name)?;
            let fixture = load_fixture(&fixture_name)?;
            bench_parse_catalog_po(&fixture, config)
        }
        "parse-catalog-fcl" | "parse-fcl" => {
            let fixture_name = args
                .next()
                .unwrap_or_else(|| "catalog-modern-de-1000".to_owned());
            let config = parse_bench_config(args, &fixture_name)?;
            let fixture = load_fixture(&fixture_name)?;
            bench_parse_catalog_fcl(&fixture, config)
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
        "stringify-catalog-po" => {
            let fixture_name = args
                .next()
                .unwrap_or_else(|| "catalog-modern-de-1000".to_owned());
            let config = parse_bench_config(args, &fixture_name)?;
            let fixture = load_fixture(&fixture_name)?;
            bench_stringify_catalog_po(&fixture, config)
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
        "update-catalog-fcl" => {
            let fixture_name = args.next().unwrap_or_else(|| "realistic".to_owned());
            let config = parse_bench_config(args, &fixture_name)?;
            let fixture = load_merge_fixture(&fixture_name)?;
            bench_update_catalog_fcl(&fixture, config)
        }
        "combine-catalogs" => {
            let fixture_name = args
                .next()
                .unwrap_or_else(|| "catalog-modern-de-1000".to_owned());
            let config = parse_bench_config(args, &fixture_name)?;
            let fixture = load_merge_fixture(&fixture_name)?;
            bench_combine_catalogs(&fixture, config)
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
            "unknown command: {other} (use verify-benchmark-env, compare, regression-check, parse, parse-borrowed, parse-catalog-po, parse-catalog-fcl, parse-icu, validate-icu, extract-icu-variables, stringify, stringify-catalog-po, merge, update-catalog, update-catalog-file, update-catalog-fcl, combine-catalogs, describe, or conformance-report)"
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
            "unknown fixture: {fixture_name} (use tiny, realistic, stress, mixed-1000, mixed-10000, gettext-<ui|commerce|saas|content>-<de|fr|pl|ar>-<count>, or catalog-modern-de-<count>)"
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
            "unknown merge fixture: {fixture_name} (use mixed-1000, mixed-10000, gettext-<ui|commerce|saas|content>-<de|fr|pl|ar>-<count>, catalog-modern-de-<count>, catalog-icu-light, catalog-icu-heavy, catalog-icu-projectable, or catalog-icu-unsupported)"
        )
    })
}

fn default_iterations(fixture_name: &str) -> usize {
    match fixture_name {
        "tiny" => 20_000,
        "mixed-10000" => 100,
        "catalog-icu-heavy" => 25,
        "catalog-modern-de-10000" => 100,
        "catalog-icu-projectable" | "catalog-icu-unsupported" => 50,
        "stress" => 1_000,
        name if name.starts_with("gettext-") && name.ends_with("-10000") => 100,
        name if name.starts_with("catalog-modern-") => 400,
        name if name.starts_with("gettext-") => 400,
        name if name.starts_with("icu-") && name.ends_with("-10000") => 50,
        name if name.starts_with("icu-") => 200,
        _ => 5_000,
    }
}

#[cfg(feature = "count-alloc")]
fn run_alloc_stats(fixture: &Fixture) -> Result<(), String> {
    use std::sync::atomic::Ordering;

    use counting_alloc::{ALLOCS, BYTES};

    let content = fixture.content();

    let measure = |label: &str, items: usize, allocs: usize, bytes: usize| {
        let per_item = |value: usize| value as f64 / items.max(1) as f64;
        println!(
            "{label}: items={items} allocs={allocs} ({:.1}/item) bytes={bytes} ({:.0}/item)",
            per_item(allocs),
            per_item(bytes),
        );
    };

    ALLOCS.store(0, Ordering::SeqCst);
    BYTES.store(0, Ordering::SeqCst);
    let owned = parse_po(content).map_err(|error| error.to_string())?;
    let owned_allocs = ALLOCS.load(Ordering::SeqCst);
    let owned_bytes = BYTES.load(Ordering::SeqCst);
    let owned_items = owned.items.len();
    std::hint::black_box(&owned);
    drop(owned);

    ALLOCS.store(0, Ordering::SeqCst);
    BYTES.store(0, Ordering::SeqCst);
    let borrowed = parse_po_borrowed(content).map_err(|error| error.to_string())?;
    let borrowed_allocs = ALLOCS.load(Ordering::SeqCst);
    let borrowed_bytes = BYTES.load(Ordering::SeqCst);
    let borrowed_items = borrowed.items.len();
    std::hint::black_box(&borrowed);
    drop(borrowed);

    ALLOCS.store(0, Ordering::SeqCst);
    BYTES.store(0, Ordering::SeqCst);
    let catalog = parse_catalog(ParseCatalogOptions {
        content,
        locale: inferred_fixture_locale(fixture.name()),
        source_locale: "en",
        mode: CatalogMode::IcuPo,
        strict: false,
    })
    .map_err(|error| error.to_string())?;
    let catalog_allocs = ALLOCS.load(Ordering::SeqCst);
    let catalog_bytes = BYTES.load(Ordering::SeqCst);
    let catalog_items = catalog.messages.len();
    std::hint::black_box(&catalog);
    drop(catalog);

    // Render the same catalog as FCL once outside the measured window, then
    // measure the FCL parse so its allocation profile is comparable to PO.
    let fcl_content = {
        let parsed = parse_catalog(ParseCatalogOptions {
            content,
            locale: inferred_fixture_locale(fixture.name()),
            source_locale: "en",
            mode: CatalogMode::IcuPo,
            strict: false,
        })
        .map_err(|error| error.to_string())?;
        render_fcl_catalog(&parsed)
    };
    ALLOCS.store(0, Ordering::SeqCst);
    BYTES.store(0, Ordering::SeqCst);
    let fcl_catalog = parse_catalog(ParseCatalogOptions {
        content: &fcl_content,
        locale: inferred_fixture_locale(fixture.name()),
        source_locale: "en",
        mode: CatalogMode::IcuFcl,
        strict: false,
    })
    .map_err(|error| error.to_string())?;
    let fcl_allocs = ALLOCS.load(Ordering::SeqCst);
    let fcl_bytes = BYTES.load(Ordering::SeqCst);
    let fcl_items = fcl_catalog.messages.len();
    std::hint::black_box(&fcl_catalog);
    drop(fcl_catalog);

    println!("fixture: {} ({} bytes)", fixture.name(), content.len());
    measure(
        "parse_po (owned)   ",
        owned_items,
        owned_allocs,
        owned_bytes,
    );
    measure(
        "parse_po_borrowed    ",
        borrowed_items,
        borrowed_allocs,
        borrowed_bytes,
    );
    measure(
        "parse_catalog        ",
        catalog_items,
        catalog_allocs,
        catalog_bytes,
    );
    measure("parse_catalog (fcl)  ", fcl_items, fcl_allocs, fcl_bytes);
    Ok(())
}

#[cfg(not(feature = "count-alloc"))]
fn run_alloc_stats(_fixture: &Fixture) -> Result<(), String> {
    Err("alloc-stats requires building with --features count-alloc".to_owned())
}

/// Reports the string-length and collection-size distribution of an owned parse
/// to gauge how much an inline string (`CompactString`, 24-byte inline) or an
/// inline vector (`SmallVec`) representation would eliminate heap allocations.
fn run_string_stats(fixture: &Fixture) -> Result<(), String> {
    let file = parse_po(fixture.content()).map_err(|error| error.to_string())?;

    let mut lengths: Vec<usize> = Vec::new();
    let mut record = |value: &str| lengths.push(value.len());
    for header in &file.headers {
        record(&header.key);
        record(&header.value);
    }
    for comment in file.comments.iter().chain(&file.extracted_comments) {
        record(comment);
    }

    // Collection-size histograms (index 3 == "3 or more") to size SmallVec.
    let mut reference_hist = [0usize; 4];
    let mut comment_hist = [0usize; 4];
    for item in &file.items {
        record(&item.msgid);
        if let Some(context) = &item.msgctxt {
            record(context);
        }
        if let Some(plural) = &item.msgid_plural {
            record(plural);
        }
        for value in item.msgstr.iter() {
            record(value);
        }
        for reference in &item.references {
            record(reference);
        }
        for comment in item.comments.iter().chain(&item.extracted_comments) {
            record(comment);
        }
        for flag in &item.flags {
            record(flag);
        }
        reference_hist[item.references.len().min(3)] += 1;
        comment_hist[(item.comments.len() + item.extracted_comments.len()).min(3)] += 1;
    }

    let total = lengths.len().max(1);
    let share = |count: usize| 100.0 * count as f64 / total as f64;
    let inline = lengths.iter().filter(|&&len| len <= 24).count();
    let mid = lengths
        .iter()
        .filter(|&&len| (25..=48).contains(&len))
        .count();
    let long = lengths.iter().filter(|&&len| len > 48).count();

    println!("fixture: {} ({} items)", fixture.name(), file.items.len());
    println!("strings: {total}");
    println!(
        "  <=24 bytes (CompactString inline): {inline} ({:.1}%)",
        share(inline)
    );
    println!(
        "  25..48 bytes:                       {mid} ({:.1}%)",
        share(mid)
    );
    println!(
        "  >48 bytes:                          {long} ({:.1}%)",
        share(long)
    );
    let items = file.items.len().max(1);
    let item_share = |count: usize| 100.0 * count as f64 / items as f64;
    println!(
        "references/item: 0={} 1={} 2={} 3+={} (1-or-fewer: {:.1}%)",
        reference_hist[0],
        reference_hist[1],
        reference_hist[2],
        reference_hist[3],
        item_share(reference_hist[0] + reference_hist[1]),
    );
    println!(
        "comments/item:   0={} 1={} 2={} 3+={} (1-or-fewer: {:.1}%)",
        comment_hist[0],
        comment_hist[1],
        comment_hist[2],
        comment_hist[3],
        item_share(comment_hist[0] + comment_hist[1]),
    );
    Ok(())
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

fn bench_parse_catalog_po(fixture: &Fixture, config: BenchConfig) -> Result<(), String> {
    let mut parsed_items = 0usize;
    let samples = run_bench(config, || {
        let start = Instant::now();
        for _ in 0..config.iterations {
            let parsed = parse_catalog(ParseCatalogOptions {
                content: fixture.content(),
                locale: inferred_fixture_locale(fixture.name()),
                source_locale: "en",
                mode: CatalogMode::IcuPo,
                strict: false,
            })
            .map_err(|error| error.to_string())?;
            parsed_items = parsed.messages.len();
            std::hint::black_box(parsed);
        }
        Ok(BenchSample::new(
            start.elapsed(),
            config.iterations,
            fixture.content().len(),
        ))
    })?;
    report(
        "parse-catalog-po",
        fixture,
        fixture.content().len(),
        parsed_items,
        config,
        &samples,
    );
    Ok(())
}
fn bench_parse_catalog_fcl(fixture: &Fixture, config: BenchConfig) -> Result<(), String> {
    let (content, locale, items_per_iteration) = fixture_fcl_content(fixture)?;
    let mut parsed_items = 0usize;
    let samples = run_bench(config, || {
        let start = Instant::now();
        for _ in 0..config.iterations {
            let parsed = parse_catalog(ParseCatalogOptions {
                content: &content,
                locale,
                source_locale: "en",
                mode: CatalogMode::IcuFcl,
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
        "parse-catalog-fcl",
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

fn bench_stringify_catalog_po(fixture: &Fixture, config: BenchConfig) -> Result<(), String> {
    let parsed = fixture_parsed_catalog(fixture)?;
    let mut bytes_per_iteration = 0usize;
    let samples = run_bench(config, || {
        let start = Instant::now();
        let mut bytes = 0usize;
        for _ in 0..config.iterations {
            let rendered = render_po_catalog(&parsed);
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
        "stringify-catalog-po",
        fixture,
        bytes_per_iteration,
        parsed.messages.len(),
        config,
        &samples,
    );
    Ok(())
}
fn bench_update_catalog_fcl(fixture: &MergeFixture, config: BenchConfig) -> Result<(), String> {
    // Convert the fixture's existing PO catalog to FCL once so the timed loop runs
    // the FCL write path (parse + merge + `stringify_catalog_fcl`) — the only
    // public way to exercise the library's FCL serializer.
    let parsed = parse_catalog(ParseCatalogOptions {
        content: fixture.existing_po(),
        locale: Some("de"),
        source_locale: "en",
        mode: CatalogMode::IcuPo,
        strict: false,
    })
    .map_err(|error| error.to_string())?;
    let existing_fcl = render_fcl_catalog(&parsed);

    let mut bytes_per_iteration = 0usize;
    let samples = run_bench(config, || {
        let start = Instant::now();
        let mut bytes = 0usize;
        for _ in 0..config.iterations {
            let rendered = update_catalog(UpdateCatalogOptions {
                locale: Some("de"),
                mode: CatalogMode::IcuFcl,
                existing: Some(&existing_fcl),
                ..UpdateCatalogOptions::new("en", fixture.api_extracted_messages().to_vec())
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
        "update-catalog-fcl",
        fixture,
        bytes_per_iteration,
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
                existing: Some(fixture.existing_po()),
                ..UpdateCatalogOptions::new("en", fixture.api_extracted_messages().to_vec())
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
                options: UpdateCatalogOptions {
                    locale: Some("de"),
                    ..UpdateCatalogOptions::new("en", fixture.api_extracted_messages().to_vec())
                },
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
fn bench_combine_catalogs(fixture: &MergeFixture, config: BenchConfig) -> Result<(), String> {
    let mut bytes_per_iteration = 0usize;
    let inputs = [
        CatalogCombineInput::labeled(fixture.existing_po(), "base"),
        CatalogCombineInput::labeled(fixture.existing_po(), "overlay"),
        CatalogCombineInput::labeled(fixture.existing_po(), "fallback"),
    ];

    let samples = run_bench(config, || {
        let start = Instant::now();
        let mut bytes = 0usize;
        for _ in 0..config.iterations {
            let rendered = combine_catalogs(CombineCatalogOptions {
                inputs: &inputs,
                locale: Some("de"),
                source_locale: "en",
                mode: CatalogMode::IcuPo,
                ..CombineCatalogOptions::new(&[], "en")
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
        "combine-catalogs",
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
fn fixture_fcl_content(fixture: &Fixture) -> Result<(String, Option<&'static str>, usize), String> {
    let parsed = fixture_parsed_catalog(fixture)?;
    let rendered = render_fcl_catalog(&parsed);
    Ok((
        rendered,
        inferred_fixture_locale(fixture.name()),
        parsed.messages.len(),
    ))
}

/// Renders a parsed catalog as FCL text, reusing the canonical per-message field
/// renderers so PO and FCL are compared on identical content.
pub(crate) fn render_fcl_catalog(parsed: &ParsedCatalog) -> String {
    let mut out = String::from("%FCL1\tsource=en");
    if let Some(locale) = &parsed.locale {
        out.push_str("\tlocale=");
        fcl_escape_into(&mut out, locale);
    }
    out.push('\n');

    // FCL is canonically sorted by (id, ctxt); the reader enforces it. Build one
    // line per message keyed by (id, ctxt), then sort before emitting.
    let mut entries: Vec<(String, Option<String>, String)> = parsed
        .messages
        .iter()
        .map(|message| {
            let id = render_canonical_id(message);
            let mut line = String::new();
            fcl_escape_into(&mut line, &id);
            line.push('\t');
            fcl_escape_into(&mut line, message.msgctxt.as_deref().unwrap_or(""));
            line.push('\t');
            fcl_escape_into(&mut line, &render_canonical_translation(message));

            let mut refs = message
                .origin
                .iter()
                .map(|origin| origin.file.clone())
                .collect::<Vec<_>>();
            refs.sort_unstable();
            refs.dedup();
            for reference in &refs {
                line.push_str("\tr=");
                fcl_escape_into(&mut line, reference);
            }
            for comment in &message.comments {
                line.push_str("\tc=");
                fcl_escape_into(&mut line, comment);
            }
            if message.obsolete.is_some() {
                line.push_str("\to");
            }
            line.push('\n');
            (id, message.msgctxt.clone(), line)
        })
        .collect();
    entries.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));
    for (_, _, line) in &entries {
        out.push_str(line);
    }

    out
}

fn fcl_escape_into(out: &mut String, value: &str) {
    if !value
        .bytes()
        .any(|byte| matches!(byte, b'\\' | b'\t' | b'\n'))
    {
        out.push_str(value);
        return;
    }
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            other => out.push(other),
        }
    }
}
fn fixture_parsed_catalog(fixture: &Fixture) -> Result<ParsedCatalog, String> {
    parse_catalog(ParseCatalogOptions {
        content: fixture.content(),
        locale: inferred_fixture_locale(fixture.name()),
        source_locale: "en",
        mode: CatalogMode::IcuPo,
        strict: false,
    })
    .map_err(|error| error.to_string())
}

fn render_po_catalog(parsed: &ParsedCatalog) -> String {
    let mut file = PoFile {
        comments: vec!["Benchmark-generated catalog render".to_owned()],
        extracted_comments: Vec::new(),
        headers: Vec::new(),
        items: Vec::with_capacity(parsed.messages.len()),
    };

    if let Some(locale) = &parsed.locale {
        file.headers.push(Header {
            key: "Language".to_owned(),
            value: locale.clone(),
        });
    }
    file.headers.push(Header {
        key: "Content-Type".to_owned(),
        value: "text/plain; charset=UTF-8".to_owned(),
    });
    file.headers.push(Header {
        key: "Content-Transfer-Encoding".to_owned(),
        value: "8bit".to_owned(),
    });

    for message in &parsed.messages {
        let mut item = PoItem::new(1);
        item.msgid = render_canonical_id(message);
        item.msgctxt = message.msgctxt.clone();
        item.msgstr = MsgStr::from(render_canonical_translation(message));
        item.extracted_comments = message.comments.clone().into();
        item.references = message
            .origin
            .iter()
            .map(|origin| origin.file.clone())
            .collect();
        item.obsolete = message.obsolete.is_some();
        file.items.push(item);
    }

    stringify_po(&file, &SerializeOptions::default())
}
fn render_canonical_id(message: &CatalogMessage) -> String {
    match &message.translation {
        TranslationShape::Singular { .. } => message.msgid.clone(),
        TranslationShape::Plural {
            source, variable, ..
        } => synthesize_icu_plural(variable, source.one.as_deref(), &source.other),
    }
}

fn render_canonical_translation(message: &CatalogMessage) -> String {
    match &message.translation {
        TranslationShape::Singular { value } => value.clone(),
        TranslationShape::Plural {
            translation,
            variable,
            ..
        } => synthesize_icu_plural_map(variable, translation),
    }
}
fn synthesize_icu_plural_map(
    variable: &str,
    forms: &std::collections::BTreeMap<String, String>,
) -> String {
    let mut rendered = format!("{{{variable}, plural,");
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
    let mut rendered = format!("{{{variable}, plural,");
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
    } else if parts.len() >= 4
        && parts.first() == Some(&"catalog")
        && parts.get(1) == Some(&"modern")
    {
        match parts[2] {
            "de" => Some("de"),
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

#[cfg(test)]
mod tests {
    use ferrocat_po::{CatalogMessage, CatalogMode, ParseCatalogOptions, parse_catalog};

    use super::{fixture_by_name, fixture_parsed_catalog, render_fcl_catalog, render_po_catalog};
    #[test]
    fn benchmark_po_catalog_renderer_roundtrips_modern_fixture() {
        let fixture = fixture_by_name("catalog-modern-de-1000").expect("fixture exists");
        let parsed = fixture_parsed_catalog(&fixture).expect("parse PO catalog");
        let rendered = render_po_catalog(&parsed);
        let reparsed = parse_catalog(ParseCatalogOptions {
            content: &rendered,
            locale: Some("de"),
            source_locale: "en",
            mode: CatalogMode::IcuPo,
            strict: false,
        })
        .expect("reparse rendered PO");

        assert_eq!(parsed.locale, reparsed.locale);
        assert_eq!(parsed.messages, reparsed.messages);
    }

    #[test]
    fn benchmark_fcl_catalog_renderer_matches_po_messages() {
        let fixture = fixture_by_name("catalog-modern-de-1000").expect("fixture exists");
        let po_parsed = fixture_parsed_catalog(&fixture).expect("parse PO catalog");
        let rendered = render_fcl_catalog(&po_parsed);
        let fcl_parsed = parse_catalog(ParseCatalogOptions {
            content: &rendered,
            locale: Some("de"),
            source_locale: "en",
            mode: CatalogMode::IcuFcl,
            strict: false,
        })
        .expect("parse rendered FCL");

        assert_eq!(po_parsed.locale, fcl_parsed.locale);
        // FCL is canonically sorted by (id, ctxt) on render, so sort both sides by
        // the same key before comparing the message sets.
        let key = |message: &CatalogMessage| (message.msgid.clone(), message.msgctxt.clone());
        let mut po_messages = po_parsed.messages;
        let mut fcl_messages = fcl_parsed.messages;
        po_messages.sort_by_key(key);
        fcl_messages.sort_by_key(key);
        assert_eq!(po_messages, fcl_messages);
    }
}
