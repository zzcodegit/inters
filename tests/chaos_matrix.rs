#[path = "helpers/mod.rs"]
mod helpers;
#[path = "baseline/support.rs"]
mod support;

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{anyhow, bail, Context};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::task::JoinHandle;
use tokio::time::{timeout, Duration};

use support::BaselineMode;
use vpnnode::config::{ClientConfigCli, ExitConfigCli, RelayConfigCli, TargetConfigCli};

#[derive(Debug, Clone, Serialize)]
struct ChaosProfileArtifact {
    profile_name: String,
    seed: u64,
    skip_packets: u64,
    loss_ppm: u32,
    duplicate_ppm: u32,
    reorder_ppm: u32,
    base_delay_ms: u64,
    jitter_ms: u64,
    reorder_extra_delay_ms: u64,
    duplicate_delay_ms: u64,
    runs: usize,
    request_body_bytes: usize,
}

#[derive(Debug, Clone)]
struct ChaosRunConfig {
    artifact_prefix: PathBuf,
    stage_trace_path: PathBuf,
    profile: ChaosProfileArtifact,
    base_port: u16,
    exact_route_cache_path: PathBuf,
    adaptive_route_cache_path: PathBuf,
    connect_timeout: Duration,
    write_timeout: Duration,
    read_timeout: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MeasurementRecord {
    profile_name: String,
    mode: String,
    run_index: usize,
    route_length: u8,
    status_code: u16,
    connect_time_ms: f64,
    ttfb_ms: f64,
    total_time_ms: f64,
    response_bytes: usize,
    body_bytes: usize,
    effective_throughput_bps: f64,
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

struct LocalStackHandles {
    handles: Vec<JoinHandle<()>>,
}

impl LocalStackHandles {
    fn abort_all(self) {
        for handle in self.handles {
            handle.abort();
        }
    }
}

#[derive(Debug, Clone)]
struct ExitStreamSummary {
    site: String,
    route_len: u64,
    stream_duration_ms: Option<u64>,
    first_target_byte_ms: Option<u64>,
    first_overlay_send_ms: Option<u64>,
    avg_inflight: Option<f64>,
    max_inflight: Option<u64>,
    ack_latency_ms_p95: Option<u64>,
    ack_latency_ms_avg: Option<u64>,
    total_retransmits: Option<u64>,
    retransmit_rate: Option<f64>,
    retransmit_rate_ppm: Option<u64>,
    window_wait_total_ms: Option<u64>,
    effective_cap_wait_total_ms: Option<u64>,
    pacing_enabled: Option<bool>,
    pacing_delay_applied: Option<u64>,
    pacing_delay_applied_ms_total: Option<u64>,
    pacing_interval_ms_avg: Option<u64>,
    pacing_interval_ms_max: Option<u64>,
    burst_prevented_count: Option<u64>,
    paced_send_batches: Option<u64>,
    max_send_burst_frames: Option<u64>,
    effective_inflight_cap_avg: Option<u64>,
    effective_inflight_cap_min: Option<u64>,
    effective_inflight_cap_max: Option<u64>,
    inflight_cap_reduced_count: Option<u64>,
    inflight_cap_restore_count: Option<u64>,
    ack_pressure_events: Option<u64>,
    send_blocked_by_effective_cap: Option<u64>,
    http_code: Option<u64>,
}

#[derive(Debug, Default)]
struct StageAnalysis {
    chaos_counts: BTreeMap<String, usize>,
    open_message_failures: BTreeMap<String, usize>,
    client_open_message_failures: BTreeMap<String, usize>,
    duplicate_drop_events: BTreeMap<String, usize>,
    client_duplicate_drop_events: BTreeMap<String, usize>,
    exit_ack_events: BTreeMap<String, usize>,
    client_late_events: BTreeMap<String, usize>,
    client_lifecycle_events: BTreeMap<String, usize>,
    client_terminal_events: BTreeMap<String, usize>,
    exact_streams: Vec<ExitStreamSummary>,
    adaptive_streams: Vec<ExitStreamSummary>,
    adaptive_decisions: Vec<Value>,
}

#[derive(Debug, Clone, Copy)]
enum RunMode {
    Exact3Hop,
    Adaptive,
}

impl RunMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Exact3Hop => "exact-3hop",
            Self::Adaptive => "adaptive",
        }
    }

    fn route_length(self) -> u8 {
        match self {
            Self::Exact3Hop | Self::Adaptive => 3,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct LocalPorts {
    target_tcp: SocketAddr,
    exit_udp: SocketAddr,
    relay1_udp: SocketAddr,
    relay2_udp: SocketAddr,
    client_tcp: SocketAddr,
}

#[tokio::test]
async fn chaos_profile_collects_transport_and_selection_artifacts() -> anyhow::Result<()> {
    support::prepare_baseline(BaselineMode::Local);

    let config = ChaosRunConfig::from_env()?;
    prepare_chaos_run(&config)?;

    let exact_ports = ports_from_base(config.base_port);
    let adaptive_ports = ports_from_base(config.base_port.saturating_add(100));

    run_mode_scenario(
        &config,
        RunMode::Exact3Hop,
        exact_ports,
        true,
        &config.exact_route_cache_path,
    )
    .await?;
    run_mode_scenario(
        &config,
        RunMode::Adaptive,
        adaptive_ports,
        false,
        &config.adaptive_route_cache_path,
    )
    .await?;

    let stage_analysis = analyze_stage_trace(&config.stage_trace_path)?;
    write_decision_trace(
        &config.decision_trace_path(),
        &stage_analysis.adaptive_decisions,
    )?;
    fs::write(
        config.report_path(),
        render_report(&config, &stage_analysis)?,
    )
    .context("write chaos profile report")?;

    if stage_analysis.adaptive_decisions.is_empty() {
        bail!("chaos adaptive run did not emit any multi-candidate route decisions");
    }
    if stage_analysis.exact_streams.is_empty() {
        bail!("chaos exact run did not emit exit stream summaries");
    }
    assert_eq!(
        stage_analysis
            .client_open_message_failures
            .get("duplicate packet detected")
            .copied()
            .unwrap_or(0),
        0,
        "chaos profile must not surface duplicate packets as client open-message failures once duplicate traffic is explicitly classified"
    );
    assert_eq!(
        stage_analysis
            .open_message_failures
            .get("duplicate packet detected")
            .copied()
            .unwrap_or(0),
        0,
        "chaos profile must not surface duplicate packets as exit open-message failures once duplicate traffic is explicitly classified"
    );

    Ok(())
}

#[tokio::test]
async fn combined_chaos_preserves_full_body_against_content_length() -> anyhow::Result<()> {
    support::prepare_baseline(BaselineMode::Local);

    let config = ChaosRunConfig::combined_regression("combined-regression-test", 19_900, 3);
    prepare_chaos_run(&config)?;

    let exact_ports = ports_from_base(config.base_port);
    run_mode_scenario(
        &config,
        RunMode::Exact3Hop,
        exact_ports,
        true,
        &config.exact_route_cache_path,
    )
    .await?;

    let measurements = read_measurements(&config.raw_path())?;
    assert_eq!(
        measurements.len(),
        config.profile.runs,
        "combined chaos regression must produce one successful full-body measurement per run"
    );
    assert!(
        measurements.iter().all(|record| record.status_code == 200),
        "combined chaos regression must keep HTTP 200"
    );
    assert!(
        measurements
            .iter()
            .all(|record| record.response_bytes > record.body_bytes && record.body_bytes > 0),
        "combined chaos regression must preserve a complete response body"
    );

    let stage_analysis = analyze_stage_trace(&config.stage_trace_path)?;
    assert_eq!(
        stage_analysis
            .client_lifecycle_events
            .get("orphaned_response_payload")
            .copied()
            .unwrap_or(0),
        0,
        "combined chaos regression must not leave payloads without a live or completed consumer"
    );
    assert_eq!(
        stage_analysis
            .client_lifecycle_events
            .get("response_payload_unknown_stream")
            .copied()
            .unwrap_or(0),
        0,
        "combined chaos regression must not drop response payloads on unknown streams"
    );

    Ok(())
}

#[tokio::test]
async fn reorder_chaos_does_not_reject_packets_inside_replay_window() -> anyhow::Result<()> {
    support::prepare_baseline(BaselineMode::Local);

    let config =
        ChaosRunConfig::mild_reorder_regression("replay-window-regression-test", 20_200, 5);
    prepare_chaos_run(&config)?;

    let exact_ports = ports_from_base(config.base_port);
    let adaptive_ports = ports_from_base(config.base_port.saturating_add(100));

    run_mode_scenario(
        &config,
        RunMode::Exact3Hop,
        exact_ports,
        true,
        &config.exact_route_cache_path,
    )
    .await?;
    run_mode_scenario(
        &config,
        RunMode::Adaptive,
        adaptive_ports,
        false,
        &config.adaptive_route_cache_path,
    )
    .await?;

    let stage_analysis = analyze_stage_trace(&config.stage_trace_path)?;
    assert_eq!(
        stage_analysis
            .client_open_message_failures
            .get("packet too old for replay window")
            .copied()
            .unwrap_or(0),
        0,
        "mild reorder chaos must not reject client packets as too old once they are still inside the expanded replay window"
    );
    assert_eq!(
        stage_analysis
            .open_message_failures
            .get("packet too old for replay window")
            .copied()
            .unwrap_or(0),
        0,
        "mild reorder chaos must not reject exit packets as too old once they are still inside the expanded replay window"
    );

    let measurements = read_measurements(&config.raw_path())?;
    assert_eq!(
        measurements.len(),
        config.profile.runs * 2,
        "reorder replay regression must produce successful exact and adaptive measurements"
    );
    assert!(
        measurements.iter().all(|record| record.status_code == 200),
        "reorder replay regression must keep HTTP 200"
    );

    Ok(())
}

#[tokio::test]
async fn mild_delay_content_length_completion_settles_terminal_signals_cleanly(
) -> anyhow::Result<()> {
    support::prepare_baseline(BaselineMode::Local);

    let config = ChaosRunConfig::mild_delay_regression("delay-tail-regression-test", 20_500, 5);
    prepare_chaos_run(&config)?;

    let exact_ports = ports_from_base(config.base_port);
    run_mode_scenario(
        &config,
        RunMode::Exact3Hop,
        exact_ports,
        true,
        &config.exact_route_cache_path,
    )
    .await?;

    let measurements = read_measurements(&config.raw_path())?;
    assert_eq!(
        measurements.len(),
        config.profile.runs,
        "delay-tail regression must produce one successful exact-route measurement per run"
    );
    assert!(
        measurements.iter().all(|record| record.status_code == 200),
        "delay-tail regression must keep HTTP 200"
    );

    let stage_analysis = analyze_stage_trace(&config.stage_trace_path)?;
    assert_eq!(
        stage_analysis
            .client_late_events
            .get("late_payload_after_completion")
            .copied()
            .unwrap_or(0),
        0,
        "mild delay regression must not classify delayed terminal payload markers as late payload after completion"
    );
    assert_eq!(
        stage_analysis
            .client_late_events
            .get("late_close_after_completion")
            .copied()
            .unwrap_or(0),
        0,
        "mild delay regression must not classify delayed CloseStream markers as late close after completion"
    );
    assert!(
        stage_analysis
            .client_terminal_events
            .get("response_transport_settlement_started")
            .copied()
            .unwrap_or(0)
            > 0,
        "mild delay regression must enter transport-settlement after local content-length completion"
    );
    assert!(
        stage_analysis
            .client_terminal_events
            .get("response_transport_terminal_payload_observed")
            .copied()
            .unwrap_or(0)
            > 0,
        "mild delay regression must prove terminal payload markers were still observed during settlement"
    );
    assert!(
        stage_analysis
            .client_terminal_events
            .get("response_transport_terminal_close_observed")
            .copied()
            .unwrap_or(0)
            > 0,
        "mild delay regression must prove CloseStream markers were still observed during settlement"
    );
    assert!(
        stage_analysis
            .client_terminal_events
            .get("response_transport_settlement_completed")
            .copied()
            .unwrap_or(0)
            > 0,
        "mild delay regression must complete transport settlement before cleanup"
    );
    assert_eq!(
        stage_analysis
            .client_terminal_events
            .get("duplicate_payload_after_local_completion")
            .copied()
            .unwrap_or(0),
        0,
        "mild delay regression must not leave duplicate payload tail in the old post-completion anomaly bucket"
    );
    assert_eq!(
        stage_analysis
            .client_terminal_events
            .get("duplicate_payload_after_local_completion_repeat")
            .copied()
            .unwrap_or(0),
        0,
        "mild delay regression must not leave repeated duplicate payload tail in the old post-completion anomaly bucket"
    );
    assert_eq!(
        stage_analysis
            .client_late_events
            .get("payload_after_local_completion_during_settlement")
            .copied()
            .unwrap_or(0),
        0,
        "mild delay regression must not deliver extra body bytes after local content-length completion"
    );

    Ok(())
}

#[tokio::test]
async fn combined_chaos_duplicate_packets_are_classified_without_open_failure() -> anyhow::Result<()>
{
    support::prepare_baseline(BaselineMode::Local);

    let config = ChaosRunConfig::combined_regression("duplicate-noise-regression-test", 20_800, 5);
    prepare_chaos_run(&config)?;

    let exact_ports = ports_from_base(config.base_port);
    let adaptive_ports = ports_from_base(config.base_port.saturating_add(100));

    run_mode_scenario(
        &config,
        RunMode::Exact3Hop,
        exact_ports,
        true,
        &config.exact_route_cache_path,
    )
    .await?;
    run_mode_scenario(
        &config,
        RunMode::Adaptive,
        adaptive_ports,
        false,
        &config.adaptive_route_cache_path,
    )
    .await?;

    let measurements = read_measurements(&config.raw_path())?;
    assert_eq!(
        measurements.len(),
        config.profile.runs * 2,
        "duplicate-noise regression must produce successful exact and adaptive measurements"
    );
    assert!(
        measurements.iter().all(|record| record.status_code == 200),
        "duplicate-noise regression must keep HTTP 200"
    );

    let stage_analysis = analyze_stage_trace(&config.stage_trace_path)?;
    assert_eq!(
        stage_analysis
            .client_open_message_failures
            .get("duplicate packet detected")
            .copied()
            .unwrap_or(0),
        0,
        "duplicate-noise regression must no longer classify duplicate packets as client open-message failures"
    );
    assert_eq!(
        stage_analysis
            .open_message_failures
            .get("duplicate packet detected")
            .copied()
            .unwrap_or(0),
        0,
        "duplicate-noise regression must no longer classify duplicate packets as exit open-message failures"
    );
    assert!(
        stage_analysis
            .client_duplicate_drop_events
            .get("duplicate_packet_dropped")
            .copied()
            .unwrap_or(0)
            + stage_analysis
                .duplicate_drop_events
                .get("duplicate_packet_dropped")
                .copied()
            .unwrap_or(0)
            > 0,
        "duplicate-noise regression must still observe duplicate packets and classify them as drops instead of failures"
    );
    assert_eq!(
        stage_analysis
            .client_terminal_events
            .get("response_transport_terminal_close_duplicate")
            .copied()
            .unwrap_or(0),
        0,
        "duplicate-noise regression must not route duplicate CloseStream markers through the old terminal duplicate lifecycle path"
    );
    assert!(
        stage_analysis
            .client_terminal_events
            .get("duplicate_close_stream_suppressed")
            .copied()
            .unwrap_or(0)
            > 0,
        "duplicate-noise regression must still observe duplicate CloseStream traffic and suppress it explicitly"
    );

    Ok(())
}

#[tokio::test]
async fn combined_chaos_terminal_close_duplicates_are_suppressed_after_first_signal(
) -> anyhow::Result<()> {
    support::prepare_baseline(BaselineMode::Local);

    let config =
        ChaosRunConfig::combined_regression("terminal-tail-ack-gap-regression-test", 20_900, 5);
    prepare_chaos_run(&config)?;

    let exact_ports = ports_from_base(config.base_port);
    let adaptive_ports = ports_from_base(config.base_port.saturating_add(100));

    run_mode_scenario(
        &config,
        RunMode::Exact3Hop,
        exact_ports,
        true,
        &config.exact_route_cache_path,
    )
    .await?;
    run_mode_scenario(
        &config,
        RunMode::Adaptive,
        adaptive_ports,
        false,
        &config.adaptive_route_cache_path,
    )
    .await?;

    let measurements = read_measurements(&config.raw_path())?;
    assert_eq!(
        measurements.len(),
        config.profile.runs * 2,
        "terminal-tail regression must produce successful exact and adaptive measurements"
    );
    assert!(
        measurements.iter().all(|record| record.status_code == 200),
        "terminal-tail regression must keep HTTP 200"
    );

    let stage_analysis = analyze_stage_trace(&config.stage_trace_path)?;
    assert_eq!(
        stage_analysis
            .client_terminal_events
            .get("response_transport_terminal_close_duplicate")
            .copied()
            .unwrap_or(0),
        0,
        "terminal-tail regression must stop forwarding duplicate CloseStream markers into settlement lifecycle"
    );
    assert!(
        stage_analysis
            .client_terminal_events
            .get("duplicate_close_stream_suppressed")
            .copied()
            .unwrap_or(0)
            > 0,
        "terminal-tail regression must still observe duplicate CloseStream traffic and suppress it explicitly"
    );
    assert!(
        stage_analysis
            .exit_ack_events
            .get("cumulative_ack_advanced")
            .copied()
            .unwrap_or(0)
            > 0,
        "terminal-tail regression must prove response ACKs still advance cumulatively"
    );
    assert_eq!(
        stage_analysis
            .exit_ack_events
            .get("ack_gap_detected")
            .copied()
            .unwrap_or(0),
        0,
        "terminal-tail regression must not surface ACK gaps once terminal close duplicates are suppressed"
    );
    assert_eq!(
        stage_analysis
            .exit_ack_events
            .get("ack_regression_ignored")
            .copied()
            .unwrap_or(0),
        0,
        "terminal-tail regression must collapse older cumulative ACKs into harmless stale duplicates"
    );
    assert!(
        stage_analysis
            .exit_ack_events
            .get("stale_ack_ignored")
            .copied()
            .unwrap_or(0)
            > 0,
        "terminal-tail regression must still observe reordered older ACKs and classify them as stale"
    );

    Ok(())
}

#[tokio::test]
async fn combined_chaos_terminal_payload_tail_is_absorbed_before_completed_tombstone(
) -> anyhow::Result<()> {
    support::prepare_baseline(BaselineMode::Local);

    let config =
        ChaosRunConfig::combined_regression("terminal-payload-tail-regression-test", 21_000, 5);
    prepare_chaos_run(&config)?;

    let exact_ports = ports_from_base(config.base_port);
    let adaptive_ports = ports_from_base(config.base_port.saturating_add(100));

    run_mode_scenario(
        &config,
        RunMode::Exact3Hop,
        exact_ports,
        true,
        &config.exact_route_cache_path,
    )
    .await?;
    run_mode_scenario(
        &config,
        RunMode::Adaptive,
        adaptive_ports,
        false,
        &config.adaptive_route_cache_path,
    )
    .await?;

    let measurements = read_measurements(&config.raw_path())?;
    assert_eq!(
        measurements.len(),
        config.profile.runs * 2,
        "terminal-payload regression must produce successful exact and adaptive measurements"
    );
    assert!(
        measurements.iter().all(|record| record.status_code == 200),
        "terminal-payload regression must keep HTTP 200"
    );

    let stage_analysis = analyze_stage_trace(&config.stage_trace_path)?;
    assert_eq!(
        stage_analysis
            .client_terminal_events
            .get("terminal_payload_after_local_completion")
            .copied()
            .unwrap_or(0),
        0,
        "terminal-payload regression must not route any post-completion empty frame into terminal_payload_after_local_completion"
    );
    assert_eq!(
        stage_analysis
            .client_terminal_events
            .get("duplicate_terminal_payload_after_local_completion")
            .copied()
            .unwrap_or(0),
        0,
        "terminal-payload regression must not route duplicate post-completion empty frames into duplicate_terminal_payload_after_local_completion"
    );
    assert!(
        stage_analysis
            .client_terminal_events
            .get("response_transport_terminal_payload_observed")
            .copied()
            .unwrap_or(0)
            > 0,
        "terminal-payload regression must still observe terminal payload markers during active settlement"
    );
    assert!(
        stage_analysis
            .client_terminal_events
            .get("response_transport_terminal_close_observed")
            .copied()
            .unwrap_or(0)
            > 0,
        "terminal-payload regression must still observe terminal CloseStream during active settlement"
    );
    assert_eq!(
        stage_analysis
            .client_late_events
            .get("payload_after_local_completion_during_settlement")
            .copied()
            .unwrap_or(0),
        0,
        "terminal-payload regression must not deliver additional body bytes after local completion"
    );

    Ok(())
}

#[tokio::test]
async fn combined_chaos_duplicate_payload_tail_is_absorbed_before_completed_tombstone(
) -> anyhow::Result<()> {
    support::prepare_baseline(BaselineMode::Local);

    let config =
        ChaosRunConfig::combined_regression("duplicate-payload-tail-regression-test", 21_100, 5);
    prepare_chaos_run(&config)?;

    let exact_ports = ports_from_base(config.base_port);
    let adaptive_ports = ports_from_base(config.base_port.saturating_add(100));

    run_mode_scenario(
        &config,
        RunMode::Exact3Hop,
        exact_ports,
        true,
        &config.exact_route_cache_path,
    )
    .await?;
    run_mode_scenario(
        &config,
        RunMode::Adaptive,
        adaptive_ports,
        false,
        &config.adaptive_route_cache_path,
    )
    .await?;

    let measurements = read_measurements(&config.raw_path())?;
    assert_eq!(
        measurements.len(),
        config.profile.runs * 2,
        "duplicate-payload regression must produce successful exact and adaptive measurements"
    );
    assert!(
        measurements.iter().all(|record| record.status_code == 200),
        "duplicate-payload regression must keep HTTP 200"
    );

    let stage_analysis = analyze_stage_trace(&config.stage_trace_path)?;
    assert_eq!(
        stage_analysis
            .client_terminal_events
            .get("duplicate_payload_after_local_completion")
            .copied()
            .unwrap_or(0),
        0,
        "duplicate-payload regression must not route duplicate covered DATA frames into duplicate_payload_after_local_completion"
    );
    assert_eq!(
        stage_analysis
            .client_terminal_events
            .get("duplicate_payload_after_local_completion_repeat")
            .copied()
            .unwrap_or(0),
        0,
        "duplicate-payload regression must not route repeated duplicate covered DATA frames into duplicate_payload_after_local_completion_repeat"
    );
    assert_eq!(
        stage_analysis
            .client_late_events
            .get("payload_after_local_completion_during_settlement")
            .copied()
            .unwrap_or(0),
        0,
        "duplicate-payload regression must not deliver any extra body bytes during settlement"
    );
    assert!(
        stage_analysis
            .chaos_counts
            .get("chaos_duplicate")
            .copied()
            .unwrap_or(0)
            > 0,
        "duplicate-payload regression must still inject duplicate transport packets even when no duplicate covered DATA tail reaches completed/tombstone handling"
    );
    assert!(
        stage_analysis
            .client_terminal_events
            .get("response_transport_local_completion")
            .copied()
            .unwrap_or(0)
            > 0,
        "duplicate-payload regression must still reach local buffered completion before settlement cleanup"
    );
    assert!(
        stage_analysis
            .client_terminal_events
            .get("response_transport_settlement_completed")
            .copied()
            .unwrap_or(0)
            > 0,
        "duplicate-payload regression must still complete settlement cleanly"
    );

    Ok(())
}

#[tokio::test]
async fn response_pacing_reduces_window_stall_under_combined() -> anyhow::Result<()> {
    support::prepare_baseline(BaselineMode::Local);

    let before_config =
        ChaosRunConfig::pacing_regression("response-pacing-before-regression-test", 21_200, 5);
    prepare_chaos_run(&before_config)?;
    let before_ports = ports_from_base(before_config.base_port);
    let mut before_exit = ExitConfigCli::default();
    before_exit.response_pacing_enabled = false;
    run_mode_scenario_with_exit_config(
        &before_config,
        RunMode::Exact3Hop,
        before_ports,
        true,
        &before_config.exact_route_cache_path,
        before_exit,
    )
    .await?;

    let before_measurements = read_measurements(&before_config.raw_path())?;
    assert_eq!(
        before_measurements.len(),
        before_config.profile.runs,
        "pre-pacing combined regression must produce one exact measurement per run"
    );
    assert!(
        before_measurements
            .iter()
            .all(|record| record.status_code == 200),
        "pre-pacing combined regression must keep HTTP 200"
    );
    let after_config =
        ChaosRunConfig::pacing_regression("response-pacing-after-regression-test", 21_350, 5);
    prepare_chaos_run(&after_config)?;
    let after_ports = ports_from_base(after_config.base_port);
    let mut after_exit = ExitConfigCli::default();
    after_exit.response_pacing_enabled = true;
    after_exit.response_pacing_bootstrap_rtt_ms = 200;
    after_exit.response_pacing_min_interval_ms = 1;
    run_mode_scenario_with_exit_config(
        &after_config,
        RunMode::Exact3Hop,
        after_ports,
        true,
        &after_config.exact_route_cache_path,
        after_exit,
    )
    .await?;

    let after_measurements = read_measurements(&after_config.raw_path())?;
    assert_eq!(
        after_measurements.len(),
        after_config.profile.runs,
        "paced combined regression must produce one exact measurement per run"
    );
    assert!(
        after_measurements
            .iter()
            .all(|record| record.status_code == 200),
        "paced combined regression must keep HTTP 200"
    );

    let before_analysis = analyze_stage_trace(&before_config.stage_trace_path)?;
    let before_streams: Vec<_> = before_analysis
        .exact_streams
        .iter()
        .filter(|stream| stream.site.contains(&before_config.profile.profile_name))
        .cloned()
        .collect();
    let after_analysis = analyze_stage_trace(&after_config.stage_trace_path)?;
    let after_streams: Vec<_> = after_analysis
        .exact_streams
        .iter()
        .filter(|stream| stream.site.contains(&after_config.profile.profile_name))
        .cloned()
        .collect();

    assert_eq!(
        before_streams.len(),
        before_config.profile.runs,
        "pre-pacing combined regression must emit one exact exit summary per run"
    );
    assert_eq!(
        after_streams.len(),
        after_config.profile.runs,
        "paced combined regression must emit one exact exit summary per run"
    );

    let before_avg_burst = average_optional(
        before_streams
            .iter()
            .map(|stream| stream.max_send_burst_frames),
    )
    .unwrap_or(0.0);
    let after_avg_burst = average_optional(
        after_streams
            .iter()
            .map(|stream| stream.max_send_burst_frames),
    )
    .unwrap_or(0.0);
    assert!(
        before_avg_burst >= 16.0,
        "pre-pacing combined regression must show bursty response sends before the fixed window fills"
    );
    assert!(
        after_avg_burst < before_avg_burst,
        "paced combined regression must reduce max send burst size ({after_avg_burst:.2} < {before_avg_burst:.2})"
    );
    assert!(
        after_streams
            .iter()
            .map(|stream| stream.pacing_delay_applied.unwrap_or(0))
            .sum::<u64>()
            > 0,
        "paced combined regression must apply pacing delay at least once"
    );
    assert!(
        after_streams
            .iter()
            .map(|stream| stream.burst_prevented_count.unwrap_or(0))
            .sum::<u64>()
            > 0,
        "paced combined regression must prove a burst was actively prevented"
    );

    Ok(())
}

#[tokio::test]
async fn response_inflight_discipline_avoids_transient_overtrigger_on_moderate_route(
) -> anyhow::Result<()> {
    support::prepare_baseline(BaselineMode::Local);

    let moderate_config = ChaosRunConfig::inflight_moderate_regression(
        "response-inflight-moderate-regression-test",
        21_500,
        3,
    );
    prepare_chaos_run(&moderate_config)?;
    let moderate_ports = ports_from_base(moderate_config.base_port);
    let mut moderate_exit = ExitConfigCli::default();
    moderate_exit.response_pacing_enabled = true;
    moderate_exit.response_inflight_discipline_enabled = true;
    run_mode_scenario_with_exit_config(
        &moderate_config,
        RunMode::Exact3Hop,
        moderate_ports,
        true,
        &moderate_config.exact_route_cache_path,
        moderate_exit,
    )
    .await?;

    let moderate_measurements = read_measurements(&moderate_config.raw_path())?;
    assert_eq!(
        moderate_measurements.len(),
        moderate_config.profile.runs,
        "moderate discipline regression must produce one exact measurement per run"
    );
    assert!(
        moderate_measurements
            .iter()
            .all(|record| record.status_code == 200),
        "moderate discipline regression must keep HTTP 200"
    );

    let moderate_analysis = analyze_stage_trace(&moderate_config.stage_trace_path)?;
    let moderate_streams: Vec<_> = moderate_analysis
        .exact_streams
        .iter()
        .filter(|stream| stream.site.contains(&moderate_config.profile.profile_name))
        .cloned()
        .collect();

    assert_eq!(
        moderate_streams.len(),
        moderate_config.profile.runs,
        "moderate discipline regression must emit one exact exit summary per run"
    );

    let moderate_avg_effective_cap_wait = average_optional(
        moderate_streams
            .iter()
            .map(|stream| stream.effective_cap_wait_total_ms),
    )
    .unwrap_or(0.0);
    let moderate_avg_effective_cap = average_optional(
        moderate_streams
            .iter()
            .map(|stream| stream.effective_inflight_cap_avg),
    )
    .unwrap_or(64.0);
    let moderate_reductions = moderate_streams
        .iter()
        .map(|stream| stream.inflight_cap_reduced_count.unwrap_or(0))
        .sum::<u64>();
    let moderate_blocked = moderate_streams
        .iter()
        .map(|stream| stream.send_blocked_by_effective_cap.unwrap_or(0))
        .sum::<u64>();

    assert!(
        moderate_avg_effective_cap >= 64.0,
        "moderate regression must keep the effective inflight cap at the hard window ({moderate_avg_effective_cap:.2} >= 64)"
    );
    assert!(
        moderate_reductions == 0,
        "moderate regression must not reduce the effective inflight cap on transient pressure ({moderate_reductions} == 0)"
    );
    assert!(
        moderate_blocked == 0,
        "moderate regression must not block on the effective inflight cap ({moderate_blocked} == 0)"
    );
    assert!(
        moderate_avg_effective_cap_wait == 0.0,
        "moderate regression must not accumulate effective-cap wait ({moderate_avg_effective_cap_wait:.2} == 0)"
    );

    Ok(())
}

async fn run_mode_scenario(
    config: &ChaosRunConfig,
    mode: RunMode,
    ports: LocalPorts,
    exact_route_only: bool,
    route_cache_path: &Path,
) -> anyhow::Result<()> {
    run_mode_scenario_with_exit_config(
        config,
        mode,
        ports,
        exact_route_only,
        route_cache_path,
        ExitConfigCli::default(),
    )
    .await
}

async fn run_mode_scenario_with_exit_config(
    config: &ChaosRunConfig,
    mode: RunMode,
    ports: LocalPorts,
    exact_route_only: bool,
    route_cache_path: &Path,
    exit_config: ExitConfigCli,
) -> anyhow::Result<()> {
    let stack = spawn_local_stack(
        ports,
        mode.route_length(),
        exact_route_only,
        route_cache_path,
        exit_config,
    );

    helpers::wait_http_ready(ports.client_tcp)
        .await
        .with_context(|| format!("{} client path did not become ready", mode.as_str()))?;

    for run_index in 1..=config.profile.runs {
        let host = format!(
            "chaos-{}-run{run_index}-{}",
            mode.as_str(),
            config.profile.profile_name
        );
        let request = build_http_post_request(&host, "/chaos", config.profile.request_body_bytes);
        let result = do_measure_http_request(ports.client_tcp, &request, config).await?;
        append_record(
            &config.raw_path(),
            &MeasurementRecord {
                profile_name: config.profile.profile_name.clone(),
                mode: mode.as_str().to_string(),
                run_index,
                route_length: mode.route_length(),
                status_code: result.status_code,
                connect_time_ms: result.connect_time_ms,
                ttfb_ms: result.ttfb_ms,
                total_time_ms: result.total_time_ms,
                response_bytes: result.response_bytes,
                body_bytes: result.body_bytes,
                effective_throughput_bps: result.effective_throughput_bps,
            },
        )?;
    }

    tokio::time::sleep(Duration::from_millis(500)).await;
    stack.abort_all();
    tokio::time::sleep(Duration::from_millis(100)).await;

    Ok(())
}

fn spawn_local_stack(
    ports: LocalPorts,
    route_length: u8,
    exact_route_only: bool,
    route_cache_path: &Path,
    mut exit_config: ExitConfigCli,
) -> LocalStackHandles {
    let target_handle = tokio::spawn(async move {
        let _ = vpnnode::roles::target::run_target(TargetConfigCli {
            listen: ports.target_tcp,
        })
        .await;
    });

    exit_config.listen = ports.exit_udp;
    exit_config.target_addr = ports.target_tcp.to_string();
    exit_config.exit_key_path = "exit.key".to_string();
    let exit_handle = tokio::spawn(async move {
        let _ = vpnnode::roles::exit::run_exit(exit_config).await;
    });

    let relay2_handle = tokio::spawn(async move {
        let _ = vpnnode::roles::relay::run_relay(RelayConfigCli {
            listen: ports.relay2_udp,
            exit_addr: ports.exit_udp.to_string(),
            relay_key_path: "relay.key".to_string(),
            ..Default::default()
        })
        .await;
    });

    let relay1_handle = tokio::spawn(async move {
        let _ = vpnnode::roles::relay::run_relay(RelayConfigCli {
            listen: ports.relay1_udp,
            exit_addr: ports.relay2_udp.to_string(),
            relay_key_path: "relay.key".to_string(),
            ..Default::default()
        })
        .await;
    });

    let route_cache_path = route_cache_path.to_string_lossy().to_string();
    let client_handle = tokio::spawn(async move {
        let _ = vpnnode::roles::client::run_client(ClientConfigCli {
            local_listen: ports.client_tcp,
            mode: "tcp".to_string(),
            relay_addr: ports.relay1_udp.to_string(),
            exit_addr: ports.exit_udp.to_string(),
            route_length,
            relay2_addr: Some(ports.relay2_udp.to_string()),
            exact_route_only,
            client_key_path: "client.key".to_string(),
            relay_pubkey_path: "relay.pub".to_string(),
            route_cache_path,
            tun_name: format!(
                "tun-chaos-{}-{}",
                if exact_route_only {
                    "exact"
                } else {
                    "adaptive"
                },
                ports.client_tcp.port()
            ),
            tun_address: "10.10.0.1".to_string(),
            tun_netmask: "255.255.255.0".to_string(),
            tun_mtu: 1500,
            max_inflight_frames: 64,
            retransmit_interval: 200,
            chunk_size: 1000,
            ..Default::default()
        })
        .await;
    });

    LocalStackHandles {
        handles: vec![
            target_handle,
            exit_handle,
            relay2_handle,
            relay1_handle,
            client_handle,
        ],
    }
}

fn build_http_post_request(host: &str, path: &str, body_bytes: usize) -> Vec<u8> {
    let body = vec![b'Z'; body_bytes];
    let header = format!(
        "POST {path} HTTP/1.1\r\nHost: {host}\r\nContent-Length: {body_bytes}\r\nConnection: close\r\n\r\n"
    );
    let mut request = Vec::with_capacity(header.len() + body.len());
    request.extend_from_slice(header.as_bytes());
    request.extend_from_slice(&body);
    request
}

async fn do_measure_http_request(
    addr: SocketAddr,
    request: &[u8],
    config: &ChaosRunConfig,
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

    let status_code = helpers::extract_http_status_code(&response)
        .ok_or_else(|| anyhow!("response missing HTTP status code"))?;
    let header_end = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|position| position + 4)
        .ok_or_else(|| anyhow!("response missing header terminator"))?;
    let expected_body_bytes = parse_content_length(&response[..header_end])?;
    let body_bytes = response.len().saturating_sub(header_end);
    let total_time_ms = overall_start.elapsed().as_secs_f64() * 1000.0;
    let ttfb_ms = first_byte_time_ms.unwrap_or(total_time_ms);
    let effective_throughput_bps = if total_time_ms > 0.0 {
        body_bytes as f64 / (total_time_ms / 1000.0)
    } else {
        0.0
    };

    if status_code != 200 {
        bail!("expected 200 OK from {addr}, got {status_code}");
    }
    if body_bytes != expected_body_bytes {
        bail!(
            "expected complete body of {expected_body_bytes} bytes from {addr}, got {body_bytes}"
        );
    }

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

fn prepare_chaos_run(config: &ChaosRunConfig) -> anyhow::Result<()> {
    prepare_output_path(&config.artifact_prefix)?;
    prepare_output_path(&config.stage_trace_path)?;
    fs::write(config.raw_path(), b"").context("truncate chaos raw output")?;
    fs::write(config.stage_trace_path.clone(), b"").context("truncate chaos stage trace")?;
    fs::write(config.decision_trace_path(), b"").context("truncate chaos decision trace")?;
    let _ = fs::remove_file(&config.exact_route_cache_path);
    let _ = fs::remove_file(&config.adaptive_route_cache_path);
    fs::write(
        config.profile_json_path(),
        serde_json::to_vec_pretty(&config.profile).context("serialize chaos profile")?,
    )
    .context("write chaos profile json")?;

    std::env::set_var(
        "VPNNODE_STAGE_TRACE_PATH",
        config.stage_trace_path.as_os_str(),
    );
    vpnnode::stage_trace::refresh_from_env();
    std::env::set_var(
        "VPNNODE_TRANSPORT_CHAOS_LABEL",
        config.profile.profile_name.as_str(),
    );
    std::env::set_var(
        "VPNNODE_TRANSPORT_CHAOS_SEED",
        config.profile.seed.to_string(),
    );
    std::env::set_var(
        "VPNNODE_TRANSPORT_CHAOS_SKIP_PACKETS",
        config.profile.skip_packets.to_string(),
    );
    std::env::set_var(
        "VPNNODE_TRANSPORT_CHAOS_LOSS_PPM",
        config.profile.loss_ppm.to_string(),
    );
    std::env::set_var(
        "VPNNODE_TRANSPORT_CHAOS_DUPLICATE_PPM",
        config.profile.duplicate_ppm.to_string(),
    );
    std::env::set_var(
        "VPNNODE_TRANSPORT_CHAOS_REORDER_PPM",
        config.profile.reorder_ppm.to_string(),
    );
    std::env::set_var(
        "VPNNODE_TRANSPORT_CHAOS_BASE_DELAY_MS",
        config.profile.base_delay_ms.to_string(),
    );
    std::env::set_var(
        "VPNNODE_TRANSPORT_CHAOS_JITTER_MS",
        config.profile.jitter_ms.to_string(),
    );
    std::env::set_var(
        "VPNNODE_TRANSPORT_CHAOS_REORDER_EXTRA_DELAY_MS",
        config.profile.reorder_extra_delay_ms.to_string(),
    );
    std::env::set_var(
        "VPNNODE_TRANSPORT_CHAOS_DUPLICATE_DELAY_MS",
        config.profile.duplicate_delay_ms.to_string(),
    );
    Ok(())
}

fn parse_content_length(header_bytes: &[u8]) -> anyhow::Result<usize> {
    let header = std::str::from_utf8(header_bytes).context("response headers not utf-8")?;
    for line in header.lines() {
        if let Some(rest) = line.strip_prefix("Content-Length:") {
            return rest
                .trim()
                .parse::<usize>()
                .map_err(|err| anyhow!("parse Content-Length '{}': {err}", rest.trim()));
        }
    }
    bail!("response missing Content-Length header")
}

fn analyze_stage_trace(path: &Path) -> anyhow::Result<StageAnalysis> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("read stage trace {}", path.display()))?;
    let mut analysis = StageAnalysis::default();
    for line in raw.lines().filter(|line| !line.trim().is_empty()) {
        let value: Value = serde_json::from_str(line).context("parse stage trace line")?;
        let component = value.get("component").and_then(Value::as_str).unwrap_or("");
        let stage = value.get("stage").and_then(Value::as_str).unwrap_or("");
        match (component, stage) {
            ("transport", "chaos_drop")
            | ("transport", "chaos_duplicate")
            | ("transport", "chaos_delay") => {
                *analysis
                    .chaos_counts
                    .entry(stage.to_string())
                    .or_insert(0usize) += 1;
            }
            ("exit", "open_message_failed") => {
                let error = value
                    .get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("<missing>");
                *analysis
                    .open_message_failures
                    .entry(normalize_open_message_error(error))
                    .or_insert(0usize) += 1;
            }
            ("exit", "duplicate_packet_dropped") => {
                *analysis
                    .duplicate_drop_events
                    .entry(stage.to_string())
                    .or_insert(0usize) += 1;
            }
            ("exit", "cumulative_ack_advanced")
            | ("exit", "ack_gap_detected")
            | ("exit", "stale_ack_ignored")
            | ("exit", "ack_regression_ignored")
            | ("exit", "inflight_cleanup_by_ack_range") => {
                *analysis
                    .exit_ack_events
                    .entry(stage.to_string())
                    .or_insert(0usize) += 1;
            }
            ("client", "open_message_failed") => {
                let error = value
                    .get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("<missing>");
                *analysis
                    .client_open_message_failures
                    .entry(normalize_open_message_error(error))
                    .or_insert(0usize) += 1;
            }
            ("client", "duplicate_packet_dropped") => {
                *analysis
                    .client_duplicate_drop_events
                    .entry(stage.to_string())
                    .or_insert(0usize) += 1;
            }
            ("client", "late_payload_after_completion")
            | ("client", "late_close_after_completion")
            | ("client", "payload_after_local_completion_during_settlement")
            | ("client", "response_timeout") => {
                *analysis
                    .client_late_events
                    .entry(stage.to_string())
                    .or_insert(0usize) += 1;
            }
            ("client", "stale_active_response_state_pruned")
            | ("client", "orphaned_response_payload")
            | ("client", "late_ack_after_completion")
            | ("client", "ack_for_unknown_stream")
            | ("client", "response_payload_unknown_stream") => {
                *analysis
                    .client_lifecycle_events
                    .entry(stage.to_string())
                    .or_insert(0usize) += 1;
            }
            ("client", "terminal_payload_after_local_completion")
            | ("client", "duplicate_terminal_payload_after_local_completion")
            | ("client", "terminal_close_after_local_completion")
            | ("client", "duplicate_terminal_close_after_local_completion")
            | ("client", "terminal_close_before_response_start_queued")
            | ("client", "terminal_close_before_local_completion_queued")
            | ("client", "duplicate_close_stream_suppressed")
            | ("client", "duplicate_payload_after_local_completion")
            | ("client", "duplicate_payload_after_local_completion_repeat")
            | ("client", "duplicate_payload_after_local_completion_absorbed")
            | ("client", "response_transport_local_completion")
            | ("client", "response_transport_settlement_started")
            | ("client", "response_transport_terminal_payload_observed")
            | ("client", "response_transport_terminal_payload_duplicate")
            | ("client", "response_transport_terminal_close_observed")
            | ("client", "response_transport_terminal_close_duplicate")
            | ("client", "response_transport_settlement_completed") => {
                *analysis
                    .client_terminal_events
                    .entry(stage.to_string())
                    .or_insert(0usize) += 1;
            }
            ("exit", "stream_complete") => {
                let site = value
                    .get("site")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let summary = ExitStreamSummary {
                    site: site.clone(),
                    route_len: value.get("route_len").and_then(Value::as_u64).unwrap_or(0),
                    stream_duration_ms: value.get("stream_duration_ms").and_then(Value::as_u64),
                    first_target_byte_ms: value.get("first_target_byte_ms").and_then(Value::as_u64),
                    first_overlay_send_ms: value
                        .get("first_overlay_send_ms")
                        .and_then(Value::as_u64),
                    avg_inflight: value.get("avg_inflight").and_then(Value::as_f64),
                    max_inflight: value.get("max_inflight").and_then(Value::as_u64),
                    ack_latency_ms_p95: value.get("ack_latency_ms_p95").and_then(Value::as_u64),
                    ack_latency_ms_avg: value.get("ack_latency_ms_avg").and_then(Value::as_u64),
                    total_retransmits: value.get("total_retransmits").and_then(Value::as_u64),
                    retransmit_rate: value.get("retransmit_rate").and_then(Value::as_f64),
                    retransmit_rate_ppm: value.get("retransmit_rate_ppm").and_then(Value::as_u64),
                    window_wait_total_ms: value.get("window_wait_total_ms").and_then(Value::as_u64),
                    effective_cap_wait_total_ms: value
                        .get("effective_cap_wait_total_ms")
                        .and_then(Value::as_u64),
                    pacing_enabled: value.get("pacing_enabled").and_then(Value::as_bool),
                    pacing_delay_applied: value.get("pacing_delay_applied").and_then(Value::as_u64),
                    pacing_delay_applied_ms_total: value
                        .get("pacing_delay_applied_ms_total")
                        .and_then(Value::as_u64),
                    pacing_interval_ms_avg: value
                        .get("pacing_interval_ms_avg")
                        .and_then(Value::as_u64),
                    pacing_interval_ms_max: value
                        .get("pacing_interval_ms_max")
                        .and_then(Value::as_u64),
                    burst_prevented_count: value
                        .get("burst_prevented_count")
                        .and_then(Value::as_u64),
                    paced_send_batches: value.get("paced_send_batches").and_then(Value::as_u64),
                    max_send_burst_frames: value
                        .get("max_send_burst_frames")
                        .and_then(Value::as_u64),
                    effective_inflight_cap_avg: value
                        .get("effective_inflight_cap_avg")
                        .and_then(Value::as_u64),
                    effective_inflight_cap_min: value
                        .get("effective_inflight_cap_min")
                        .and_then(Value::as_u64),
                    effective_inflight_cap_max: value
                        .get("effective_inflight_cap_max")
                        .and_then(Value::as_u64),
                    inflight_cap_reduced_count: value
                        .get("inflight_cap_reduced_count")
                        .and_then(Value::as_u64),
                    inflight_cap_restore_count: value
                        .get("inflight_cap_restore_count")
                        .and_then(Value::as_u64),
                    ack_pressure_events: value.get("ack_pressure_events").and_then(Value::as_u64),
                    send_blocked_by_effective_cap: value
                        .get("send_blocked_by_effective_cap")
                        .and_then(Value::as_u64),
                    http_code: value.get("http_code").and_then(Value::as_u64),
                };
                if site.starts_with("chaos-exact-3hop-run") {
                    analysis.exact_streams.push(summary);
                } else if site.starts_with("chaos-adaptive-run") {
                    analysis.adaptive_streams.push(summary);
                }
            }
            ("client", "route_candidates_scored") => {
                let site = value.get("site").and_then(Value::as_str).unwrap_or("");
                if site.starts_with("chaos-adaptive-run") {
                    analysis.adaptive_decisions.push(value);
                }
            }
            _ => {}
        }
    }
    Ok(analysis)
}

fn normalize_open_message_error(error: &str) -> String {
    if error.starts_with("packet too old for replay window") {
        "packet too old for replay window".to_string()
    } else if error.starts_with("duplicate packet detected") {
        "duplicate packet detected".to_string()
    } else {
        error.to_string()
    }
}

fn write_decision_trace(path: &Path, decisions: &[Value]) -> anyhow::Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .with_context(|| format!("open decision trace output {}", path.display()))?;
    for decision in decisions {
        serde_json::to_writer(&mut file, decision).context("serialize decision trace event")?;
        file.write_all(b"\n").context("append decision newline")?;
    }
    Ok(())
}

fn render_report(config: &ChaosRunConfig, analysis: &StageAnalysis) -> anyhow::Result<String> {
    let measurements = read_measurements(&config.raw_path())?;
    let exact_records: Vec<_> = measurements
        .iter()
        .filter(|record| record.mode == RunMode::Exact3Hop.as_str())
        .collect();
    let adaptive_records: Vec<_> = measurements
        .iter()
        .filter(|record| record.mode == RunMode::Adaptive.as_str())
        .collect();
    let exact_selection_changes = 0usize;
    let adaptive_selection_changes = count_route_changes(&analysis.adaptive_decisions);

    let mut out = String::new();
    out.push_str(&format!(
        "# Chaos Profile Report: {}\n\n",
        config.profile.profile_name
    ));
    out.push_str("## Profile\n\n");
    out.push_str("```json\n");
    out.push_str(
        &serde_json::to_string_pretty(&config.profile).context("serialize profile for report")?,
    );
    out.push_str("\n```\n\n");
    out.push_str("## Measurements\n\n");
    out.push_str(&format_measurement_block(
        RunMode::Exact3Hop.as_str(),
        &exact_records,
    ));
    out.push('\n');
    out.push_str(&format_measurement_block(
        RunMode::Adaptive.as_str(),
        &adaptive_records,
    ));
    out.push_str("\n## Exit Response Path\n\n");
    out.push_str(&format_exit_stream_block(
        "exact-3hop",
        &analysis.exact_streams,
    ));
    out.push('\n');
    out.push_str(&format_exit_stream_block(
        "adaptive",
        &analysis.adaptive_streams,
    ));
    out.push_str("\n## Route Decisions\n\n");
    out.push_str(&format!(
        "- adaptive decision events: {}\n- adaptive route changes: {}\n- exact route changes: {}\n",
        analysis.adaptive_decisions.len(),
        adaptive_selection_changes,
        exact_selection_changes,
    ));
    let decision_reasons = count_decision_reasons(&analysis.adaptive_decisions);
    if !decision_reasons.is_empty() {
        out.push_str("- adaptive decision reasons:\n");
        for (reason, count) in decision_reasons {
            out.push_str(&format!("  - {reason}: {count}\n"));
        }
    }
    out.push_str("\n## Chaos Actions\n\n");
    if analysis.chaos_counts.is_empty() {
        out.push_str("- no transport chaos actions emitted\n");
    } else {
        for (stage, count) in &analysis.chaos_counts {
            out.push_str(&format!("- {stage}: {count}\n"));
        }
    }
    if !analysis.open_message_failures.is_empty() {
        out.push_str("\n## Open Message Failures\n\n");
        for (error, count) in &analysis.open_message_failures {
            out.push_str(&format!("- {error}: {count}\n"));
        }
    }
    if !analysis.duplicate_drop_events.is_empty() {
        out.push_str("\n## Exit Duplicate Drops\n\n");
        for (stage, count) in &analysis.duplicate_drop_events {
            out.push_str(&format!("- {stage}: {count}\n"));
        }
    }
    if !analysis.client_late_events.is_empty() {
        out.push_str("\n## Client Late Events\n\n");
        for (stage, count) in &analysis.client_late_events {
            out.push_str(&format!("- {stage}: {count}\n"));
        }
    }
    if !analysis.client_open_message_failures.is_empty() {
        out.push_str("\n## Client Open Message Failures\n\n");
        for (error, count) in &analysis.client_open_message_failures {
            out.push_str(&format!("- {error}: {count}\n"));
        }
    }
    if !analysis.client_duplicate_drop_events.is_empty() {
        out.push_str("\n## Client Duplicate Drops\n\n");
        for (stage, count) in &analysis.client_duplicate_drop_events {
            out.push_str(&format!("- {stage}: {count}\n"));
        }
    }
    if !analysis.client_lifecycle_events.is_empty() {
        out.push_str("\n## Client Lifecycle Events\n\n");
        for (stage, count) in &analysis.client_lifecycle_events {
            out.push_str(&format!("- {stage}: {count}\n"));
        }
    }
    if !analysis.exit_ack_events.is_empty() {
        out.push_str("\n## Exit ACK Events\n\n");
        for (stage, count) in &analysis.exit_ack_events {
            out.push_str(&format!("- {stage}: {count}\n"));
        }
    }
    if !analysis.client_terminal_events.is_empty() {
        out.push_str("\n## Client Settlement Events\n\n");
        for (stage, count) in &analysis.client_terminal_events {
            out.push_str(&format!("- {stage}: {count}\n"));
        }
    }
    Ok(out)
}

fn format_measurement_block(label: &str, records: &[&MeasurementRecord]) -> String {
    if records.is_empty() {
        return format!("### {label}\n\n- no records\n");
    }
    let avg_connect = average(records.iter().map(|record| record.connect_time_ms));
    let avg_ttfb = average(records.iter().map(|record| record.ttfb_ms));
    let avg_total = average(records.iter().map(|record| record.total_time_ms));
    let avg_throughput = average(records.iter().map(|record| record.effective_throughput_bps));
    format!(
        "### {label}\n\n- runs: {}\n- avg connect ms: {:.2}\n- avg TTFB ms: {:.2}\n- avg total ms: {:.2}\n- avg throughput Bps: {:.2}\n",
        records.len(),
        avg_connect,
        avg_ttfb,
        avg_total,
        avg_throughput,
    )
}

fn format_exit_stream_block(label: &str, streams: &[ExitStreamSummary]) -> String {
    if streams.is_empty() {
        return format!("### {label}\n\n- no exit stream summaries\n");
    }
    let mut route_len_counts = BTreeMap::new();
    let mut http_code_counts = BTreeMap::new();
    for stream in streams {
        *route_len_counts.entry(stream.route_len).or_insert(0usize) += 1;
        *http_code_counts
            .entry(stream.http_code.unwrap_or_default())
            .or_insert(0usize) += 1;
    }
    let avg_ack_p95 = average_optional(streams.iter().map(|stream| stream.ack_latency_ms_p95));
    let avg_ack_avg = average_optional(streams.iter().map(|stream| stream.ack_latency_ms_avg));
    let avg_total_retransmits =
        average_optional(streams.iter().map(|stream| stream.total_retransmits));
    let avg_retransmit_rate =
        average_f64_optional(streams.iter().map(|stream| stream.retransmit_rate));
    let avg_retransmit_ppm =
        average_optional(streams.iter().map(|stream| stream.retransmit_rate_ppm));
    let avg_window_wait =
        average_optional(streams.iter().map(|stream| stream.window_wait_total_ms));
    let avg_effective_cap_wait = average_optional(
        streams
            .iter()
            .map(|stream| stream.effective_cap_wait_total_ms),
    );
    let avg_pacing_delay_ms = average_optional(
        streams
            .iter()
            .map(|stream| stream.pacing_delay_applied_ms_total),
    );
    let avg_pacing_interval_ms =
        average_optional(streams.iter().map(|stream| stream.pacing_interval_ms_avg));
    let avg_burst_prevented =
        average_optional(streams.iter().map(|stream| stream.burst_prevented_count));
    let avg_paced_batches =
        average_optional(streams.iter().map(|stream| stream.paced_send_batches));
    let avg_max_burst = average_optional(streams.iter().map(|stream| stream.max_send_burst_frames));
    let avg_effective_cap = average_optional(
        streams
            .iter()
            .map(|stream| stream.effective_inflight_cap_avg),
    );
    let avg_cap_reduced = average_optional(
        streams
            .iter()
            .map(|stream| stream.inflight_cap_reduced_count),
    );
    let avg_cap_restore = average_optional(
        streams
            .iter()
            .map(|stream| stream.inflight_cap_restore_count),
    );
    let avg_ack_pressure =
        average_optional(streams.iter().map(|stream| stream.ack_pressure_events));
    let avg_blocked_by_cap = average_optional(
        streams
            .iter()
            .map(|stream| stream.send_blocked_by_effective_cap),
    );
    let avg_max_inflight = average_optional(streams.iter().map(|stream| stream.max_inflight));
    let avg_total = average_optional(streams.iter().map(|stream| stream.stream_duration_ms));
    let avg_overlay_gap = average_optional(streams.iter().map(|stream| {
        stream
            .first_overlay_send_ms
            .zip(stream.first_target_byte_ms)
            .map(|(overlay, target)| overlay.saturating_sub(target))
    }));
    let pacing_modes = streams
        .iter()
        .map(|stream| stream.pacing_enabled.unwrap_or(false))
        .collect::<Vec<_>>();
    let pacing_label = if pacing_modes.iter().all(|value| *value) {
        "enabled"
    } else if pacing_modes.iter().all(|value| !*value) {
        "disabled"
    } else {
        "mixed"
    };
    format!(
        "### {label}\n\n- streams: {}\n- sites: {}\n- route lens: {}\n- http codes: {}\n- pacing: {}\n- avg ack avg ms: {}\n- avg ack p95 ms: {}\n- avg total retransmits: {}\n- avg retransmit rate: {}\n- avg retransmit ppm: {}\n- avg hard window wait ms: {}\n- avg effective cap wait ms: {}\n- avg pacing delay ms: {}\n- avg pacing interval ms: {}\n- avg burst prevented: {}\n- avg paced batches: {}\n- avg max send burst frames: {}\n- avg effective inflight cap: {}\n- avg inflight cap reduced count: {}\n- avg inflight cap restore count: {}\n- avg ack pressure events: {}\n- avg blocked by effective cap: {}\n- avg max inflight: {}\n- avg stream duration ms: {}\n- avg overlay first-send gap ms: {}\n",
        streams.len(),
        streams
            .iter()
            .map(|stream| stream.site.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        route_len_counts
            .iter()
            .map(|(route_len, count)| format!("{route_len}x{count}"))
            .collect::<Vec<_>>()
            .join(", "),
        http_code_counts
            .iter()
            .map(|(http_code, count)| format!("{http_code}x{count}"))
            .collect::<Vec<_>>()
            .join(", "),
        pacing_label,
        fmt_optional(avg_ack_avg),
        fmt_optional(avg_ack_p95),
        fmt_optional(avg_total_retransmits),
        avg_retransmit_rate
            .map(|value| format!("{:.4}", value))
            .unwrap_or_else(|| "n/a".to_string()),
        fmt_optional(avg_retransmit_ppm),
        fmt_optional(avg_window_wait),
        fmt_optional(avg_effective_cap_wait),
        fmt_optional(avg_pacing_delay_ms),
        fmt_optional(avg_pacing_interval_ms),
        fmt_optional(avg_burst_prevented),
        fmt_optional(avg_paced_batches),
        fmt_optional(avg_max_burst),
        fmt_optional(avg_effective_cap),
        fmt_optional(avg_cap_reduced),
        fmt_optional(avg_cap_restore),
        fmt_optional(avg_ack_pressure),
        fmt_optional(avg_blocked_by_cap),
        fmt_optional(avg_max_inflight),
        fmt_optional(avg_total),
        fmt_optional(avg_overlay_gap),
    )
}

fn count_decision_reasons(decisions: &[Value]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for decision in decisions {
        let reason = decision
            .get("selection_policy")
            .and_then(|value| value.get("decision_reason"))
            .and_then(Value::as_str)
            .unwrap_or("<missing>");
        *counts.entry(reason.to_string()).or_insert(0) += 1;
    }
    counts
}

fn count_route_changes(decisions: &[Value]) -> usize {
    let mut previous = None::<String>;
    let mut changes = 0usize;
    for decision in decisions {
        let selected = decision
            .get("selection_policy")
            .and_then(|value| value.get("selected_route"))
            .and_then(Value::as_str)
            .map(|value| value.to_string());
        if let Some(selected) = selected {
            if let Some(previous_selected) = previous.as_ref() {
                if previous_selected != &selected {
                    changes = changes.saturating_add(1);
                }
            }
            previous = Some(selected);
        }
    }
    changes
}

fn average(values: impl Iterator<Item = f64>) -> f64 {
    let values: Vec<f64> = values.collect();
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

fn average_optional(values: impl Iterator<Item = Option<u64>>) -> Option<f64> {
    let values: Vec<u64> = values.flatten().collect();
    if values.is_empty() {
        return None;
    }
    Some(values.iter().map(|value| *value as f64).sum::<f64>() / values.len() as f64)
}

fn average_f64_optional(values: impl Iterator<Item = Option<f64>>) -> Option<f64> {
    let values: Vec<f64> = values.flatten().collect();
    if values.is_empty() {
        return None;
    }
    Some(values.iter().sum::<f64>() / values.len() as f64)
}

fn fmt_optional(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.2}"))
        .unwrap_or_else(|| "n/a".to_string())
}

fn read_measurements(path: &Path) -> anyhow::Result<Vec<MeasurementRecord>> {
    let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    raw.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).context("parse measurement line"))
        .collect()
}

fn append_record(path: &Path, record: &MeasurementRecord) -> anyhow::Result<()> {
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .with_context(|| format!("open chaos raw output {}", path.display()))?;
    serde_json::to_writer(&mut file, record).context("serialize chaos measurement")?;
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

fn ports_from_base(base: u16) -> LocalPorts {
    LocalPorts {
        target_tcp: SocketAddr::from(([127, 0, 0, 1], base + 80)),
        exit_udp: SocketAddr::from(([127, 0, 0, 1], base + 1)),
        relay1_udp: SocketAddr::from(([127, 0, 0, 1], base + 2)),
        relay2_udp: SocketAddr::from(([127, 0, 0, 1], base + 3)),
        client_tcp: SocketAddr::from(([127, 0, 0, 1], base + 4)),
    }
}

impl ChaosRunConfig {
    fn from_env() -> anyhow::Result<Self> {
        let profile_name = env_or("VPNNODE_CHAOS_PROFILE_NAME", "none");
        let artifact_prefix = PathBuf::from(env_or(
            "VPNNODE_CHAOS_ARTIFACT_PREFIX",
            &format!("docs/artifacts/chaos_matrix_{}", profile_name),
        ));
        let stage_trace_path = PathBuf::from(env_or(
            "VPNNODE_STAGE_TRACE_PATH",
            &format!("{}.stage.jsonl", artifact_prefix.display()),
        ));
        let temp_dir = std::env::temp_dir();
        Ok(Self {
            exact_route_cache_path: temp_dir.join(format!(
                "vpnnode-chaos-{}-exact-route-cache.json",
                profile_name
            )),
            adaptive_route_cache_path: temp_dir.join(format!(
                "vpnnode-chaos-{}-adaptive-route-cache.json",
                profile_name
            )),
            artifact_prefix,
            stage_trace_path,
            profile: ChaosProfileArtifact {
                profile_name,
                seed: env_parse_or("VPNNODE_TRANSPORT_CHAOS_SEED", 1_u64)?,
                skip_packets: env_parse_or("VPNNODE_TRANSPORT_CHAOS_SKIP_PACKETS", 0_u64)?,
                loss_ppm: env_parse_or("VPNNODE_TRANSPORT_CHAOS_LOSS_PPM", 0_u32)?,
                duplicate_ppm: env_parse_or("VPNNODE_TRANSPORT_CHAOS_DUPLICATE_PPM", 0_u32)?,
                reorder_ppm: env_parse_or("VPNNODE_TRANSPORT_CHAOS_REORDER_PPM", 0_u32)?,
                base_delay_ms: env_parse_or("VPNNODE_TRANSPORT_CHAOS_BASE_DELAY_MS", 0_u64)?,
                jitter_ms: env_parse_or("VPNNODE_TRANSPORT_CHAOS_JITTER_MS", 0_u64)?,
                reorder_extra_delay_ms: env_parse_or(
                    "VPNNODE_TRANSPORT_CHAOS_REORDER_EXTRA_DELAY_MS",
                    0_u64,
                )?,
                duplicate_delay_ms: env_parse_or(
                    "VPNNODE_TRANSPORT_CHAOS_DUPLICATE_DELAY_MS",
                    2_u64,
                )?,
                runs: env_parse_or("VPNNODE_CHAOS_RUNS", 5_usize)?,
                request_body_bytes: env_parse_or("VPNNODE_CHAOS_REQUEST_BODY_BYTES", 65_536_usize)?,
            },
            base_port: env_parse_or("VPNNODE_CHAOS_BASE_PORT", 19_300_u16)?,
            connect_timeout: Duration::from_secs(env_parse_or(
                "VPNNODE_CHAOS_CONNECT_TIMEOUT_SECS",
                5_u64,
            )?),
            write_timeout: Duration::from_secs(env_parse_or(
                "VPNNODE_CHAOS_WRITE_TIMEOUT_SECS",
                10_u64,
            )?),
            read_timeout: Duration::from_secs(env_parse_or(
                "VPNNODE_CHAOS_READ_TIMEOUT_SECS",
                45_u64,
            )?),
        })
    }

    fn combined_regression(profile_name: &str, base_port: u16, runs: usize) -> Self {
        let temp_dir = std::env::temp_dir();
        let artifact_prefix = temp_dir.join(format!("vpnnode-chaos-{}-artifacts", profile_name));
        let stage_trace_path = PathBuf::from(format!("{}.stage.jsonl", artifact_prefix.display()));
        Self {
            exact_route_cache_path: temp_dir.join(format!(
                "vpnnode-chaos-{}-exact-route-cache.json",
                profile_name
            )),
            adaptive_route_cache_path: temp_dir.join(format!(
                "vpnnode-chaos-{}-adaptive-route-cache.json",
                profile_name
            )),
            artifact_prefix,
            stage_trace_path,
            profile: ChaosProfileArtifact {
                profile_name: profile_name.to_string(),
                seed: 31,
                skip_packets: 24,
                loss_ppm: 1000,
                duplicate_ppm: 2000,
                reorder_ppm: 15000,
                base_delay_ms: 8,
                jitter_ms: 6,
                reorder_extra_delay_ms: 30,
                duplicate_delay_ms: 3,
                runs,
                request_body_bytes: 32_768,
            },
            base_port,
            connect_timeout: Duration::from_secs(5),
            write_timeout: Duration::from_secs(10),
            read_timeout: Duration::from_secs(45),
        }
    }

    fn mild_reorder_regression(profile_name: &str, base_port: u16, runs: usize) -> Self {
        let temp_dir = std::env::temp_dir();
        let artifact_prefix = temp_dir.join(format!("vpnnode-chaos-{}-artifacts", profile_name));
        let stage_trace_path = PathBuf::from(format!("{}.stage.jsonl", artifact_prefix.display()));
        Self {
            exact_route_cache_path: temp_dir.join(format!(
                "vpnnode-chaos-{}-exact-route-cache.json",
                profile_name
            )),
            adaptive_route_cache_path: temp_dir.join(format!(
                "vpnnode-chaos-{}-adaptive-route-cache.json",
                profile_name
            )),
            artifact_prefix,
            stage_trace_path,
            profile: ChaosProfileArtifact {
                profile_name: profile_name.to_string(),
                seed: 23,
                skip_packets: 24,
                loss_ppm: 0,
                duplicate_ppm: 0,
                reorder_ppm: 20_000,
                base_delay_ms: 0,
                jitter_ms: 0,
                reorder_extra_delay_ms: 40,
                duplicate_delay_ms: 2,
                runs,
                request_body_bytes: 32_768,
            },
            base_port,
            connect_timeout: Duration::from_secs(5),
            write_timeout: Duration::from_secs(10),
            read_timeout: Duration::from_secs(45),
        }
    }

    fn mild_delay_regression(profile_name: &str, base_port: u16, runs: usize) -> Self {
        let temp_dir = std::env::temp_dir();
        let artifact_prefix = temp_dir.join(format!("vpnnode-chaos-{}-artifacts", profile_name));
        let stage_trace_path = PathBuf::from(format!("{}.stage.jsonl", artifact_prefix.display()));
        Self {
            exact_route_cache_path: temp_dir.join(format!(
                "vpnnode-chaos-{}-exact-route-cache.json",
                profile_name
            )),
            adaptive_route_cache_path: temp_dir.join(format!(
                "vpnnode-chaos-{}-adaptive-route-cache.json",
                profile_name
            )),
            artifact_prefix,
            stage_trace_path,
            profile: ChaosProfileArtifact {
                profile_name: profile_name.to_string(),
                seed: 29,
                skip_packets: 24,
                loss_ppm: 0,
                duplicate_ppm: 0,
                reorder_ppm: 0,
                base_delay_ms: 4,
                jitter_ms: 2,
                reorder_extra_delay_ms: 0,
                duplicate_delay_ms: 2,
                runs,
                request_body_bytes: 32_768,
            },
            base_port,
            connect_timeout: Duration::from_secs(5),
            write_timeout: Duration::from_secs(10),
            read_timeout: Duration::from_secs(45),
        }
    }

    fn inflight_pressure_regression(profile_name: &str, base_port: u16, runs: usize) -> Self {
        let temp_dir = std::env::temp_dir();
        let artifact_prefix = temp_dir.join(format!("vpnnode-chaos-{}-artifacts", profile_name));
        let stage_trace_path = PathBuf::from(format!("{}.stage.jsonl", artifact_prefix.display()));
        Self {
            exact_route_cache_path: temp_dir.join(format!(
                "vpnnode-chaos-{}-exact-route-cache.json",
                profile_name
            )),
            adaptive_route_cache_path: temp_dir.join(format!(
                "vpnnode-chaos-{}-adaptive-route-cache.json",
                profile_name
            )),
            artifact_prefix,
            stage_trace_path,
            profile: ChaosProfileArtifact {
                profile_name: profile_name.to_string(),
                seed: 43,
                skip_packets: 24,
                loss_ppm: 0,
                duplicate_ppm: 0,
                reorder_ppm: 0,
                base_delay_ms: 22,
                jitter_ms: 6,
                reorder_extra_delay_ms: 0,
                duplicate_delay_ms: 2,
                runs,
                request_body_bytes: 262_144,
            },
            base_port,
            connect_timeout: Duration::from_secs(5),
            write_timeout: Duration::from_secs(10),
            read_timeout: Duration::from_secs(90),
        }
    }

    fn pacing_regression(profile_name: &str, base_port: u16, runs: usize) -> Self {
        let temp_dir = std::env::temp_dir();
        let artifact_prefix = temp_dir.join(format!("vpnnode-chaos-{}-artifacts", profile_name));
        let stage_trace_path = PathBuf::from(format!("{}.stage.jsonl", artifact_prefix.display()));
        Self {
            exact_route_cache_path: temp_dir.join(format!(
                "vpnnode-chaos-{}-exact-route-cache.json",
                profile_name
            )),
            adaptive_route_cache_path: temp_dir.join(format!(
                "vpnnode-chaos-{}-adaptive-route-cache.json",
                profile_name
            )),
            artifact_prefix,
            stage_trace_path,
            profile: ChaosProfileArtifact {
                profile_name: profile_name.to_string(),
                seed: 37,
                skip_packets: 24,
                loss_ppm: 0,
                duplicate_ppm: 0,
                reorder_ppm: 0,
                base_delay_ms: 12,
                jitter_ms: 4,
                reorder_extra_delay_ms: 0,
                duplicate_delay_ms: 2,
                runs,
                request_body_bytes: 32_768,
            },
            base_port,
            connect_timeout: Duration::from_secs(5),
            write_timeout: Duration::from_secs(10),
            read_timeout: Duration::from_secs(45),
        }
    }

    fn inflight_moderate_regression(profile_name: &str, base_port: u16, runs: usize) -> Self {
        let temp_dir = std::env::temp_dir();
        let artifact_prefix = temp_dir.join(format!("vpnnode-chaos-{}-artifacts", profile_name));
        let stage_trace_path = PathBuf::from(format!("{}.stage.jsonl", artifact_prefix.display()));
        Self {
            exact_route_cache_path: temp_dir.join(format!(
                "vpnnode-chaos-{}-exact-route-cache.json",
                profile_name
            )),
            adaptive_route_cache_path: temp_dir.join(format!(
                "vpnnode-chaos-{}-adaptive-route-cache.json",
                profile_name
            )),
            artifact_prefix,
            stage_trace_path,
            profile: ChaosProfileArtifact {
                profile_name: profile_name.to_string(),
                seed: 41,
                skip_packets: 24,
                loss_ppm: 0,
                duplicate_ppm: 0,
                reorder_ppm: 0,
                base_delay_ms: 12,
                jitter_ms: 4,
                reorder_extra_delay_ms: 0,
                duplicate_delay_ms: 2,
                runs,
                request_body_bytes: 262_144,
            },
            base_port,
            connect_timeout: Duration::from_secs(5),
            write_timeout: Duration::from_secs(10),
            read_timeout: Duration::from_secs(90),
        }
    }

    fn raw_path(&self) -> PathBuf {
        PathBuf::from(format!("{}.jsonl", self.artifact_prefix.display()))
    }

    fn decision_trace_path(&self) -> PathBuf {
        PathBuf::from(format!(
            "{}.decisions.jsonl",
            self.artifact_prefix.display()
        ))
    }

    fn report_path(&self) -> PathBuf {
        PathBuf::from(format!("{}.md", self.artifact_prefix.display()))
    }

    fn profile_json_path(&self) -> PathBuf {
        PathBuf::from(format!("{}.profile.json", self.artifact_prefix.display()))
    }
}

fn env_or(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_string())
}

fn env_parse_or<T>(name: &str, default: T) -> anyhow::Result<T>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match std::env::var(name) {
        Ok(value) => value
            .parse::<T>()
            .map_err(|err| anyhow!("parse {name}={value}: {err}")),
        Err(_) => Ok(default),
    }
}
