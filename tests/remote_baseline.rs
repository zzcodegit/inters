#[path = "helpers/mod.rs"]
mod helpers;
#[path = "baseline/support.rs"]
mod support;

use anyhow::Context;

use support::{BaselineMode, RemoteBaselineConfig};

#[tokio::test]
async fn remote_http_smoke_requires_usable_path() -> anyhow::Result<()> {
    support::prepare_baseline(BaselineMode::Remote);

    let config = RemoteBaselineConfig::from_env()?;
    let request = support::http_get_request(&config.http_host);

    eprintln!(
        "baseline_mode=remote local_listen={} route_length={} target_scheme={} expected_status={} route_chain={}",
        config.local_listen,
        config.route_length,
        config.target_scheme,
        config.expected_ready_status,
        config.route_chain()
    );

    support::spawn_remote_client(&config);
    helpers::wait_tcp_listener(config.local_listen)
        .await
        .context("remote smoke: local client TCP listener must come up")?;

    let probe = helpers::wait_http_status_with_request(
        config.local_listen,
        &request,
        &[config.expected_ready_status],
    )
    .await
    .with_context(|| {
        format!(
            "remote smoke: usable HTTP path was not observed on {} via {}",
            config.local_listen,
            config.route_chain()
        )
    })?;

    eprintln!(
        "remote_smoke observed_status={} response_bytes={} route_chain={}",
        probe.status_code,
        probe.response.len(),
        config.route_chain()
    );
    Ok(())
}
