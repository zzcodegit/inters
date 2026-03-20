use clap::Parser;
use std::collections::HashMap;
use std::fs;
use std::net::TcpStream;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(name = "vpnnode-validate", about = "Validation Track V1 data collection")]
struct CliArgs {
    /// Sites to test, comma-separated.
    #[arg(long, value_delimiter = ',')]
    sites: Vec<String>,

    /// CSV output path.
    #[arg(long)]
    output: PathBuf,

    /// Client local listen port where curl should connect for overlay tests.
    #[arg(long)]
    overlay_port: u16,

    /// File path containing vpnnode logs (must include VALIDATION_ARTIFACT lines).
    #[arg(long)]
    log_file: Option<PathBuf>,

    /// Curl request path (default: "/").
    #[arg(long, default_value = "/")]
    request_path: String,

    /// Wait after overlay request before reading logs.
    #[arg(long, default_value_t = 350)]
    log_wait_ms: u64,

    /// URL scheme for curl. Use `http` for the built-in target service.
    #[arg(long, default_value = "http")]
    scheme: String,

    /// Port used in curl URL and --connect-to (e.g. 80 for http, 443 for https).
    #[arg(long, default_value_t = 80)]
    port: u16,

    /// If true, refuse to run overlay tests when local client TCP port is not listening.
    #[arg(long, default_value_t = true)]
    require_client_port: bool,
}

fn csv_escape(v: &str) -> String {
    if v.contains(',') || v.contains('"') || v.contains('\n') || v.contains('\r') {
        format!("\"{}\"", v.replace('"', "\"\""))
    } else {
        v.to_string()
    }
}

fn parse_key_value_line(line: &str) -> HashMap<String, String> {
    let mut out = HashMap::<String, String>::new();
    // Example: connect=0.01 appconnect=0.80 starttransfer=1.85 total=1.90 code=200
    for part in line.split_whitespace() {
        if let Some((k, v)) = part.split_once('=') {
            out.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    out
}

fn run_curl_metrics(
    site: &str,
    overlay: bool,
    overlay_port: u16,
    request_path: &str,
    scheme: &str,
    port: u16,
) -> anyhow::Result<HashMap<String, String>> {
    let dev_null = if cfg!(windows) { "NUL" } else { "/dev/null" };
    let path = if request_path.starts_with('/') {
        request_path.to_string()
    } else {
        format!("/{}", request_path)
    };
    // Avoid double-slash URLs when request_path == "/".
    // With the previous logic `path.trim_start_matches('/')` becomes "", resulting in `...://host:port//`.
    let trimmed = path.trim_start_matches('/');
    let url = if trimmed.is_empty() {
        format!("{}://{}:{}/", scheme, site, port)
    } else {
        format!("{}://{}:{}/{}", scheme, site, port, trimmed)
    };
    let mut cmd = Command::new("curl");
    cmd.arg("-sk") // silent + insecure (ignore cert validation)
        // Bound curl runtime so `vpnnode-validate` can't hang indefinitely
        // on slow/unreachable overlay/exit targets, but keep enough time
        // for full TLS/HTTP bodies.
        .arg("--connect-timeout")
        .arg("15")
        .arg("--max-time")
        .arg("60")
        .arg("-o")
        .arg(dev_null)
        .arg("-w")
        .arg("connect=%{time_connect} appconnect=%{time_appconnect} starttransfer=%{time_starttransfer} total=%{time_total} code=%{http_code}\n");

    if overlay {
        let connect_to = format!("{}:{}:127.0.0.1:{}", site, port, overlay_port);
        cmd.arg("--connect-to").arg(connect_to);
    }

    cmd.arg(url);

    let output = cmd.output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    // Be tolerant: find the first non-empty line.
    let metrics_line = stdout
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
        .trim();

    Ok(parse_key_value_line(metrics_line))
}

fn read_new_log_text(log_file: &Path, start_offset: u64) -> anyhow::Result<String> {
    let mut f = fs::File::open(log_file)?;
    let file_len = f.metadata()?.len();
    let offset = if start_offset <= file_len { start_offset } else { 0 };
    f.seek(SeekFrom::Start(offset))?;
    let mut buf = String::new();
    f.read_to_string(&mut buf)?;
    Ok(buf)
}

fn parse_validation_artifact_line(line: &str) -> Option<HashMap<String, String>> {
    let idx = line.find("VALIDATION_ARTIFACT,")?;
    let s = &line[idx..];
    let parts: Vec<&str> = s.trim().split(',').collect();
    if parts.is_empty() || parts[0] != "VALIDATION_ARTIFACT" {
        return None;
    }
    let mut out = HashMap::<String, String>::new();
    for p in parts.into_iter().skip(1) {
        let (k, v) = p.split_once('=')?;
        out.insert(k.trim().to_string(), v.trim().to_string());
    }
    Some(out)
}

fn find_artifact_for_site_in_text(
    text: &str,
    target_site: &str,
) -> Option<HashMap<String, String>> {
    // Scan from end (latest first), but match the requested site.
    for line in text.lines().rev() {
        if let Some(m) = parse_validation_artifact_line(line) {
            if m.get("site").map(|s| s == target_site).unwrap_or(false) {
                return Some(m);
            }
        }
    }
    None
}

fn get_field(map: &HashMap<String, String>, key: &str) -> String {
    map.get(key).cloned().unwrap_or_default()
}

fn main() -> anyhow::Result<()> {
    let args = CliArgs::parse();
    if args.sites.is_empty() {
        anyhow::bail!("--sites must not be empty");
    }

    // CSV header.
    let headers = vec![
        "site",
        "mode",
        "exit",
        "route",
        "connect",
        "appconnect",
        "starttransfer",
        "total",
        "code",
        "resp_bytes",
        "frames_sent",
        "avg_inflight",
        "max_inflight",
        "retransmit_rate",
        "throughput_bps",
        "ack_latency_ms_avg",
    ];

    let mut csv = String::new();
    csv.push_str(&headers.join(","));
    csv.push('\n');

    for site in args.sites.iter() {
        // 1) Direct
        let direct_metrics = run_curl_metrics(
            site,
            false,
            args.overlay_port,
            &args.request_path,
            &args.scheme,
            args.port,
        )?;
        let direct_row = vec![
            site.to_string(),
            "direct".to_string(),
            "".to_string(), // exit
            "".to_string(), // route
            direct_metrics.get("connect").cloned().unwrap_or_default(),
            direct_metrics.get("appconnect").cloned().unwrap_or_default(),
            direct_metrics.get("starttransfer").cloned().unwrap_or_default(),
            direct_metrics.get("total").cloned().unwrap_or_default(),
            direct_metrics.get("code").cloned().unwrap_or_default(),
            "".to_string(), // resp_bytes
            "".to_string(), // frames_sent
            "".to_string(), // avg_inflight
            "".to_string(), // max_inflight
            "".to_string(), // retransmit_rate
            "".to_string(), // throughput_bps
            "".to_string(), // ack_latency_ms_avg
        ];
        csv.push_str(
            &direct_row
                .iter()
                .map(|v| csv_escape(v))
                .collect::<Vec<_>>()
                .join(","),
        );
        csv.push('\n');

        // 2) Overlay
        if args.require_client_port {
            let addr = format!("127.0.0.1:{}", args.overlay_port);
            match TcpStream::connect_timeout(
                &addr.parse().unwrap(),
                Duration::from_millis(300),
            ) {
                Ok(_) => {}
                Err(e) => {
                    anyhow::bail!(
                        "overlay precheck failed: local client TCP {} not reachable: {e}. \
Re-check that vpnnode client tcp-listener is running and handshake succeeded.",
                        addr
                    );
                }
            }
        }

        let artifact_start_offset = args
            .log_file
            .as_ref()
            .and_then(|p| fs::metadata(p).map(|m| m.len()).ok())
            .unwrap_or(0);

        let _overlay_metrics = run_curl_metrics(
            site,
            true,
            args.overlay_port,
            &args.request_path,
            &args.scheme,
            args.port,
        )?;

        // Read appended logs (best-effort, but match the requested site).
        if let Some(ref log_path) = args.log_file {
            let mut artifact: Option<HashMap<String, String>> = None;
            let deadline = std::time::Instant::now() + Duration::from_millis(args.log_wait_ms);
            while std::time::Instant::now() < deadline {
                let new_text = read_new_log_text(log_path, artifact_start_offset)?;
                artifact = find_artifact_for_site_in_text(&new_text, site);
                if artifact.is_some() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(200));
            }

            let overlay_metrics = _overlay_metrics; // moved below
            let (exit, route, resp_bytes, frames_sent, avg_inflight, max_inflight, retransmit_rate, throughput_bps, ack_latency_ms_avg) =
                if let Some(ref m) = artifact {
                    (
                        get_field(m, "exit"),
                        get_field(m, "route"),
                        get_field(m, "resp_bytes"),
                        get_field(m, "frames_sent"),
                        get_field(m, "avg_inflight"),
                        get_field(m, "max_inflight"),
                        get_field(m, "retransmit_rate"),
                        get_field(m, "throughput_bps"),
                        get_field(m, "ack_latency_ms_avg"),
                    )
                } else {
                    (
                        "".to_string(),
                        "".to_string(),
                        "".to_string(),
                        "".to_string(),
                        "".to_string(),
                        "".to_string(),
                        "".to_string(),
                        "".to_string(),
                        "".to_string(),
                    )
                };

            let overlay_row = vec![
                site.to_string(),
                "overlay".to_string(),
                exit,
                route,
                overlay_metrics.get("connect").cloned().unwrap_or_default(),
                overlay_metrics.get("appconnect").cloned().unwrap_or_default(),
                overlay_metrics.get("starttransfer").cloned().unwrap_or_default(),
                overlay_metrics.get("total").cloned().unwrap_or_default(),
                overlay_metrics.get("code").cloned().unwrap_or_default(),
                resp_bytes,
                frames_sent,
                avg_inflight,
                max_inflight,
                retransmit_rate,
                throughput_bps,
                ack_latency_ms_avg,
            ];

            csv.push_str(
                &overlay_row
                    .iter()
                    .map(|v| csv_escape(v))
                    .collect::<Vec<_>>()
                    .join(","),
            );
            csv.push('\n');
        } else {
            // Without log-file we can still fill curl metrics, but artifact-specific columns remain blank.
            let overlay_metrics = _overlay_metrics;
            let overlay_row = vec![
                site.to_string(),
                "overlay".to_string(),
                "".to_string(),
                "".to_string(),
                overlay_metrics.get("connect").cloned().unwrap_or_default(),
                overlay_metrics.get("appconnect").cloned().unwrap_or_default(),
                overlay_metrics.get("starttransfer").cloned().unwrap_or_default(),
                overlay_metrics.get("total").cloned().unwrap_or_default(),
                overlay_metrics.get("code").cloned().unwrap_or_default(),
                "".to_string(),
                "".to_string(),
                "".to_string(),
                "".to_string(),
                "".to_string(),
                "".to_string(),
                "".to_string(),
            ];
                csv.push_str(
                    &overlay_row
                        .iter()
                        .map(|v| csv_escape(v))
                        .collect::<Vec<_>>()
                        .join(","),
                );
            csv.push('\n');
        }
    }

    fs::write(&args.output, csv)?;
    Ok(())
}

