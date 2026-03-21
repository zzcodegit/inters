#[path = "helpers/mod.rs"]
mod helpers;
#[path = "baseline/support.rs"]
mod support;

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use anyhow::{anyhow, bail, Context};
use serde::Serialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

use support::{BaselineMode, RemoteBaselineConfig};

#[derive(Debug, Clone)]
struct PerfMatrixConfig {
    remote: RemoteBaselineConfig,
    direct_addr: SocketAddr,
    request_path: String,
    runs: usize,
    raw_output_path: PathBuf,
    report_output_path: PathBuf,
    connect_timeout: Duration,
    write_timeout: Duration,
    read_timeout: Duration,
}

#[derive(Debug, Clone)]
struct ScenarioSpec {
    name: &'static str,
    host_header: &'static str,
    route_length: Option<u8>,
    local_listen: Option<SocketAddr>,
    route_cache_path: Option<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
struct PerfRecord {
    scenario: String,
    run_index: usize,
    route_length: u8,
    route_chain: String,
    endpoint: String,
    request_path: String,
    status_code: Option<u16>,
    connect_time_ms: Option<f64>,
    ttfb_ms: Option<f64>,
    total_time_ms: Option<f64>,
    response_bytes: Option<usize>,
    body_bytes: Option<usize>,
    effective_throughput_bps: Option<f64>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
struct ScenarioSummary {
    name: String,
    route_length: u8,
    route_chain: String,
    endpoint: String,
    success_count: usize,
    error_count: usize,
    response_bytes_avg: Option<f64>,
    connect_min_ms: Option<f64>,
    connect_avg_ms: Option<f64>,
    connect_max_ms: Option<f64>,
    ttfb_min_ms: Option<f64>,
    ttfb_avg_ms: Option<f64>,
    ttfb_max_ms: Option<f64>,
    total_min_ms: Option<f64>,
    total_avg_ms: Option<f64>,
    total_max_ms: Option<f64>,
    throughput_avg_bps: Option<f64>,
}

#[tokio::test]
async fn remote_perf_matrix_collects_measurements() -> anyhow::Result<()> {
    support::prepare_baseline(BaselineMode::Remote);

    let config = PerfMatrixConfig::from_env()?;
    prepare_output_path(&config.raw_output_path)?;
    prepare_output_path(&config.report_output_path)?;
    fs::write(&config.raw_output_path, b"").context("truncate raw output file")?;

    let scenarios = vec![
        ScenarioSpec {
            name: "direct",
            host_header: "perf-direct",
            route_length: None,
            local_listen: None,
            route_cache_path: None,
        },
        ScenarioSpec {
            name: "remote-1hop",
            host_header: "perf-1hop",
            route_length: Some(1),
            local_listen: Some("127.0.0.1:19181".parse().unwrap()),
            route_cache_path: Some("route_cache_perf_1hop.json"),
        },
        ScenarioSpec {
            name: "remote-2hop",
            host_header: "perf-2hop",
            route_length: Some(2),
            local_listen: Some("127.0.0.1:19182".parse().unwrap()),
            route_cache_path: Some("route_cache_perf_2hop.json"),
        },
        ScenarioSpec {
            name: "remote-3hop",
            host_header: "perf-3hop",
            route_length: Some(3),
            local_listen: Some("127.0.0.1:19183".parse().unwrap()),
            route_cache_path: Some("route_cache_perf_3hop.json"),
        },
    ];

    let mut all_records = Vec::new();
    let mut summaries = Vec::new();

    for scenario in scenarios {
        let started_at = Instant::now();
        let request = support::http_get_request_path(scenario.host_header, &config.request_path);
        let summary = if let Some(route_length) = scenario.route_length {
            run_reset_if_configured()?;
            let remote_config = config.remote.for_scenario(
                route_length,
                scenario.local_listen.unwrap(),
                scenario.route_cache_path.unwrap(),
                scenario.host_header.to_string(),
            )?;
            let route_chain = remote_config.route_chain();
            let endpoint = remote_config.local_listen.to_string();
            eprintln!(
                "perf_scenario={} type=remote route_length={} route_chain={} endpoint={} request_path={} runs={}",
                scenario.name,
                route_length,
                route_chain,
                endpoint,
                config.request_path,
                config.runs
            );

            let mut client = support::spawn_remote_client(&remote_config)?;
            let result = run_remote_perf_scenario(
                &config,
                &remote_config,
                scenario.name,
                &route_chain,
                &request,
            )
            .await;
            client.stop();
            result?
        } else {
            let route_chain = format!("client -> {} -> target", config.direct_addr);
            eprintln!(
                "perf_scenario={} type=direct route_chain={} endpoint={} request_path={} runs={}",
                scenario.name, route_chain, config.direct_addr, config.request_path, config.runs
            );
            run_direct_perf_scenario(&config, scenario.name, &route_chain, &request).await?
        };

        for record in &summary.1 {
            append_record(&config.raw_output_path, record)?;
        }

        eprintln!(
            "perf_scenario={} success_count={} error_count={} duration_ms={}",
            scenario.name,
            summary.0.success_count,
            summary.0.error_count,
            started_at.elapsed().as_millis()
        );

        all_records.extend(summary.1);
        summaries.push(summary.0);
    }

    let report = render_report(&config, &summaries);
    fs::write(&config.report_output_path, report)
        .with_context(|| format!("write report {}", config.report_output_path.display()))?;

    let total_errors: usize = all_records
        .iter()
        .filter(|record| record.error.is_some())
        .count();
    if total_errors > 0 {
        bail!("remote perf matrix recorded {total_errors} failed measurements");
    }

    Ok(())
}

async fn run_direct_perf_scenario(
    config: &PerfMatrixConfig,
    scenario_name: &str,
    route_chain: &str,
    request: &[u8],
) -> anyhow::Result<(ScenarioSummary, Vec<PerfRecord>)> {
    helpers::wait_http_status_with_request_options(
        config.direct_addr,
        request,
        &[200],
        helpers::HttpProbeOptions {
            ready_timeout: Duration::from_secs(30),
            connect_timeout: config.connect_timeout,
            probe_io_timeout: config.read_timeout,
            probe_attempt_timeout: config.read_timeout,
            retry_delay: Duration::from_millis(250),
        },
    )
    .await
    .with_context(|| format!("direct path {} did not become ready", config.direct_addr))?;

    let mut records = Vec::with_capacity(config.runs);
    for run_index in 1..=config.runs {
        let record = measure_http_request(
            scenario_name,
            0,
            route_chain,
            &config.direct_addr.to_string(),
            &config.request_path,
            config.direct_addr,
            request,
            config,
            run_index,
        )
        .await;
        records.push(record);
    }

    let summary = summarize_records(
        scenario_name,
        0,
        route_chain,
        &config.direct_addr.to_string(),
        &records,
    );
    Ok((summary, records))
}

async fn run_remote_perf_scenario(
    config: &PerfMatrixConfig,
    remote_config: &RemoteBaselineConfig,
    scenario_name: &str,
    route_chain: &str,
    request: &[u8],
) -> anyhow::Result<(ScenarioSummary, Vec<PerfRecord>)> {
    helpers::wait_tcp_listener(remote_config.local_listen)
        .await
        .with_context(|| format!("{scenario_name}: local client TCP listener must come up"))?;
    helpers::wait_http_status_with_request_options(
        remote_config.local_listen,
        request,
        &[remote_config.expected_ready_status],
        helpers::HttpProbeOptions {
            ready_timeout: Duration::from_secs(remote_config.ready_timeout_secs),
            connect_timeout: config.connect_timeout,
            probe_io_timeout: Duration::from_secs(remote_config.probe_attempt_timeout_secs),
            probe_attempt_timeout: Duration::from_secs(remote_config.probe_attempt_timeout_secs),
            retry_delay: Duration::from_millis(500),
        },
    )
    .await
    .with_context(|| format!("{scenario_name}: remote path was not ready via {route_chain}"))?;

    let mut records = Vec::with_capacity(config.runs);
    for run_index in 1..=config.runs {
        let record = measure_http_request(
            scenario_name,
            remote_config.route_length,
            route_chain,
            &remote_config.local_listen.to_string(),
            &config.request_path,
            remote_config.local_listen,
            request,
            config,
            run_index,
        )
        .await;
        records.push(record);
    }

    let summary = summarize_records(
        scenario_name,
        remote_config.route_length,
        route_chain,
        &remote_config.local_listen.to_string(),
        &records,
    );
    Ok((summary, records))
}

async fn measure_http_request(
    scenario_name: &str,
    route_length: u8,
    route_chain: &str,
    endpoint: &str,
    request_path: &str,
    addr: SocketAddr,
    request: &[u8],
    config: &PerfMatrixConfig,
    run_index: usize,
) -> PerfRecord {
    match do_measure_http_request(addr, request, config).await {
        Ok(result) => PerfRecord {
            scenario: scenario_name.to_string(),
            run_index,
            route_length,
            route_chain: route_chain.to_string(),
            endpoint: endpoint.to_string(),
            request_path: request_path.to_string(),
            status_code: Some(result.status_code),
            connect_time_ms: Some(result.connect_time_ms),
            ttfb_ms: Some(result.ttfb_ms),
            total_time_ms: Some(result.total_time_ms),
            response_bytes: Some(result.response_bytes),
            body_bytes: Some(result.body_bytes),
            effective_throughput_bps: Some(result.effective_throughput_bps),
            error: None,
        },
        Err(err) => PerfRecord {
            scenario: scenario_name.to_string(),
            run_index,
            route_length,
            route_chain: route_chain.to_string(),
            endpoint: endpoint.to_string(),
            request_path: request_path.to_string(),
            status_code: None,
            connect_time_ms: None,
            ttfb_ms: None,
            total_time_ms: None,
            response_bytes: None,
            body_bytes: None,
            effective_throughput_bps: None,
            error: Some(err.to_string()),
        },
    }
}

struct MeasureResult {
    status_code: u16,
    connect_time_ms: f64,
    ttfb_ms: f64,
    total_time_ms: f64,
    response_bytes: usize,
    body_bytes: usize,
    effective_throughput_bps: f64,
}

async fn do_measure_http_request(
    addr: SocketAddr,
    request: &[u8],
    config: &PerfMatrixConfig,
) -> anyhow::Result<MeasureResult> {
    let overall_start = Instant::now();
    let connect_start = Instant::now();
    let stream = timeout(config.connect_timeout, TcpStream::connect(addr))
        .await
        .context("TCP connect timed out")?
        .with_context(|| format!("TCP connect failed to {addr}"))?;
    let connect_time_ms = connect_start.elapsed().as_secs_f64() * 1000.0;

    let mut stream = stream;
    timeout(config.write_timeout, stream.write_all(request))
        .await
        .context("write_all timed out")?
        .context("write_all failed")?;
    timeout(config.write_timeout, stream.flush())
        .await
        .context("flush timed out")?
        .context("flush failed")?;

    let request_flushed_at = Instant::now();
    let mut response = Vec::new();
    let mut first_byte_seen_at = None;
    let mut buf = [0_u8; 8192];

    loop {
        let read = timeout(config.read_timeout, stream.read(&mut buf))
            .await
            .context("read timed out")?
            .context("read failed")?;
        if read == 0 {
            break;
        }
        if first_byte_seen_at.is_none() {
            first_byte_seen_at = Some(request_flushed_at.elapsed().as_secs_f64() * 1000.0);
        }
        response.extend_from_slice(&buf[..read]);
    }

    let status_code = helpers::extract_http_status_code(&response)
        .ok_or_else(|| anyhow!("response did not contain a parseable HTTP status line"))?;
    if status_code != 200 {
        bail!("expected HTTP 200, got {status_code}");
    }

    let response_bytes = response.len();
    let body_bytes = extract_http_body(&response).len();
    let ttfb_ms = first_byte_seen_at.unwrap_or(0.0);
    let total_time_ms = overall_start.elapsed().as_secs_f64() * 1000.0;
    let effective_throughput_bps = body_bytes as f64 / (total_time_ms / 1000.0);

    Ok(MeasureResult {
        status_code,
        connect_time_ms,
        ttfb_ms,
        total_time_ms,
        response_bytes,
        body_bytes,
        effective_throughput_bps,
    })
}

fn extract_http_body(response: &[u8]) -> &[u8] {
    let marker = b"\r\n\r\n";
    if let Some(pos) = response
        .windows(marker.len())
        .position(|window| window == marker)
    {
        &response[(pos + marker.len())..]
    } else {
        &[]
    }
}

fn summarize_records(
    scenario_name: &str,
    route_length: u8,
    route_chain: &str,
    endpoint: &str,
    records: &[PerfRecord],
) -> ScenarioSummary {
    let successful = records
        .iter()
        .filter(|record| record.error.is_none())
        .collect::<Vec<_>>();
    let connect = successful
        .iter()
        .filter_map(|record| record.connect_time_ms)
        .collect::<Vec<_>>();
    let ttfb = successful
        .iter()
        .filter_map(|record| record.ttfb_ms)
        .collect::<Vec<_>>();
    let total = successful
        .iter()
        .filter_map(|record| record.total_time_ms)
        .collect::<Vec<_>>();
    let throughput = successful
        .iter()
        .filter_map(|record| record.effective_throughput_bps)
        .collect::<Vec<_>>();
    let response_bytes = successful
        .iter()
        .filter_map(|record| record.response_bytes.map(|bytes| bytes as f64))
        .collect::<Vec<_>>();

    let (connect_min_ms, connect_avg_ms, connect_max_ms) = min_avg_max(&connect);
    let (ttfb_min_ms, ttfb_avg_ms, ttfb_max_ms) = min_avg_max(&ttfb);
    let (total_min_ms, total_avg_ms, total_max_ms) = min_avg_max(&total);
    let (_, throughput_avg_bps, _) = min_avg_max(&throughput);
    let (_, response_bytes_avg, _) = min_avg_max(&response_bytes);

    ScenarioSummary {
        name: scenario_name.to_string(),
        route_length,
        route_chain: route_chain.to_string(),
        endpoint: endpoint.to_string(),
        success_count: successful.len(),
        error_count: records.len().saturating_sub(successful.len()),
        response_bytes_avg,
        connect_min_ms,
        connect_avg_ms,
        connect_max_ms,
        ttfb_min_ms,
        ttfb_avg_ms,
        ttfb_max_ms,
        total_min_ms,
        total_avg_ms,
        total_max_ms,
        throughput_avg_bps,
    }
}

fn min_avg_max(values: &[f64]) -> (Option<f64>, Option<f64>, Option<f64>) {
    if values.is_empty() {
        return (None, None, None);
    }
    let min = values
        .iter()
        .copied()
        .fold(f64::INFINITY, |acc, value| acc.min(value));
    let max = values
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, |acc, value| acc.max(value));
    let avg = values.iter().sum::<f64>() / values.len() as f64;
    (Some(min), Some(avg), Some(max))
}

fn append_record(path: &Path, record: &PerfRecord) -> anyhow::Result<()> {
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .with_context(|| format!("open raw output {}", path.display()))?;
    serde_json::to_writer(&mut file, record).context("serialize perf record")?;
    file.write_all(b"\n").context("append newline")?;
    Ok(())
}

fn prepare_output_path(path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create artifact directory {}", parent.display()))?;
    }
    Ok(())
}

fn run_reset_if_configured() -> anyhow::Result<()> {
    let raw = match std::env::var("VPNNODE_BASELINE_REMOTE_RESET_CMD") {
        Ok(value) if !value.trim().is_empty() => value,
        _ => return Ok(()),
    };

    #[cfg(windows)]
    let status = Command::new("cmd")
        .args(["/C", &raw])
        .status()
        .with_context(|| format!("run reset command: {raw}"))?;

    #[cfg(not(windows))]
    let status = Command::new("sh")
        .args(["-lc", &raw])
        .status()
        .with_context(|| format!("run reset command: {raw}"))?;

    if !status.success() {
        bail!("reset command failed with status {status}");
    }
    Ok(())
}

impl PerfMatrixConfig {
    fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            remote: RemoteBaselineConfig::from_env()?,
            direct_addr: required_env("VPNNODE_PERF_DIRECT_ADDR")?
                .parse()
                .map_err(|e| anyhow!("failed to parse VPNNODE_PERF_DIRECT_ADDR: {e}"))?,
            request_path: env_or_default("VPNNODE_PERF_REQUEST_PATH", "/perf-262144.bin"),
            runs: parse_env_or_default("VPNNODE_PERF_RUNS", 5_usize)?,
            raw_output_path: PathBuf::from(env_or_default(
                "VPNNODE_PERF_RAW_PATH",
                "docs/artifacts/remote_perf_matrix_2026-03-21.jsonl",
            )),
            report_output_path: PathBuf::from(env_or_default(
                "VPNNODE_PERF_REPORT_PATH",
                "docs/artifacts/remote_perf_matrix_2026-03-21.md",
            )),
            connect_timeout: Duration::from_secs(parse_env_or_default(
                "VPNNODE_PERF_CONNECT_TIMEOUT_SECS",
                5_u64,
            )?),
            write_timeout: Duration::from_secs(parse_env_or_default(
                "VPNNODE_PERF_WRITE_TIMEOUT_SECS",
                5_u64,
            )?),
            read_timeout: Duration::from_secs(parse_env_or_default(
                "VPNNODE_PERF_READ_TIMEOUT_SECS",
                60_u64,
            )?),
        })
    }
}

fn required_env(name: &str) -> anyhow::Result<String> {
    std::env::var(name).with_context(|| format!("{name} must be set"))
}

fn env_or_default(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_string())
}

fn parse_env_or_default<T>(name: &str, default: T) -> anyhow::Result<T>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match std::env::var(name) {
        Ok(raw) => raw
            .parse::<T>()
            .map_err(|e| anyhow!("failed to parse {name}={raw:?}: {e}")),
        Err(_) => Ok(default),
    }
}

fn fmt_ms(value: Option<f64>) -> String {
    value
        .map(|number| format!("{number:.2}"))
        .unwrap_or_else(|| "-".to_string())
}

fn fmt_bps(value: Option<f64>) -> String {
    value
        .map(|number| format!("{number:.0}"))
        .unwrap_or_else(|| "-".to_string())
}

fn fmt_count(value: Option<f64>) -> String {
    value
        .map(|number| format!("{number:.0}"))
        .unwrap_or_else(|| "-".to_string())
}

fn render_report(config: &PerfMatrixConfig, summaries: &[ScenarioSummary]) -> String {
    let direct = summaries.iter().find(|summary| summary.name == "direct");
    let mut markdown = String::new();
    markdown.push_str("# Remote WAN performance matrix\n\n");
    markdown.push_str("## Setup\n\n");
    markdown.push_str(&format!(
        "- payload path: `{}`\n- direct public path: `{}`\n- remote exit path: `{} -> target`\n- runs per scenario: `{}`\n- exact route only: `{}`\n\n",
        config.request_path,
        config.direct_addr,
        config.remote.exit_addr,
        config.runs,
        config.remote.exact_route_only
    ));
    markdown.push_str("## Summary\n\n");
    markdown.push_str("| scenario | route | success | errors | resp bytes avg | connect ms min/avg/max | TTFB ms min/avg/max | total ms min/avg/max | avg throughput Bps |\n");
    markdown.push_str("| --- | --- | --- | --- | --- | --- | --- | --- | --- |\n");
    for summary in summaries {
        markdown.push_str(&format!(
            "| {} | {} | {} | {} | {} | {}/{}/{} | {}/{}/{} | {}/{}/{} | {} |\n",
            summary.name,
            summary.route_length,
            summary.success_count,
            summary.error_count,
            fmt_count(summary.response_bytes_avg),
            fmt_ms(summary.connect_min_ms),
            fmt_ms(summary.connect_avg_ms),
            fmt_ms(summary.connect_max_ms),
            fmt_ms(summary.ttfb_min_ms),
            fmt_ms(summary.ttfb_avg_ms),
            fmt_ms(summary.ttfb_max_ms),
            fmt_ms(summary.total_min_ms),
            fmt_ms(summary.total_avg_ms),
            fmt_ms(summary.total_max_ms),
            fmt_bps(summary.throughput_avg_bps),
        ));
    }

    markdown.push_str("\n## Degradation vs direct\n\n");
    markdown.push_str("| scenario | avg connect delta ms | avg connect delta % | avg TTFB delta ms | avg TTFB delta % | avg total delta ms | avg total delta % | avg throughput delta Bps | avg throughput delta % |\n");
    markdown.push_str("| --- | --- | --- | --- | --- | --- | --- | --- | --- |\n");
    for summary in summaries.iter().filter(|summary| summary.name != "direct") {
        markdown.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            summary.name,
            delta_ms(
                summary.connect_avg_ms,
                direct.and_then(|item| item.connect_avg_ms)
            ),
            delta_pct(
                summary.connect_avg_ms,
                direct.and_then(|item| item.connect_avg_ms)
            ),
            delta_ms(
                summary.ttfb_avg_ms,
                direct.and_then(|item| item.ttfb_avg_ms)
            ),
            delta_pct(
                summary.ttfb_avg_ms,
                direct.and_then(|item| item.ttfb_avg_ms)
            ),
            delta_ms(
                summary.total_avg_ms,
                direct.and_then(|item| item.total_avg_ms)
            ),
            delta_pct(
                summary.total_avg_ms,
                direct.and_then(|item| item.total_avg_ms)
            ),
            delta_bps(
                summary.throughput_avg_bps,
                direct.and_then(|item| item.throughput_avg_bps)
            ),
            delta_pct(
                summary.throughput_avg_bps,
                direct.and_then(|item| item.throughput_avg_bps)
            ),
        ));
    }

    markdown.push_str("\n## Topology proof\n\n");
    for summary in summaries {
        markdown.push_str(&format!(
            "- `{}` endpoint=`{}` route_chain=`{}`\n",
            summary.name, summary.endpoint, summary.route_chain
        ));
    }
    markdown.push_str("\n");
    markdown.push_str("Current relay/exit runtime still keeps effectively single-session state, so the perf runner uses the same explicit reset hook between remote scenarios as the accepted WAN matrix.\n");
    markdown
}

fn delta_ms(current: Option<f64>, baseline: Option<f64>) -> String {
    match (current, baseline) {
        (Some(current), Some(baseline)) => format!("{:.2}", current - baseline),
        _ => "-".to_string(),
    }
}

fn delta_bps(current: Option<f64>, baseline: Option<f64>) -> String {
    match (current, baseline) {
        (Some(current), Some(baseline)) => format!("{:.0}", current - baseline),
        _ => "-".to_string(),
    }
}

fn delta_pct(current: Option<f64>, baseline: Option<f64>) -> String {
    match (current, baseline) {
        (Some(_), Some(baseline)) if baseline.abs() < f64::EPSILON => "-".to_string(),
        (Some(current), Some(baseline)) => format!("{:.2}", ((current / baseline) - 1.0) * 100.0),
        _ => "-".to_string(),
    }
}
