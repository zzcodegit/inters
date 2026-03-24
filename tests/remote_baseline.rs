#[path = "helpers/mod.rs"]
mod helpers;
#[path = "baseline/support.rs"]
mod support;

use std::net::SocketAddr;
use std::time::Instant;

use anyhow::Context;
use tokio::time::Duration;

use support::{BaselineMode, RemoteBaselineConfig};

async fn run_remote_http_scenario(
    scenario_name: &str,
    route_length: u8,
    local_listen: SocketAddr,
    route_cache_path: &str,
) -> anyhow::Result<()> {
    support::prepare_baseline(BaselineMode::Remote);

    let base = RemoteBaselineConfig::from_env()?;
    let scenario_host = format!("remote-{scenario_name}");
    let config = base.for_scenario(
        route_length,
        local_listen,
        route_cache_path,
        scenario_host.clone(),
    )?;
    let request = support::http_get_request(&config.http_host);
    let start = Instant::now();

    eprintln!(
        "remote_scenario={} local_listen={} route_length={} exact_route_only={} target_scheme={} expected_status={} ready_timeout_secs={} probe_attempt_timeout_secs={} route_chain={} http_host={}",
        scenario_name,
        config.local_listen,
        config.route_length,
        config.exact_route_only,
        config.target_scheme,
        config.expected_ready_status,
        config.ready_timeout_secs,
        config.probe_attempt_timeout_secs,
        config.route_chain(),
        config.http_host
    );

    let mut client = support::spawn_remote_client(&config)?;
    helpers::wait_tcp_listener(config.local_listen)
        .await
        .with_context(|| format!("{scenario_name}: local client TCP listener must come up"))?;

    let probe = helpers::wait_http_status_with_request_options(
        config.local_listen,
        &request,
        &[config.expected_ready_status],
        helpers::HttpProbeOptions {
            ready_timeout: Duration::from_secs(config.ready_timeout_secs),
            connect_timeout: Duration::from_secs(3),
            probe_io_timeout: Duration::from_secs(config.probe_attempt_timeout_secs),
            probe_attempt_timeout: Duration::from_secs(config.probe_attempt_timeout_secs),
            retry_delay: Duration::from_millis(500),
        },
    )
    .await
    .with_context(|| {
        format!(
            "{scenario_name}: usable HTTP path was not observed on {} via {}",
            config.local_listen,
            config.route_chain()
        )
    })?;

    eprintln!(
        "remote_scenario={} observed_status={} response_bytes={} duration_ms={} route_chain={} http_host={}",
        scenario_name,
        probe.status_code,
        probe.response.len(),
        start.elapsed().as_millis(),
        config.route_chain(),
        config.http_host
    );
    client.stop();
    Ok(())
}

fn scenario_listen(route_length: u8) -> SocketAddr {
    format!("127.0.0.1:1908{}", route_length)
        .parse()
        .expect("scenario listen addr")
}

fn scenario_cache(route_length: u8) -> String {
    format!("route_cache_remote_{}hop.json", route_length)
}

#[tokio::test]
async fn remote_http_1hop_returns_200() -> anyhow::Result<()> {
    run_remote_http_scenario(
        "1hop",
        1,
        scenario_listen(1),
        &scenario_cache(1),
    )
    .await
}

#[tokio::test]
async fn remote_http_2hop_returns_200() -> anyhow::Result<()> {
    run_remote_http_scenario(
        "2hop",
        2,
        scenario_listen(2),
        &scenario_cache(2),
    )
    .await
}

#[tokio::test]
async fn remote_http_3hop_returns_200() -> anyhow::Result<()> {
    run_remote_http_scenario(
        "3hop",
        3,
        scenario_listen(3),
        &scenario_cache(3),
    )
    .await
}

#[tokio::test]
async fn remote_http_5hop_returns_200() -> anyhow::Result<()> {
    let base = RemoteBaselineConfig::from_env()?;
    if base.route_length < 5 {
        eprintln!("remote_http_5hop_returns_200 skipped: configured route_length < 5");
        return Ok(());
    }
    run_remote_http_scenario("5hop", 5, scenario_listen(5), &scenario_cache(5)).await
}
