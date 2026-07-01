use std::fs;
use std::path::Path;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct MetricSummary {
    pub wall_clock: Duration,
    pub cli_calls: usize,
    pub agent_visible_bytes: u64,
    pub artifact_bytes: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub metrics_source: &'static str,
}

impl MetricSummary {
    pub fn new(wall_clock: Duration, artifact_bytes: u64) -> Self {
        Self {
            wall_clock,
            cli_calls: 1,
            agent_visible_bytes: 0,
            artifact_bytes,
            cache_hits: 0,
            cache_misses: 0,
            metrics_source: "estimated",
        }
    }

    pub fn agent_visible_tokens(&self) -> u64 {
        estimate_tokens(self.agent_visible_bytes)
    }

    pub fn artifact_tokens_preserved(&self) -> u64 {
        estimate_tokens(self.artifact_bytes)
    }

    pub fn avoided_tokens(&self) -> u64 {
        self.artifact_tokens_preserved()
            .saturating_sub(self.agent_visible_tokens())
    }
}

pub fn render_with_metrics<F>(mut metrics: MetricSummary, render: F) -> String
where
    F: Fn(&MetricSummary) -> String,
{
    let mut output = render(&metrics);
    for _ in 0..4 {
        let visible_bytes = output.len() as u64;
        if visible_bytes == metrics.agent_visible_bytes {
            break;
        }
        metrics.agent_visible_bytes = visible_bytes;
        output = render(&metrics);
    }
    output
}

pub fn push_metric_block(output: &mut String, metrics: &MetricSummary) {
    output.push_str("metrics:\n");
    output.push_str(&format!(
        "  wall_clock: {}\n",
        format_duration(metrics.wall_clock)
    ));
    output.push_str(&format!("  cli_calls: {}\n", metrics.cli_calls));
    output.push_str(&format!(
        "  agent_visible_tokens: ~{}\n",
        metrics.agent_visible_tokens()
    ));
    output.push_str(&format!(
        "  artifact_tokens_preserved: ~{}\n",
        metrics.artifact_tokens_preserved()
    ));
    output.push_str(&format!(
        "  avoided_tokens: ~{}\n",
        metrics.avoided_tokens()
    ));
    output.push_str(&format!("  cache_hits: {}\n", metrics.cache_hits));
    output.push_str(&format!("  cache_misses: {}\n", metrics.cache_misses));
    output.push_str(&format!("  metrics_source: {}\n", metrics.metrics_source));
}

pub fn existing_paths_bytes<I, P>(paths: I) -> u64
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    paths
        .into_iter()
        .filter_map(|path| fs::metadata(path.as_ref()).ok())
        .filter(|metadata| metadata.is_file())
        .map(|metadata| metadata.len())
        .sum()
}

pub fn estimate_tokens(bytes: u64) -> u64 {
    bytes.saturating_add(3) / 4
}

fn format_duration(duration: Duration) -> String {
    if duration.as_secs() > 0 {
        format!("{:.2}s", duration.as_secs_f64())
    } else {
        format!("{}ms", duration.as_millis())
    }
}
