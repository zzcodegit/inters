#[path = "helpers/mod.rs"]
mod helpers;
#[path = "baseline/support.rs"]
mod support;

use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use anyhow::{anyhow, bail, Context};
use serde::Serialize;
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

use support::{BaselineMode, RemoteBaselineConfig};

#[derive(Debug, Clone)]
struct RouteQualityConfig {
    remote: RemoteBaselineConfig,
    request_path: String,
    runs: usize,
    raw_output_path: PathBuf,
    stage_trace_path: PathBuf,
    report_output_path: PathBuf,
    local_listen: SocketAddr,
    route_cache_path: String,
    request_host: String,
    probe_host: String,
    connect_timeout: Duration,
    write_timeout: Duration,
    read_timeout: Duration,
}

#[derive(Debug, Clone, Serialize)]
struct RouteQualityMeasurement {
    run_index: usize,
    status_code: u16,
    connect_time_ms: f64,
    ttfb_ms: f64,
    total_time_ms: f64,
    response_bytes: usize,
    body_bytes: usize,
    effective_throughput_bps: f64,
}

#[derive(Debug, Clone)]
struct CandidateScore {
    route_len: u64,
    route_chain: String,
    final_score: f64,
    quality_confidence: Option<f64>,
    warmup_confidence: Option<f64>,
    instability_factor: Option<f64>,
    metric_instability_ppm: Option<u64>,
    flap_penalty_factor: Option<f64>,
    recent_flap_count: Option<u64>,
    recent_ttfb_ms: Option<u64>,
    recent_total_ms: Option<u64>,
    recent_ack_p95_ms: Option<u64>,
    recent_retransmit_rate_ppm: Option<u64>,
    recent_window_wait_ratio_ppm: Option<u64>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct RouteSelection {
    stream_id: u64,
    attempt: u64,
    route_len: u64,
    route_chain: String,
    score: f64,
    decision_reason: String,
    tie_break_reason: Option<String>,
    switched: bool,
    best_idx: Option<u64>,
    previous_idx: Option<u64>,
    score_delta_abs: Option<f64>,
    score_delta_ratio: Option<f64>,
    required_abs_margin: Option<f64>,
    required_rel_margin: Option<f64>,
    hold_remaining_ms: Option<u64>,
    selected_quality_confidence: Option<f64>,
    best_quality_confidence: Option<f64>,
    previous_quality_confidence: Option<f64>,
    selected_metric_instability_ppm: Option<u64>,
    best_metric_instability_ppm: Option<u64>,
    previous_metric_instability_ppm: Option<u64>,
    recent_ttfb_ms: Option<u64>,
    recent_total_ms: Option<u64>,
    recent_ack_p95_ms: Option<u64>,
    recent_retransmit_rate_ppm: Option<u64>,
    recent_window_wait_ratio_ppm: Option<u64>,
}

#[derive(Debug, Clone)]
struct RouteQualityFeedbackEvent {
    route_len: u64,
    route_chain: String,
    ack_latency_ms_p95: Option<u64>,
    retransmit_rate_ppm: Option<u64>,
    window_wait_total_ms: Option<u64>,
    stream_duration_ms: Option<u64>,
    http_code: Option<u64>,
}

#[derive(Debug)]
struct MeasureResult {
    status_code: u16,
    connect_time_ms: f64,
    ttfb_ms: f64,
    total_time_ms: f64,
    response_bytes: usize,
    body_bytes: usize,
    effective_throughput_bps: f64,
}

#[tokio::test]
async fn remote_route_quality_selection_uses_quality_feedback() -> anyhow::Result<()> {
    support::prepare_baseline(BaselineMode::Remote);

    let config = RouteQualityConfig::from_env()?;
    prepare_output_path(&config.raw_output_path)?;
    prepare_output_path(&config.stage_trace_path)?;
    prepare_output_path(&config.report_output_path)?;
    fs::write(&config.raw_output_path, b"").context("truncate raw route-quality output")?;
    fs::write(&config.stage_trace_path, b"").context("truncate stage trace output")?;

    run_reset_if_configured()?;

    let mut remote = config.remote.clone();
    remote.local_listen = config.local_listen;
    remote.route_cache_path = config.route_cache_path.clone();
    remote.http_host = config.request_host.clone();
    remote.exact_route_only = false;

    std::env::set_var(
        "VPNNODE_STAGE_TRACE_PATH",
        config.stage_trace_path.as_os_str(),
    );
    std::env::set_var("VPNNODE_CLIENT_PROBE_PATH", &config.request_path);
    std::env::set_var("VPNNODE_CLIENT_PROBE_HOST", &config.probe_host);

    eprintln!(
        "route_quality mode=remote exact_route_only={} route_length={} local_listen={} request_path={} route_chain={} runs={} raw={} stage_trace={} report={}",
        remote.exact_route_only,
        remote.route_length,
        remote.local_listen,
        config.request_path,
        remote.route_chain(),
        config.runs,
        config.raw_output_path.display(),
        config.stage_trace_path.display(),
        config.report_output_path.display()
    );

    let mut client = support::spawn_remote_client(&remote)?;
    helpers::wait_tcp_listener(remote.local_listen)
        .await
        .context("remote route-quality client listener did not come up")?;

    let ready_request = support::http_get_request_path(&config.request_host, &config.request_path);
    helpers::wait_http_status_with_request_options(
        remote.local_listen,
        &ready_request,
        &[remote.expected_ready_status],
        helpers::HttpProbeOptions {
            ready_timeout: Duration::from_secs(remote.ready_timeout_secs),
            connect_timeout: config.connect_timeout,
            probe_io_timeout: Duration::from_secs(remote.probe_attempt_timeout_secs),
            probe_attempt_timeout: Duration::from_secs(remote.probe_attempt_timeout_secs),
            retry_delay: Duration::from_millis(500),
        },
    )
    .await
    .with_context(|| {
        format!(
            "route-quality path was not ready on {} via {}",
            remote.local_listen,
            remote.route_chain()
        )
    })?;

    let mut measurements = Vec::with_capacity(config.runs);
    for run_index in 1..=config.runs {
        let request_host = format!("{}-run{run_index}", config.request_host);
        let request = support::http_get_request_path(&request_host, &config.request_path);
        let result = do_measure_http_request(remote.local_listen, &request, &config).await?;
        let measurement = RouteQualityMeasurement {
            run_index,
            status_code: result.status_code,
            connect_time_ms: result.connect_time_ms,
            ttfb_ms: result.ttfb_ms,
            total_time_ms: result.total_time_ms,
            response_bytes: result.response_bytes,
            body_bytes: result.body_bytes,
            effective_throughput_bps: result.effective_throughput_bps,
        };
        append_record(&config.raw_output_path, &measurement)?;
        eprintln!(
            "route_quality_run={} status={} connect_ms={:.2} ttfb_ms={:.2} total_ms={:.2} body_bytes={} throughput_bps={:.2}",
            run_index,
            measurement.status_code,
            measurement.connect_time_ms,
            measurement.ttfb_ms,
            measurement.total_time_ms,
            measurement.body_bytes,
            measurement.effective_throughput_bps
        );
        measurements.push(measurement);
    }

    client.stop();

    let analysis = analyze_stage_trace(&config.stage_trace_path)?;
    let report = render_report(&config, &measurements, &analysis);
    fs::write(&config.report_output_path, report).with_context(|| {
        format!(
            "write route-quality report {}",
            config.report_output_path.display()
        )
    })?;

    if analysis.multi_candidate_events == 0 {
        bail!("route-quality trace did not capture any multi-candidate selection events");
    }
    if analysis.quality_enriched_selections == 0 {
        bail!("route-quality trace never selected a route with quality metrics attached");
    }
    if analysis.feedback_events.is_empty() {
        bail!("route-quality trace did not capture any response-path feedback events");
    }

    Ok(())
}

fn prepare_output_path(path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create artifact directory {}", parent.display()))?;
    }
    Ok(())
}

fn append_record(path: &Path, record: &RouteQualityMeasurement) -> anyhow::Result<()> {
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .with_context(|| format!("open route-quality raw output {}", path.display()))?;
    serde_json::to_writer(&mut file, record).context("serialize route-quality measurement")?;
    file.write_all(b"\n").context("append newline")?;
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

async fn do_measure_http_request(
    addr: SocketAddr,
    request: &[u8],
    config: &RouteQualityConfig,
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
    let mut buf = [0u8; 8192];
    let mut first_byte_time_ms = None;

    loop {
        let read = timeout(config.read_timeout, stream.read(&mut buf))
            .await
            .context("read timed out")?
            .with_context(|| format!("read failed from {addr}"))?;
        if read == 0 {
            break;
        }
        if first_byte_time_ms.is_none() {
            first_byte_time_ms = Some(request_flushed_at.elapsed().as_secs_f64() * 1000.0);
        }
        response.extend_from_slice(&buf[..read]);
    }

    let total_time_ms = overall_start.elapsed().as_secs_f64() * 1000.0;
    let ttfb_ms = first_byte_time_ms.unwrap_or(total_time_ms);
    let status_code = helpers::extract_http_status_code(&response)
        .ok_or_else(|| anyhow!("response did not contain a parseable HTTP status"))?;
    if status_code != 200 {
        bail!("expected HTTP 200 from route-quality run, got {status_code}");
    }

    let body_bytes = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|idx| response.len().saturating_sub(idx + 4))
        .unwrap_or(0);
    let effective_throughput_bps = if total_time_ms > 0.0 {
        body_bytes as f64 / (total_time_ms / 1000.0)
    } else {
        0.0
    };

    Ok(MeasureResult {
        status_code,
        connect_time_ms,
        ttfb_ms,
        total_time_ms,
        response_bytes: response.len(),
        body_bytes,
        effective_throughput_bps,
    })
}

#[derive(Debug, Default)]
struct StageTraceAnalysis {
    candidate_snapshots: HashMap<(u64, u64), Vec<CandidateScore>>,
    selections: Vec<RouteSelection>,
    feedback_events: Vec<RouteQualityFeedbackEvent>,
    multi_candidate_events: usize,
    quality_enriched_selections: usize,
    non_shortest_selection_count: usize,
}

fn analyze_stage_trace(path: &Path) -> anyhow::Result<StageTraceAnalysis> {
    let file =
        fs::File::open(path).with_context(|| format!("open stage trace {}", path.display()))?;
    let reader = BufReader::new(file);
    let mut analysis = StageTraceAnalysis::default();

    for line in reader.lines() {
        let line = line.context("read stage trace line")?;
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(&line).context("parse stage trace json")?;
        let Some(stage) = value.get("stage").and_then(Value::as_str) else {
            continue;
        };
        match stage {
            "route_candidates_scored" => {
                let Some(stream_id) = value.get("stream_id").and_then(Value::as_u64) else {
                    continue;
                };
                let Some(attempt) = value.get("attempt").and_then(Value::as_u64) else {
                    continue;
                };
                let candidates = value
                    .get("candidates")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|candidate| CandidateScore {
                        route_len: candidate
                            .get("route_len")
                            .and_then(Value::as_u64)
                            .unwrap_or(0),
                        route_chain: candidate
                            .get("route_chain")
                            .and_then(Value::as_str)
                            .unwrap_or("<missing>")
                            .to_string(),
                        final_score: candidate
                            .get("final_score")
                            .and_then(Value::as_f64)
                            .unwrap_or(0.0),
                        quality_confidence: candidate
                            .get("quality_confidence")
                            .and_then(Value::as_f64),
                        warmup_confidence: candidate
                            .get("warmup_confidence")
                            .and_then(Value::as_f64),
                        instability_factor: candidate
                            .get("instability_factor")
                            .and_then(Value::as_f64),
                        metric_instability_ppm: candidate
                            .get("metric_instability_ppm")
                            .and_then(Value::as_u64),
                        flap_penalty_factor: candidate
                            .get("flap_penalty_factor")
                            .and_then(Value::as_f64),
                        recent_flap_count: candidate
                            .get("recent_flap_count")
                            .and_then(Value::as_u64),
                        recent_ttfb_ms: candidate.get("recent_ttfb_ms").and_then(Value::as_u64),
                        recent_total_ms: candidate.get("recent_total_ms").and_then(Value::as_u64),
                        recent_ack_p95_ms: candidate
                            .get("recent_ack_p95_ms")
                            .and_then(Value::as_u64),
                        recent_retransmit_rate_ppm: candidate
                            .get("recent_retransmit_rate_ppm")
                            .and_then(Value::as_u64),
                        recent_window_wait_ratio_ppm: candidate
                            .get("recent_window_wait_ratio_ppm")
                            .and_then(Value::as_u64),
                    })
                    .collect::<Vec<_>>();
                if candidates.len() >= 2 {
                    analysis.multi_candidate_events =
                        analysis.multi_candidate_events.saturating_add(1);
                }
                analysis
                    .candidate_snapshots
                    .insert((stream_id, attempt), candidates);
            }
            "route_selected" => {
                let selection = RouteSelection {
                    stream_id: value.get("stream_id").and_then(Value::as_u64).unwrap_or(0),
                    attempt: value.get("attempt").and_then(Value::as_u64).unwrap_or(0),
                    route_len: value.get("route_len").and_then(Value::as_u64).unwrap_or(0),
                    route_chain: value
                        .get("route_chain")
                        .and_then(Value::as_str)
                        .unwrap_or("<missing>")
                        .to_string(),
                    score: value.get("score").and_then(Value::as_f64).unwrap_or(0.0),
                    decision_reason: value
                        .get("decision_reason")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown")
                        .to_string(),
                    tie_break_reason: value
                        .get("tie_break_reason")
                        .and_then(Value::as_str)
                        .map(|value| value.to_string()),
                    switched: value.get("switched").and_then(Value::as_bool).unwrap_or(false),
                    best_idx: value.get("best_idx").and_then(Value::as_u64),
                    previous_idx: value.get("previous_idx").and_then(Value::as_u64),
                    score_delta_abs: value.get("score_delta_abs").and_then(Value::as_f64),
                    score_delta_ratio: value.get("score_delta_ratio").and_then(Value::as_f64),
                    required_abs_margin: value.get("required_abs_margin").and_then(Value::as_f64),
                    required_rel_margin: value.get("required_rel_margin").and_then(Value::as_f64),
                    hold_remaining_ms: value.get("hold_remaining_ms").and_then(Value::as_u64),
                    selected_quality_confidence: value
                        .get("selected_quality_confidence")
                        .and_then(Value::as_f64),
                    best_quality_confidence: value
                        .get("best_quality_confidence")
                        .and_then(Value::as_f64),
                    previous_quality_confidence: value
                        .get("previous_quality_confidence")
                        .and_then(Value::as_f64),
                    selected_metric_instability_ppm: value
                        .get("selected_metric_instability_ppm")
                        .and_then(Value::as_u64),
                    best_metric_instability_ppm: value
                        .get("best_metric_instability_ppm")
                        .and_then(Value::as_u64),
                    previous_metric_instability_ppm: value
                        .get("previous_metric_instability_ppm")
                        .and_then(Value::as_u64),
                    recent_ttfb_ms: value.get("recent_ttfb_ms").and_then(Value::as_u64),
                    recent_total_ms: value.get("recent_total_ms").and_then(Value::as_u64),
                    recent_ack_p95_ms: value.get("recent_ack_p95_ms").and_then(Value::as_u64),
                    recent_retransmit_rate_ppm: value
                        .get("recent_retransmit_rate_ppm")
                        .and_then(Value::as_u64),
                    recent_window_wait_ratio_ppm: value
                        .get("recent_window_wait_ratio_ppm")
                        .and_then(Value::as_u64),
                };
                if selection.recent_total_ms.is_some()
                    || selection.recent_ack_p95_ms.is_some()
                    || selection.recent_retransmit_rate_ppm.is_some()
                    || selection.recent_window_wait_ratio_ppm.is_some()
                {
                    analysis.quality_enriched_selections =
                        analysis.quality_enriched_selections.saturating_add(1);
                }
                if let Some(candidates) = analysis
                    .candidate_snapshots
                    .get(&(selection.stream_id, selection.attempt))
                {
                    if let Some(shortest_len) =
                        candidates.iter().map(|candidate| candidate.route_len).min()
                    {
                        if selection.route_len > shortest_len {
                            analysis.non_shortest_selection_count =
                                analysis.non_shortest_selection_count.saturating_add(1);
                        }
                    }
                }
                analysis.selections.push(selection);
            }
            "route_quality_feedback_received" => {
                analysis.feedback_events.push(RouteQualityFeedbackEvent {
                    route_len: value.get("route_len").and_then(Value::as_u64).unwrap_or(0),
                    route_chain: value
                        .get("route_chain")
                        .and_then(Value::as_str)
                        .unwrap_or("<missing>")
                        .to_string(),
                    ack_latency_ms_p95: value.get("ack_latency_ms_p95").and_then(Value::as_u64),
                    retransmit_rate_ppm: value.get("retransmit_rate_ppm").and_then(Value::as_u64),
                    window_wait_total_ms: value.get("window_wait_total_ms").and_then(Value::as_u64),
                    stream_duration_ms: value.get("stream_duration_ms").and_then(Value::as_u64),
                    http_code: value.get("http_code").and_then(Value::as_u64),
                });
            }
            _ => {}
        }
    }

    Ok(analysis)
}

fn render_report(
    config: &RouteQualityConfig,
    measurements: &[RouteQualityMeasurement],
    analysis: &StageTraceAnalysis,
) -> String {
    let avg_total = average(
        measurements
            .iter()
            .map(|measurement| measurement.total_time_ms),
    );
    let avg_ttfb = average(measurements.iter().map(|measurement| measurement.ttfb_ms));
    let avg_throughput = average(
        measurements
            .iter()
            .map(|measurement| measurement.effective_throughput_bps),
    );

    let mut markdown = String::new();
    markdown.push_str("# Remote WAN route-quality scoring\n\n");
    markdown.push_str("## Setup\n\n");
    markdown.push_str(&format!(
        "- exact route only: `false`\n- requested max route length: `{}`\n- candidate topology: `client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`\n- local ingress: `{}`\n- request path: `{}`\n- probe path: `{}`\n- runs: `{}`\n\n",
        config.remote.route_length,
        config.local_listen,
        config.request_path,
        config.request_path,
        config.runs,
    ));
    markdown.push_str("## Measurements\n\n");
    markdown.push_str(
        "| run | status | connect ms | TTFB ms | total ms | body bytes | throughput Bps |\n",
    );
    markdown.push_str("| --- | --- | --- | --- | --- | --- | --- |\n");
    for measurement in measurements {
        markdown.push_str(&format!(
            "| {} | {} | {:.2} | {:.2} | {:.2} | {} | {:.2} |\n",
            measurement.run_index,
            measurement.status_code,
            measurement.connect_time_ms,
            measurement.ttfb_ms,
            measurement.total_time_ms,
            measurement.body_bytes,
            measurement.effective_throughput_bps
        ));
    }
    markdown.push_str(&format!(
        "\nAverages: total `{:.2} ms`, TTFB `{:.2} ms`, throughput `{:.2} Bps`.\n\n",
        avg_total, avg_ttfb, avg_throughput
    ));

    markdown.push_str("## Candidate Scoring Evidence\n\n");
    markdown.push_str(&format!(
        "- multi-candidate score events: `{}`\n- selections with quality metrics attached: `{}`\n- non-shortest selections observed: `{}`\n- feedback events received from exit: `{}`\n\n",
        analysis.multi_candidate_events,
        analysis.quality_enriched_selections,
        analysis.non_shortest_selection_count,
        analysis.feedback_events.len()
    ));

    markdown.push_str("| selection | route len | selected route | score | reason | switched | score delta abs | score delta rel % | q conf sel/best | instability sel/best ppm | recent TTFB ms | recent total ms | recent ACK p95 ms | recent retransmit ppm | recent stall ppm |\n");
    markdown.push_str("| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |\n");
    for (idx, selection) in analysis.selections.iter().take(12).enumerate() {
        markdown.push_str(&format!(
            "| {} | {} | `{}` | {:.4} | `{}` | `{}` | {} | {} | {} / {} | {} / {} | {} | {} | {} | {} | {} |\n",
            idx + 1,
            selection.route_len,
            selection.route_chain,
            selection.score,
            selection.decision_reason,
            selection.switched,
            fmt_opt_f64(selection.score_delta_abs),
            fmt_opt_percent(selection.score_delta_ratio),
            fmt_opt_f64(selection.selected_quality_confidence),
            fmt_opt_f64(selection.best_quality_confidence),
            fmt_opt(selection.selected_metric_instability_ppm),
            fmt_opt(selection.best_metric_instability_ppm),
            fmt_opt(selection.recent_ttfb_ms),
            fmt_opt(selection.recent_total_ms),
            fmt_opt(selection.recent_ack_p95_ms),
            fmt_opt(selection.recent_retransmit_rate_ppm),
            fmt_opt(selection.recent_window_wait_ratio_ppm),
        ));
    }

    if let Some(first_selection) = analysis.selections.first() {
        if let Some(candidates) = analysis
            .candidate_snapshots
            .get(&(first_selection.stream_id, first_selection.attempt))
        {
            markdown.push_str("\nRepresentative candidate snapshot:\n\n");
            markdown.push_str("| route len | candidate route | final score | q conf | warmup | instability ppm / factor | flap penalty | flap count | recent TTFB ms | recent total ms | recent ACK p95 ms | retransmit ppm | stall ppm |\n");
            markdown.push_str("| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |\n");
            for candidate in candidates {
                markdown.push_str(&format!(
                    "| {} | `{}` | {:.4} | {} | {} | {} / {} | {} | {} | {} | {} | {} | {} | {} |\n",
                    candidate.route_len,
                    candidate.route_chain,
                    candidate.final_score,
                    fmt_opt_f64(candidate.quality_confidence),
                    fmt_opt_f64(candidate.warmup_confidence),
                    fmt_opt(candidate.metric_instability_ppm),
                    fmt_opt_f64(candidate.instability_factor),
                    fmt_opt_f64(candidate.flap_penalty_factor),
                    fmt_opt(candidate.recent_flap_count),
                    fmt_opt(candidate.recent_ttfb_ms),
                    fmt_opt(candidate.recent_total_ms),
                    fmt_opt(candidate.recent_ack_p95_ms),
                    fmt_opt(candidate.recent_retransmit_rate_ppm),
                    fmt_opt(candidate.recent_window_wait_ratio_ppm),
                ));
            }
        }
    }

    if !analysis.feedback_events.is_empty() {
        markdown.push_str("\nFeedback events seen from exit:\n\n");
        markdown.push_str("| route len | route | ACK p95 ms | retransmit ppm | window wait total ms | stream duration ms | http code |\n");
        markdown.push_str("| --- | --- | --- | --- | --- | --- | --- |\n");
        for event in analysis.feedback_events.iter().take(12) {
            markdown.push_str(&format!(
                "| {} | `{}` | {} | {} | {} | {} | {} |\n",
                event.route_len,
                event.route_chain,
                fmt_opt(event.ack_latency_ms_p95),
                fmt_opt(event.retransmit_rate_ppm),
                fmt_opt(event.window_wait_total_ms),
                fmt_opt(event.stream_duration_ms),
                fmt_opt(event.http_code),
            ));
        }
    }

    markdown.push_str("\n## Conclusion\n\n");
    markdown.push_str("The client no longer selects purely by availability or shortest path. Each candidate now carries recent end-to-end timing plus exit-side response-path feedback: ACK latency, retransmit rate, and window/backpressure stall. The selected route is the candidate with the best blended score at selection time, and the raw stage trace in the committed artifact shows both the full candidate list and the winning score for each selection event.\n");
    markdown
}

fn average(values: impl Iterator<Item = f64>) -> f64 {
    let mut sum = 0.0;
    let mut count = 0.0;
    for value in values {
        sum += value;
        count += 1.0;
    }
    if count == 0.0 {
        0.0
    } else {
        sum / count
    }
}

fn fmt_opt(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn fmt_opt_f64(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.4}"))
        .unwrap_or_else(|| "-".to_string())
}

fn fmt_opt_percent(value: Option<f64>) -> String {
    value
        .map(|value| format!("{:.2}", value * 100.0))
        .unwrap_or_else(|| "-".to_string())
}

impl RouteQualityConfig {
    fn from_env() -> anyhow::Result<Self> {
        let mut remote = RemoteBaselineConfig::from_env()?;
        remote.route_length = parse_env_or_default("VPNNODE_ROUTE_QUALITY_ROUTE_LENGTH", 3_u8)?;
        if !(1..=3).contains(&remote.route_length) {
            bail!(
                "VPNNODE_ROUTE_QUALITY_ROUTE_LENGTH must be 1, 2, or 3; got {}",
                remote.route_length
            );
        }
        remote.exact_route_only = false;

        Ok(Self {
            remote,
            request_path: env_or_default("VPNNODE_ROUTE_QUALITY_REQUEST_PATH", "/perf-262144.bin"),
            runs: parse_env_or_default("VPNNODE_ROUTE_QUALITY_RUNS", 6_usize)?,
            raw_output_path: PathBuf::from(env_or_default(
                "VPNNODE_ROUTE_QUALITY_RAW_PATH",
                "docs/artifacts/route_quality_remote_2026-03-21.jsonl",
            )),
            stage_trace_path: PathBuf::from(env_or_default(
                "VPNNODE_ROUTE_QUALITY_STAGE_TRACE_PATH",
                "docs/artifacts/route_quality_remote_2026-03-21.client.jsonl",
            )),
            report_output_path: PathBuf::from(env_or_default(
                "VPNNODE_ROUTE_QUALITY_REPORT_PATH",
                "docs/artifacts/route_quality_remote_2026-03-21.md",
            )),
            local_listen: parse_env_or_default(
                "VPNNODE_ROUTE_QUALITY_LOCAL_LISTEN",
                "127.0.0.1:19380"
                    .parse()
                    .expect("default route-quality local listen"),
            )?,
            route_cache_path: env_or_default(
                "VPNNODE_ROUTE_QUALITY_ROUTE_CACHE_PATH",
                "route_cache_route_quality.json",
            ),
            request_host: env_or_default("VPNNODE_ROUTE_QUALITY_HTTP_HOST", "route-quality"),
            probe_host: env_or_default("VPNNODE_ROUTE_QUALITY_PROBE_HOST", "route-quality-probe"),
            connect_timeout: Duration::from_secs(parse_env_or_default(
                "VPNNODE_ROUTE_QUALITY_CONNECT_TIMEOUT_SECS",
                3_u64,
            )?),
            write_timeout: Duration::from_secs(parse_env_or_default(
                "VPNNODE_ROUTE_QUALITY_WRITE_TIMEOUT_SECS",
                10_u64,
            )?),
            read_timeout: Duration::from_secs(parse_env_or_default(
                "VPNNODE_ROUTE_QUALITY_READ_TIMEOUT_SECS",
                60_u64,
            )?),
        })
    }
}

fn parse_env_or_default<T>(key: &str, default: T) -> anyhow::Result<T>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match std::env::var(key) {
        Ok(value) => value
            .parse::<T>()
            .map_err(|err| anyhow!("failed to parse {key}: {err}")),
        Err(_) => Ok(default),
    }
}

fn env_or_default(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}
