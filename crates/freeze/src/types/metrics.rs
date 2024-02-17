use comfy_table::{presets::UTF8_FULL, Cell, Table};
use human_bytes::human_bytes;
use std::collections::HashMap;
use tokio::sync::mpsc;

/// Alias type for response size in bytes.
type ResponseSize = u64;

/// Alias type for response time in nanoseconds.
type ResponseTime = u128;

/// Metric datapoint
#[derive(Debug)]
pub struct MetricsData {
    /// Method name. Example `get_logs`
    pub method_name: &'static str,
    /// Duration in nanoseconds of the request
    pub duration: u128,
    /// Response size
    pub response_size: u64,
}

/// Metrics report over a specific RPC method.
#[derive(Debug)]
pub struct MetricsReport {
    /// Maximum size of all responses.
    max_size: ResponseSize,
    /// Minimum size of all responses.
    min_size: ResponseSize,
    /// Maximum waiting time for a response.
    max_time: ResponseTime,
    /// Minimum waiting time for a response.
    min_time: ResponseTime,
    /// Average waiting time for a response.
    avg_size: ResponseSize,
    /// Average size of a response.
    avg_time: ResponseTime,
    /// Total duration of all requests.
    total_duration: ResponseTime,
    /// Total size of all responses.
    total_size: ResponseSize,
}
impl MetricsReport {
    /// Pretty prints report into a table.
    pub fn pretty_print(reports: HashMap<&str, MetricsReport>) {
        let mut table = Table::new();
        table.load_preset(UTF8_FULL).set_header(vec![
            "Method",
            "Max Size (KB)",
            "Min Size (KB)",
            "Max Time (s)",
            "Min Time (s)",
            "Avg Time (s)",
            "Avg Size (KB)",
            "Total Duration (s)",
            "Total Size (KB)",
        ]);

        for (method, report) in reports {
            table.add_row(vec![
                Cell::new(method),
                Cell::new(&format!("{:.2}", human_bytes(report.max_size as f64))),
                Cell::new(&format!("{:.2}", human_bytes(report.min_size as f64))),
                Cell::new(&format!("{:.6}", report.max_time as f64 / 1_000_000_000.0)),
                Cell::new(&format!("{:.6}", report.min_time as f64 / 1_000_000_000.0)),
                Cell::new(&format!("{:.6}", report.avg_time as f64 / 1_000_000_000.0)),
                Cell::new(&format!("{:.2}", human_bytes(report.avg_size as f64))),
                Cell::new(&format!("{:.6}", report.total_duration as f64 / 1_000_000_000.0)),
                Cell::new(&format!("{:.2}", human_bytes(report.total_size as f64))),
            ]);
        }
        println!("{table}")
    }
}

/// Collects and aggregate metrics returning a report for each method at the end.
pub async fn metrics_aggregator(
    mut receiver: mpsc::Receiver<MetricsData>,
) -> HashMap<&'static str, MetricsReport> {
    let mut metrics = HashMap::new();
    while let Some(MetricsData { method_name, response_size, duration }) = receiver.recv().await {
        metrics.entry(method_name).or_insert_with(Vec::new).push((response_size, duration));
    }

    metrics
        .into_iter()
        .map(|(method, records)| {
            let (sizes, times): (Vec<_>, Vec<_>) = records.into_iter().unzip();
            let (max_size, min_size, total_size) =
                sizes.iter().fold((0, u64::MAX, 0), |(max, min, total), &size| {
                    (max.max(size), min.min(size), total + size)
                });
            let (max_time, min_time, total_duration) =
                times.iter().fold((0, u128::MAX, 0), |(max, min, total), &time| {
                    (max.max(time), min.min(time), total + time)
                });
            let avg_size =
                if !sizes.is_empty() { sizes.iter().sum::<u64>() / sizes.len() as u64 } else { 0 };
            let avg_time = if !times.is_empty() { total_duration / times.len() as u128 } else { 0 };
            (
                method,
                MetricsReport {
                    max_size,
                    min_size,
                    max_time,
                    min_time,
                    avg_time,
                    avg_size,
                    total_duration,
                    total_size,
                },
            )
        })
        .collect()
}
