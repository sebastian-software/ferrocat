//! Stable compare-report schema and measurement statistics.

use super::*;

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct CompareReport {
    pub(super) profile: String,
    pub(super) generated_at: String,
    pub(super) reference_host_policy: String,
    pub(super) environment: EnvironmentMetadata,
    pub(super) scenarios: Vec<ScenarioReport>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct ScenarioReport {
    pub(super) id: String,
    pub(super) comparison_group: String,
    pub(super) workload: String,
    pub(super) operation: String,
    pub(super) fixture: String,
    pub(super) implementation: String,
    pub(super) tool_version: String,
    pub(super) iterations_per_sample: usize,
    pub(super) warmup_runs: usize,
    pub(super) measured_runs: usize,
    pub(super) semantic_digest: String,
    pub(super) baseline_strategy: Option<String>,
    pub(super) baseline_fixture: Option<String>,
    pub(super) statistics: ScenarioStatistics,
    pub(super) samples: Vec<ScenarioSampleReport>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct ScenarioSampleReport {
    pub(super) elapsed_ns: u128,
    pub(super) baseline_elapsed_ns: Option<u128>,
    pub(super) adjusted_elapsed_ns: Option<u128>,
    pub(super) bytes_processed: u64,
    pub(super) items_processed: Option<u64>,
    pub(super) messages_processed: Option<u64>,
    pub(super) mib_per_sec: f64,
    pub(super) units_per_sec: f64,
    pub(super) adjusted_mib_per_sec: Option<f64>,
    pub(super) adjusted_units_per_sec: Option<f64>,
}

impl ScenarioSampleReport {
    pub(super) fn from_execution(sample: &ExecutionResult) -> Self {
        let elapsed_seconds = nanos_to_seconds(sample.elapsed_ns);
        let units = f64_from_u64(
            sample
                .items_processed
                .or(sample.messages_processed)
                .unwrap_or(0),
        );
        let adjusted_elapsed_ns = sample.adjusted_elapsed_ns();
        let adjusted_seconds = adjusted_elapsed_ns.map(nanos_to_seconds);
        Self {
            elapsed_ns: sample.elapsed_ns,
            baseline_elapsed_ns: sample.baseline_elapsed_ns,
            adjusted_elapsed_ns,
            bytes_processed: sample.bytes_processed,
            items_processed: sample.items_processed,
            messages_processed: sample.messages_processed,
            mib_per_sec: throughput_mib(sample.bytes_processed, elapsed_seconds),
            units_per_sec: throughput_units(units, elapsed_seconds),
            adjusted_mib_per_sec: adjusted_seconds
                .filter(|seconds| *seconds > 0.0)
                .map(|seconds| throughput_mib(sample.bytes_processed, seconds)),
            adjusted_units_per_sec: adjusted_seconds
                .filter(|seconds| *seconds > 0.0)
                .map(|seconds| throughput_units(units, seconds)),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct ScenarioStatistics {
    pub(super) median_elapsed_ns: u128,
    pub(super) mean_elapsed_ns: f64,
    pub(super) min_elapsed_ns: u128,
    pub(super) max_elapsed_ns: u128,
    pub(super) stddev_elapsed_ns: f64,
    pub(super) median_absolute_deviation_ns: u128,
    pub(super) coefficient_of_variation: f64,
    pub(super) relative_span_percent: f64,
    pub(super) noisy: bool,
    pub(super) median_mib_per_sec: f64,
    pub(super) median_units_per_sec: f64,
    pub(super) median_baseline_elapsed_ns: Option<u128>,
    pub(super) median_adjusted_elapsed_ns: Option<u128>,
    pub(super) median_adjusted_mib_per_sec: Option<f64>,
    pub(super) median_adjusted_units_per_sec: Option<f64>,
}

impl ScenarioStatistics {
    #[expect(
        clippy::too_many_lines,
        reason = "Benchmark statistics are computed together so the report contract stays easy to verify."
    )]
    pub(super) fn from_samples(samples: &[ExecutionResult]) -> Self {
        let mut elapsed = samples
            .iter()
            .map(|sample| sample.elapsed_ns)
            .collect::<Vec<_>>();
        elapsed.sort_unstable();
        let median_elapsed_ns = elapsed[elapsed.len() / 2];
        let min_elapsed_ns = *elapsed.first().unwrap_or(&0);
        let max_elapsed_ns = *elapsed.last().unwrap_or(&0);
        let mean = elapsed
            .iter()
            .map(|value| f64_from_u128(*value))
            .sum::<f64>()
            / f64_from_usize(elapsed.len());
        let variance = elapsed
            .iter()
            .map(|value| {
                let delta = f64_from_u128(*value) - mean;
                delta * delta
            })
            .sum::<f64>()
            / f64_from_usize(elapsed.len());
        let median_absolute_deviation_ns = elapsed
            .iter()
            .map(|value| value.abs_diff(median_elapsed_ns))
            .collect::<Vec<_>>();
        let median_absolute_deviation_ns =
            median_u128(median_absolute_deviation_ns).unwrap_or_default();
        let stddev_elapsed_ns = variance.sqrt();
        let coefficient_of_variation = if mean <= f64::EPSILON {
            0.0
        } else {
            stddev_elapsed_ns / mean
        };
        let relative_span_percent = if median_elapsed_ns == 0 {
            0.0
        } else {
            (f64_from_u128(max_elapsed_ns - min_elapsed_ns) / f64_from_u128(median_elapsed_ns))
                * 100.0
        };

        let mut sample_reports = samples
            .iter()
            .map(|sample| {
                let seconds = nanos_to_seconds(sample.elapsed_ns);
                (
                    throughput_mib(sample.bytes_processed, seconds),
                    throughput_units(
                        f64_from_u64(
                            sample
                                .items_processed
                                .or(sample.messages_processed)
                                .unwrap_or(0),
                        ),
                        seconds,
                    ),
                )
            })
            .collect::<Vec<_>>();
        sample_reports.sort_by(|left, right| left.0.total_cmp(&right.0));
        let median_mib_per_sec = sample_reports
            .get(sample_reports.len() / 2)
            .map_or(0.0, |entry| entry.0);
        sample_reports.sort_by(|left, right| left.1.total_cmp(&right.1));
        let median_units_per_sec = sample_reports
            .get(sample_reports.len() / 2)
            .map_or(0.0, |entry| entry.1);

        let baseline_elapsed = samples
            .iter()
            .filter_map(|sample| sample.baseline_elapsed_ns)
            .collect::<Vec<_>>();
        let adjusted_elapsed = samples
            .iter()
            .filter_map(ExecutionResult::adjusted_elapsed_ns)
            .collect::<Vec<_>>();
        let adjusted_rates = samples
            .iter()
            .filter_map(|sample| {
                let adjusted_ns = sample.adjusted_elapsed_ns()?;
                if adjusted_ns == 0 {
                    return None;
                }
                let seconds = nanos_to_seconds(adjusted_ns);
                Some((
                    throughput_mib(sample.bytes_processed, seconds),
                    throughput_units(
                        f64_from_u64(
                            sample
                                .items_processed
                                .or(sample.messages_processed)
                                .unwrap_or(0),
                        ),
                        seconds,
                    ),
                ))
            })
            .collect::<Vec<_>>();
        let median_baseline_elapsed_ns = median_u128(baseline_elapsed);
        let median_adjusted_elapsed_ns = median_u128(adjusted_elapsed);
        let median_adjusted_mib_per_sec = median_f64(
            adjusted_rates
                .iter()
                .map(|entry| entry.0)
                .collect::<Vec<_>>(),
        );
        let median_adjusted_units_per_sec = median_f64(
            adjusted_rates
                .iter()
                .map(|entry| entry.1)
                .collect::<Vec<_>>(),
        );

        Self {
            median_elapsed_ns,
            mean_elapsed_ns: mean,
            min_elapsed_ns,
            max_elapsed_ns,
            stddev_elapsed_ns,
            median_absolute_deviation_ns,
            coefficient_of_variation,
            relative_span_percent,
            noisy: coefficient_of_variation > NOISE_CV_WARNING_THRESHOLD
                || relative_span_percent > NOISE_RELATIVE_SPAN_WARNING_THRESHOLD,
            median_mib_per_sec,
            median_units_per_sec,
            median_baseline_elapsed_ns,
            median_adjusted_elapsed_ns,
            median_adjusted_mib_per_sec,
            median_adjusted_units_per_sec,
        }
    }
}

pub(super) fn median_u128(mut values: Vec<u128>) -> Option<u128> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    values.get(values.len() / 2).copied()
}

pub(super) fn median_f64(mut values: Vec<f64>) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    values.get(values.len() / 2).copied()
}

pub(super) fn round_robin_schedule(run_counts: &[usize]) -> Vec<usize> {
    let mut schedule = Vec::new();
    let max_runs = run_counts.iter().copied().max().unwrap_or(0);
    for round in 0..max_runs {
        for (index, run_count) in run_counts.iter().copied().enumerate() {
            if round < run_count {
                schedule.push(index);
            }
        }
    }
    schedule
}

pub(super) fn nanos_to_seconds(value: u128) -> f64 {
    f64_from_u128(value) / 1_000_000_000.0
}

pub(super) fn nanos_to_millis(value: u128) -> f64 {
    f64_from_u128(value) / 1_000_000.0
}

pub(super) fn throughput_mib(bytes: u64, seconds: f64) -> f64 {
    if seconds <= 0.0 {
        return f64::INFINITY;
    }
    f64_from_u64(bytes) / (1024.0 * 1024.0 * seconds)
}

pub(super) fn throughput_units(units: f64, seconds: f64) -> f64 {
    if seconds <= 0.0 {
        return f64::INFINITY;
    }
    units / seconds
}

#[expect(
    clippy::cast_precision_loss,
    reason = "Benchmark summaries intentionally use approximate floating-point display values."
)]
pub(super) const fn f64_from_u128(value: u128) -> f64 {
    value as f64
}

#[expect(
    clippy::cast_precision_loss,
    reason = "Benchmark summaries intentionally use approximate floating-point display values."
)]
pub(super) const fn f64_from_u64(value: u64) -> f64 {
    value as f64
}

#[expect(
    clippy::cast_precision_loss,
    reason = "Benchmark summaries intentionally use approximate floating-point display values."
)]
pub(super) const fn f64_from_usize(value: usize) -> f64 {
    value as f64
}

#[derive(Debug, Clone)]
pub(super) struct ExecutionResult {
    pub(super) tool_version: String,
    pub(super) reported_digest: String,
    pub(super) elapsed_ns: u128,
    pub(super) baseline_elapsed_ns: Option<u128>,
    pub(super) bytes_processed: u64,
    pub(super) items_processed: Option<u64>,
    pub(super) messages_processed: Option<u64>,
    pub(super) artifact: Option<ExecutionArtifact>,
}

impl ExecutionResult {
    pub(super) fn adjusted_elapsed_ns(&self) -> Option<u128> {
        self.baseline_elapsed_ns
            .map(|baseline| self.elapsed_ns.saturating_sub(baseline))
    }
}

#[derive(Debug, Clone)]
pub(super) enum ExecutionArtifact {
    PoSummary(PoSemanticSummary),
    CatalogSummary(CatalogSemanticSummary),
    IcuSummary(IcuFixtureSummary),
    RenderedPo(String),
    RenderedPoPath(PathBuf),
}
