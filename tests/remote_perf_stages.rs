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
struct StagePerfConfig {
    remote: RemoteBaselineConfig,
    direct_addr: SocketAddr,
    request_path: String,
    runs: usize,
    local_raw_path: PathBuf,
    client_stage_path: PathBuf,
    connect_timeout: Duration,
    write_timeout: Duration,
    read_timeout: Duration,
}

#[derive(Debug, Clone)]
struct ScenarioSpec {
    name: &'static str,
    route_length: Option<u8>,
    local_listen: Option<SocketAddr>,
    route_cache_path: Option<&'static str>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "record_type", rename_all = "snake_case")]
enum StageRecord {
    Warmup {
        scenario: String,
        route_length: u8,
        route_chain: String,
        endpoint: String,
        listener_ready_ms: Option<f64>,
        route_ready_ms: Option<f64>,
    },
    Measurement {
        scenario: String,
        run_index: usize,
        route_length: u8,
        route_chain: String,
        endpoint: String,
        request_host: String,
        request_path: String,
        status_code: Option<u16>,
        connect_time_ms: Option<f64>,
        ttfb_ms: Option<f64>,
        total_time_ms: Option<f64>,
        response_bytes: Option<usize>,
        body_bytes: Option<usize>,
        effective_throughput_bps: Option<f64>,
        error: Option<String>,
    },
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
async fn remote_perf_stage_matrix_collects_measurements() -> anyhow::Result<()> {
    support::prepare_baseline(BaselineMode::Remote);

    let config = StagePerfConfig::from_env()?;
    prepare_output_path(&config.local_raw_path)?;
    prepare_output_path(&config.client_stage_path)?;
    fs::write(&config.local_raw_path, b"").context("truncate local raw output file")?;
    fs::write(&config.client_stage_path, b"").context("truncate client stage output file")?;
    std::env::set_var(
        "VPNNODE_STAGE_TRACE_PATH",
        config.client_stage_path.as_os_str(),
    );

    let scenarios = vec![
        ScenarioSpec {
            name: "direct",
            route_length: None,
            local_listen: None,
            route_cache_path: None,
        },
        ScenarioSpec {
            name: "remote-1hop",
            route_length: Some(1),
            local_listen: Some("127.0.0.1:19281".parse().unwrap()),
            route_cache_path: Some("route_cache_perf_stage_1hop.json"),
        },
        ScenarioSpec {
            name: "remote-2hop",
            route_length: Some(2),
            local_listen: Some("127.0.0.1:19282".parse().unwrap()),
            route_cache_path: Some("route_cache_perf_stage_2hop.json"),
        },
        ScenarioSpec {
            name: "remote-3hop",
            route_length: Some(3),
            local_listen: Some("127.0.0.1:19283".parse().unwrap()),
            route_cache_path: Some("route_cache_perf_stage_3hop.json"),
        },
    ];

    for scenario in scenarios {
        if let Some(route_length) = scenario.route_length {
            run_reset_if_configured()?;
            let remote_config = config.remote.for_scenario(
                route_length,
                scenario.local_listen.unwrap(),
                scenario.route_cache_path.unwrap(),
                format!("stage-{}", scenario.name),
            )?;
            let route_chain = remote_config.route_chain();
            let endpoint = remote_config.local_listen.to_string();

            let scenario_start = Instant::now();
            let mut client = support::spawn_remote_client(&remote_config)?;
            helpers::wait_tcp_listener(remote_config.local_listen)
                .await
                .with_context(|| format!("{}: listener did not start", scenario.name))?;
            let listener_ready_ms = scenario_start.elapsed().as_secs_f64() * 1000.0;

            let warmup_host = format!("stage-{}-warmup", scenario.name);
            let warmup_request = support::http_get_request_path(&warmup_host, &config.request_path);
            helpers::wait_http_status_with_request_options(
                remote_config.local_listen,
                &warmup_request,
                &[remote_config.expected_ready_status],
                helpers::HttpProbeOptions {
                    ready_timeout: Duration::from_secs(remote_config.ready_timeout_secs),
                    connect_timeout: config.connect_timeout,
                    probe_io_timeout: Duration::from_secs(remote_config.probe_attempt_timeout_secs),
                    probe_attempt_timeout: Duration::from_secs(
                        remote_config.probe_attempt_timeout_secs,
                    ),
                    retry_delay: Duration::from_millis(500),
                },
            )
            .await
            .with_context(|| format!("{}: route did not become ready", scenario.name))?;
            let route_ready_ms = scenario_start.elapsed().as_secs_f64() * 1000.0;

            append_record(
                &config.local_raw_path,
                &StageRecord::Warmup {
                    scenario: scenario.name.to_string(),
                    route_length,
                    route_chain: route_chain.clone(),
                    endpoint: endpoint.clone(),
                    listener_ready_ms: Some(listener_ready_ms),
                    route_ready_ms: Some(route_ready_ms),
                },
            )?;

            for run_index in 1..=config.runs {
                let request_host = format!("stage-{}-run{}", scenario.name, run_index);
                let request = support::http_get_request_path(&request_host, &config.request_path);
                let record =
                    match do_measure_http_request(remote_config.local_listen, &request, &config)
                        .await
                    {
                        Ok(result) => StageRecord::Measurement {
                            scenario: scenario.name.to_string(),
                            run_index,
                            route_length,
                            route_chain: route_chain.clone(),
                            endpoint: endpoint.clone(),
                            request_host,
                            request_path: config.request_path.clone(),
                            status_code: Some(result.status_code),
                            connect_time_ms: Some(result.connect_time_ms),
                            ttfb_ms: Some(result.ttfb_ms),
                            total_time_ms: Some(result.total_time_ms),
                            response_bytes: Some(result.response_bytes),
                            body_bytes: Some(result.body_bytes),
                            effective_throughput_bps: Some(result.effective_throughput_bps),
                            error: None,
                        },
                        Err(err) => StageRecord::Measurement {
                            scenario: scenario.name.to_string(),
                            run_index,
                            route_length,
                            route_chain: route_chain.clone(),
                            endpoint: endpoint.clone(),
                            request_host,
                            request_path: config.request_path.clone(),
                            status_code: None,
                            connect_time_ms: None,
                            ttfb_ms: None,
                            total_time_ms: None,
                            response_bytes: None,
                            body_bytes: None,
                            effective_throughput_bps: None,
                            error: Some(err.to_string()),
                        },
                    };
                append_record(&config.local_raw_path, &record)?;
                if let StageRecord::Measurement {
                    error: Some(error), ..
                } = &record
                {
                    client.stop();
                    bail!("{} run {} failed: {error}", scenario.name, run_index);
                }
            }

            client.stop();
        } else {
            let route_chain = format!("client -> {} -> target", config.direct_addr);
            let endpoint = config.direct_addr.to_string();
            append_record(
                &config.local_raw_path,
                &StageRecord::Warmup {
                    scenario: scenario.name.to_string(),
                    route_length: 0,
                    route_chain: route_chain.clone(),
                    endpoint: endpoint.clone(),
                    listener_ready_ms: None,
                    route_ready_ms: None,
                },
            )?;
            for run_index in 1..=config.runs {
                let request_host = format!("stage-{}-run{}", scenario.name, run_index);
                let request = support::http_get_request_path(&request_host, &config.request_path);
                let record =
                    match do_measure_http_request(config.direct_addr, &request, &config).await {
                        Ok(result) => StageRecord::Measurement {
                            scenario: scenario.name.to_string(),
                            run_index,
                            route_length: 0,
                            route_chain: route_chain.clone(),
                            endpoint: endpoint.clone(),
                            request_host,
                            request_path: config.request_path.clone(),
                            status_code: Some(result.status_code),
                            connect_time_ms: Some(result.connect_time_ms),
                            ttfb_ms: Some(result.ttfb_ms),
                            total_time_ms: Some(result.total_time_ms),
                            response_bytes: Some(result.response_bytes),
                            body_bytes: Some(result.body_bytes),
                            effective_throughput_bps: Some(result.effective_throughput_bps),
                            error: None,
                        },
                        Err(err) => StageRecord::Measurement {
                            scenario: scenario.name.to_string(),
                            run_index,
                            route_length: 0,
                            route_chain: route_chain.clone(),
                            endpoint: endpoint.clone(),
                            request_host,
                            request_path: config.request_path.clone(),
                            status_code: None,
                            connect_time_ms: None,
                            ttfb_ms: None,
                            total_time_ms: None,
                            response_bytes: None,
                            body_bytes: None,
                            effective_throughput_bps: None,
                            error: Some(err.to_string()),
                        },
                    };
                append_record(&config.local_raw_path, &record)?;
                if let StageRecord::Measurement {
                    error: Some(error), ..
                } = &record
                {
                    bail!("{} run {} failed: {error}", scenario.name, run_index);
                }
            }
        }
    }

    Ok(())
}

async fn do_measure_http_request(
    addr: SocketAddr,
    request: &[u8],
    config: &StagePerfConfig,
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

fn append_record(path: &Path, record: &StageRecord) -> anyhow::Result<()> {
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .with_context(|| format!("open stage raw output {}", path.display()))?;
    serde_json::to_writer(&mut file, record).context("serialize stage perf record")?;
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

impl StagePerfConfig {
    fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            remote: RemoteBaselineConfig::from_env()?,
            direct_addr: required_env("VPNNODE_PERF_DIRECT_ADDR")?
                .parse()
                .map_err(|e| anyhow!("failed to parse VPNNODE_PERF_DIRECT_ADDR: {e}"))?,
            request_path: env_or_default("VPNNODE_PERF_REQUEST_PATH", "/perf-262144.bin"),
            runs: parse_env_or_default("VPNNODE_PERF_RUNS", 5_usize)?,
            local_raw_path: PathBuf::from(env_or_default(
                "VPNNODE_PERF_STAGE_LOCAL_RAW_PATH",
                "docs/artifacts/remote_perf_stage_matrix_2026-03-21.local.jsonl",
            )),
            client_stage_path: PathBuf::from(env_or_default(
                "VPNNODE_STAGE_TRACE_PATH",
                "docs/artifacts/remote_perf_stage_matrix_2026-03-21.client.jsonl",
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
