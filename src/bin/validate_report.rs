use clap::Parser;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "vpnnode-validate-report", about = "Summarize Validation Track V1 CSV results")]
struct CliArgs {
    #[arg(long)]
    input: PathBuf,
    #[arg(long)]
    output: PathBuf,

    /// Also print a human-readable console summary.
    #[arg(long, default_value_t = true)]
    console: bool,
}

#[derive(Default, Debug, Clone)]
struct AvgAgg {
    n: u64,
    sum: f64,
}

impl AvgAgg {
    fn push(&mut self, v: f64) {
        self.n += 1;
        self.sum += v;
    }

    fn avg(&self) -> Option<f64> {
        if self.n == 0 {
            None
        } else {
            Some(self.sum / self.n as f64)
        }
    }
}

#[derive(Default, Debug, Clone)]
struct SuccessAgg {
    code_present_n: u64,
    success_n: u64,
}

impl SuccessAgg {
    fn push_code(&mut self, code: Option<u16>) {
        if let Some(c) = code {
            self.code_present_n += 1;
            let ok = c < 400 && c > 0;
            if ok {
                self.success_n += 1;
            }
        }
    }

    fn success_rate(&self) -> Option<f64> {
        if self.code_present_n == 0 {
            None
        } else {
            Some(self.success_n as f64 / self.code_present_n as f64)
        }
    }
}

#[derive(Default, Debug, Clone)]
struct DirectAgg {
    connect: AvgAgg,
    appconnect: AvgAgg,
    starttransfer: AvgAgg,
    total: AvgAgg,
    success: SuccessAgg,
}

#[derive(Default, Debug, Clone)]
struct OverlayAgg {
    connect: AvgAgg,
    appconnect: AvgAgg,
    starttransfer: AvgAgg,
    total: AvgAgg,
    resp_bytes: AvgAgg,
    avg_inflight: AvgAgg,
    max_inflight: AvgAgg,
    retransmit_rate: AvgAgg,
    throughput_bps: AvgAgg,
    ack_latency_ms_avg: AvgAgg,
    success: SuccessAgg,
}

fn parse_csv(content: &str) -> Vec<Vec<String>> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut chars = content.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                if in_quotes && chars.peek() == Some(&'"') {
                    // Escaped quote.
                    cur.push('"');
                    let _ = chars.next();
                } else {
                    in_quotes = !in_quotes;
                }
            }
            ',' if !in_quotes => {
                row.push(cur.clone());
                cur.clear();
            }
            '\n' if !in_quotes => {
                row.push(cur.clone());
                cur.clear();
                // Trim trailing \r on the last field.
                if let Some(last) = row.last_mut() {
                    if last.ends_with('\r') {
                        last.truncate(last.len() - 1);
                    }
                }
                rows.push(row);
                row = Vec::new();
            }
            _ => cur.push(ch),
        }
    }

    // Flush last row.
    if !cur.is_empty() || !row.is_empty() {
        row.push(cur);
        if let Some(last) = row.last_mut() {
            if last.ends_with('\r') {
                last.truncate(last.len() - 1);
            }
        }
        rows.push(row);
    }

    // Drop completely empty rows.
    rows.into_iter()
        .filter(|r| !r.iter().all(|c| c.trim().is_empty()))
        .collect()
}

fn idx_map(header: &[String]) -> HashMap<String, usize> {
    let mut m = HashMap::new();
    for (i, h) in header.iter().enumerate() {
        m.insert(h.trim().to_string(), i);
    }
    m
}

fn get_field<'a>(
    row: &'a [String],
    idx: &'a HashMap<String, usize>,
    key: &str,
) -> Option<&'a str> {
    let i = *idx.get(key)?;
    row.get(i).map(|s| s.trim()).filter(|s| !s.is_empty())
}

fn parse_f64(s: Option<&str>) -> Option<f64> {
    let s = s?;
    s.parse::<f64>().ok()
}

fn parse_u16(s: Option<&str>) -> Option<u16> {
    let s = s?;
    s.parse::<u16>().ok()
}

fn overhead_pct(direct_avg: f64, overlay_avg: f64) -> f64 {
    ((overlay_avg - direct_avg) / direct_avg) * 100.0
}

fn fmt_opt_f64(v: Option<f64>) -> String {
    match v {
        None => String::new(),
        Some(x) => {
            if !x.is_finite() {
                return String::new();
            }
            let s = format!("{:.6}", x);
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        }
    }
}

fn csv_escape(v: &str) -> String {
    if v.contains(',') || v.contains('"') || v.contains('\n') || v.contains('\r') {
        format!("\"{}\"", v.replace('"', "\"\""))
    } else {
        v.to_string()
    }
}

fn main() -> anyhow::Result<()> {
    let args = CliArgs::parse();
    let content = fs::read_to_string(&args.input)?;
    let rows = parse_csv(&content);
    if rows.len() < 2 {
        anyhow::bail!("input CSV has no data rows");
    }

    let header = rows[0].clone();
    let idx = idx_map(&header);
    let required = [
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
    for k in required.iter() {
        if !idx.contains_key(*k) {
            anyhow::bail!("missing required CSV column: {k}");
        }
    }

    let mut direct_by_site: HashMap<String, DirectAgg> = HashMap::new();
    let mut overlay_by_site_exit: HashMap<(String, String), OverlayAgg> = HashMap::new();

    for r in rows.iter().skip(1) {
        let site = match get_field(r, &idx, "site") {
            Some(s) => s.to_string(),
            None => continue,
        };
        let mode = match get_field(r, &idx, "mode") {
            Some(s) => s,
            None => continue,
        };

        let code = parse_u16(get_field(r, &idx, "code"));

        if mode == "direct" {
            let entry = direct_by_site.entry(site).or_default();
            if let Some(v) = parse_f64(get_field(r, &idx, "connect")) {
                entry.connect.push(v);
            }
            if let Some(v) = parse_f64(get_field(r, &idx, "appconnect")) {
                entry.appconnect.push(v);
            }
            if let Some(v) = parse_f64(get_field(r, &idx, "starttransfer")) {
                entry.starttransfer.push(v);
            }
            if let Some(v) = parse_f64(get_field(r, &idx, "total")) {
                entry.total.push(v);
            }
            entry.success.push_code(code);
        } else if mode == "overlay" {
            let exit = get_field(r, &idx, "exit").unwrap_or("").to_string();
            let key = (site.clone(), exit.clone());
            let entry = overlay_by_site_exit.entry(key).or_insert_with(|| OverlayAgg {
                ..Default::default()
            });

            // Some artifact fields can be blank if logs missing; tolerate it.
            if let Some(v) = parse_f64(get_field(r, &idx, "connect")) {
                entry.connect.push(v);
            }
            if let Some(v) = parse_f64(get_field(r, &idx, "appconnect")) {
                entry.appconnect.push(v);
            }
            if let Some(v) = parse_f64(get_field(r, &idx, "starttransfer")) {
                entry.starttransfer.push(v);
            }
            if let Some(v) = parse_f64(get_field(r, &idx, "total")) {
                entry.total.push(v);
            }
            if let Some(v) = parse_f64(get_field(r, &idx, "resp_bytes")) {
                entry.resp_bytes.push(v);
            }
            if let Some(v) = parse_f64(get_field(r, &idx, "avg_inflight")) {
                entry.avg_inflight.push(v);
            }
            if let Some(v) = parse_f64(get_field(r, &idx, "max_inflight")) {
                entry.max_inflight.push(v);
            }
            if let Some(v) = parse_f64(get_field(r, &idx, "retransmit_rate")) {
                entry.retransmit_rate.push(v);
            }
            if let Some(v) = parse_f64(get_field(r, &idx, "throughput_bps")) {
                entry.throughput_bps.push(v);
            }
            if let Some(v) = parse_f64(get_field(r, &idx, "ack_latency_ms_avg")) {
                entry.ack_latency_ms_avg.push(v);
            }
            entry.success.push_code(code);
        }
    }

    // Decide best exit candidates per site.
    let mut candidate_best_exit: HashMap<String, String> = HashMap::new();
    for (site, direct) in direct_by_site.iter() {
        let mut candidates: Vec<(&String, &OverlayAgg)> = overlay_by_site_exit
            .iter()
            .filter(|((s, _exit), _agg)| s == site)
            .map(|((_, exit), agg)| (exit, agg))
            .collect();

        candidates.sort_by(|a, b| {
            let a_total = a.1.total.avg().unwrap_or(f64::INFINITY);
            let b_total = b.1.total.avg().unwrap_or(f64::INFINITY);
            a_total.partial_cmp(&b_total).unwrap_or(std::cmp::Ordering::Equal)
        });

        // "successful exits" => success_rate >= 0.95 (consistent with low_success flag).
        let best = candidates.iter().find(|(_, agg)| {
            agg.success.success_rate().unwrap_or(0.0) >= 0.95
                && agg.total.avg().is_some()
        });

        if let Some((exit, _agg)) = best {
            candidate_best_exit.insert(site.clone(), (*exit).clone());
        } else if candidates.first().is_some() && direct.total.avg().is_some() {
            // If no exit meets success threshold, keep candidate empty (all false).
        }
    }

    // Prepare summary rows.
    let out_headers = vec![
        "site",
        "exit",
        "direct_total_avg",
        "overlay_total_avg",
        "overhead_total_pct",
        "direct_starttransfer_avg",
        "overlay_starttransfer_avg",
        "overhead_starttransfer_pct",
        "overlay_connect_avg",
        "overlay_appconnect_avg",
        "success_rate",
        "avg_inflight",
        "max_inflight",
        "retransmit_rate",
        "throughput_bps",
        "ack_latency_ms_avg",
        "low_window_utilization",
        "high_retransmit",
        "low_success",
        "candidate_best_exit",
    ];

    let mut summary_csv = String::new();
    summary_csv.push_str(&out_headers.join(","));
    summary_csv.push('\n');

    // For console output: iterate sites, then exits sorted by avg total.
    let mut sites: Vec<String> = direct_by_site.keys().cloned().collect();
    sites.sort();

    for site in sites {
        let direct = direct_by_site.get(&site).cloned().unwrap_or_default();
        let direct_total_avg = direct.total.avg();
        let direct_starttransfer_avg = direct.starttransfer.avg();

        let mut overlay_groups: Vec<((String, String), OverlayAgg)> = overlay_by_site_exit
            .iter()
            .filter(|((s, _exit), _agg)| s == &site)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        overlay_groups.sort_by(|a, b| {
            let a_total = a.1.total.avg().unwrap_or(f64::INFINITY);
            let b_total = b.1.total.avg().unwrap_or(f64::INFINITY);
            a_total.partial_cmp(&b_total).unwrap_or(std::cmp::Ordering::Equal)
        });

        if args.console {
            println!("Site: {site}");
            println!(
                "  Direct avg total: {}",
                fmt_opt_f64(direct_total_avg)
            );
        }

        for ((_s, exit), overlay) in overlay_groups {
            let overlay_total_avg = overlay.total.avg();
            let overlay_starttransfer_avg = overlay.starttransfer.avg();
            let overlay_connect_avg = overlay.connect.avg();
            let overlay_appconnect_avg = overlay.appconnect.avg();
            let success_rate = overlay.success.success_rate();

            let overhead_total_pct = match (direct_total_avg, overlay_total_avg) {
                (Some(d), Some(o)) if d != 0.0 => Some(overhead_pct(d, o)),
                _ => None,
            };
            let overhead_starttransfer_pct = match (direct_starttransfer_avg, overlay_starttransfer_avg) {
                (Some(d), Some(o)) if d != 0.0 => Some(overhead_pct(d, o)),
                _ => None,
            };

            let avg_inflight = overlay.avg_inflight.avg();
            let max_inflight = overlay.max_inflight.avg();
            let retransmit_rate = overlay.retransmit_rate.avg();
            let throughput_bps = overlay.throughput_bps.avg();
            let ack_latency_ms_avg = overlay.ack_latency_ms_avg.avg();

            let low_window_utilization = avg_inflight.map(|v| v < 1.5);
            let high_retransmit = retransmit_rate.map(|v| v > 0.05);
            let low_success = success_rate.map(|r| r < 0.95);

            let candidate_best_exit = candidate_best_exit
                .get(&site)
                .map(|best| best == &exit)
                .unwrap_or(false);

            if args.console {
                let overhead_str = fmt_opt_f64(overhead_total_pct);
                let best_note = if candidate_best_exit { " (best)" } else { "" };
                println!(
                    "  Overlay {} avg total: {} (overhead {}%){}",
                    exit,
                    fmt_opt_f64(overlay_total_avg),
                    overhead_str,
                    best_note
                );
            }

            let row = vec![
                csv_escape(&site),
                csv_escape(&exit),
                fmt_opt_f64(direct_total_avg),
                fmt_opt_f64(overlay_total_avg),
                fmt_opt_f64(overhead_total_pct),
                fmt_opt_f64(direct_starttransfer_avg),
                fmt_opt_f64(overlay_starttransfer_avg),
                fmt_opt_f64(overhead_starttransfer_pct),
                fmt_opt_f64(overlay_connect_avg),
                fmt_opt_f64(overlay_appconnect_avg),
                fmt_opt_f64(success_rate),
                fmt_opt_f64(avg_inflight),
                fmt_opt_f64(max_inflight),
                fmt_opt_f64(retransmit_rate),
                fmt_opt_f64(throughput_bps),
                fmt_opt_f64(ack_latency_ms_avg),
                low_window_utilization
                    .map(|b| b.to_string())
                    .unwrap_or_default(),
                high_retransmit
                    .map(|b| b.to_string())
                    .unwrap_or_default(),
                low_success.map(|b| b.to_string()).unwrap_or_default(),
                candidate_best_exit.to_string(),
            ];

            summary_csv.push_str(&row.join(","));
            summary_csv.push('\n');
        }
    }

    fs::write(&args.output, summary_csv)?;
    if args.console {
        println!("Wrote summary CSV: {}", args.output.display());
    }

    Ok(())
}

