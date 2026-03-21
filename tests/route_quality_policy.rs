use anyhow::Context;
use serde::Serialize;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use vpnnode::addr::NodeAddr;
use vpnnode::protocol::ResponseQualityFeedback;
use vpnnode::route::Route;
use vpnnode::route_store::{
    LocalRouteObservation, RouteFailureKind, RouteSelectionPolicy, RouteStore,
};

#[derive(Debug, Serialize)]
struct CandidateArtifact {
    route_id: String,
    route_len: usize,
    route_chain: String,
    final_score: f32,
    base: f32,
    transport_agg: f32,
    quality_agg: f32,
    hop_factor: f32,
    quality_confidence: f32,
    warmup_confidence: f32,
    instability_factor: f32,
    metric_instability_ppm: Option<u32>,
    flap_penalty_factor: f32,
    recent_flap_count: u64,
    recent_ttfb_ms: Option<u64>,
    recent_total_ms: Option<u64>,
    recent_ack_p95_ms: Option<u64>,
    recent_retransmit_rate_ppm: Option<u32>,
    recent_window_wait_ratio_ppm: Option<u32>,
}

#[derive(Debug, Serialize)]
struct PhaseArtifact {
    phase: String,
    selected_route_id: String,
    selected_route_chain: String,
    best_route_id: String,
    best_route_chain: String,
    previous_route_id: Option<String>,
    previous_route_chain: Option<String>,
    decision_reason: &'static str,
    tie_break_reason: Option<&'static str>,
    switched: bool,
    score_delta_abs: f32,
    score_delta_ratio: f32,
    required_abs_margin: f32,
    required_rel_margin: f32,
    hold_remaining_ms: Option<u64>,
    candidates: Vec<CandidateArtifact>,
}

fn route_id(idx: usize) -> String {
    match idx {
        0 => "A".to_string(),
        1 => "B".to_string(),
        2 => "C".to_string(),
        _ => format!("R{idx}"),
    }
}

fn route_chain(route: &Route) -> String {
    route
        .hops
        .iter()
        .map(|hop| hop.to_string())
        .collect::<Vec<_>>()
        .join(" -> ")
}

fn r(s: &str) -> SocketAddr {
    s.parse().unwrap()
}

fn prepare_output_path(path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create artifact directory {}", parent.display()))?;
    }
    Ok(())
}

fn append_jsonl<T: Serialize>(path: &Path, value: &T) -> anyhow::Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("open {}", path.display()))?;
    serde_json::to_writer(&mut file, value).context("serialize jsonl record")?;
    file.write_all(b"\n").context("append newline")?;
    Ok(())
}

fn sample_feedback(
    route_len: u8,
    ack_p95_ms: u64,
    retransmit_rate_ppm: u32,
    window_wait_total_ms: u64,
    stream_duration_ms: u64,
    http_code: u16,
) -> ResponseQualityFeedback {
    ResponseQualityFeedback {
        stream_id: 77,
        route_len,
        resp_bytes: 256 * 1024,
        frames_sent: 256,
        stream_duration_ms,
        first_target_byte_ms: Some(2),
        first_overlay_send_ms: Some(4),
        ack_latency_ms_avg: Some(ack_p95_ms / 2),
        ack_latency_ms_p50: Some(ack_p95_ms / 3),
        ack_latency_ms_p95: Some(ack_p95_ms),
        ack_latency_ms_max: Some(ack_p95_ms.saturating_add(40)),
        total_retransmits: 3,
        retransmit_rate_ppm,
        window_wait_events: 8,
        window_wait_total_ms,
        window_wait_max_ms: 80,
        http_code: Some(http_code),
    }
}

fn capture_phase(
    store: &mut RouteStore,
    routes: &[Route],
    phase: &str,
    now: Instant,
) -> anyhow::Result<PhaseArtifact> {
    let scored = store.scored_candidates(now, &[]);
    let decision = store
        .apply_selection_policy(&scored, now)
        .with_context(|| format!("selection failed for phase {phase}"))?;
    let candidates = scored
        .iter()
        .map(|(idx, details)| CandidateArtifact {
            route_id: route_id(*idx),
            route_len: routes[*idx].len(),
            route_chain: route_chain(&routes[*idx]),
            final_score: details.final_score,
            base: details.base,
            transport_agg: details.transport_agg,
            quality_agg: details.quality_agg,
            hop_factor: details.hop_factor,
            quality_confidence: details.quality_confidence,
            warmup_confidence: details.warmup_confidence,
            instability_factor: details.instability_factor,
            metric_instability_ppm: details.metric_instability_ppm,
            flap_penalty_factor: details.flap_penalty_factor,
            recent_flap_count: details.recent_flap_count,
            recent_ttfb_ms: details.recent_ttfb_ms,
            recent_total_ms: details.recent_total_ms,
            recent_ack_p95_ms: details.recent_ack_p95_ms,
            recent_retransmit_rate_ppm: details.recent_retransmit_rate_ppm,
            recent_window_wait_ratio_ppm: details.recent_window_wait_ratio_ppm,
        })
        .collect::<Vec<_>>();
    Ok(PhaseArtifact {
        phase: phase.to_string(),
        selected_route_id: route_id(decision.selected_idx),
        selected_route_chain: route_chain(&routes[decision.selected_idx]),
        best_route_id: route_id(decision.best_idx),
        best_route_chain: route_chain(&routes[decision.best_idx]),
        previous_route_id: decision.previous_idx.map(route_id),
        previous_route_chain: decision.previous_idx.map(|idx| route_chain(&routes[idx])),
        decision_reason: decision.decision_reason,
        tie_break_reason: decision.tie_break_reason,
        switched: decision.switched,
        score_delta_abs: decision.score_delta_abs,
        score_delta_ratio: decision.score_delta_ratio,
        required_abs_margin: decision.required_abs_margin,
        required_rel_margin: decision.required_rel_margin,
        hold_remaining_ms: decision.hold_remaining_ms,
        candidates,
    })
}

fn render_report(policy: RouteSelectionPolicy, phases: &[PhaseArtifact]) -> String {
    let mut markdown = String::new();
    markdown.push_str("# Controlled Route-Quality Causality\n\n");
    markdown.push_str("## Policy\n\n");
    markdown.push_str(&format!(
        "- switch threshold: absolute `>= {:.3}` and relative `>= {:.1}%`\n- hysteresis margin for ties: absolute `< {:.3}` and relative `< {:.1}%`\n- hold time: `{}s`\n- emergency switch during hold: absolute `>= {:.3}` or relative `>= {:.1}%`\n\n",
        policy.switch_absolute_margin,
        policy.switch_relative_margin * 100.0,
        policy.tie_absolute_margin,
        policy.tie_relative_margin * 100.0,
        policy.hold_time.as_secs(),
        policy.emergency_switch_absolute_margin,
        policy.emergency_switch_relative_margin * 100.0
    ));
    markdown.push_str("## Phases\n\n");
    for phase in phases {
        markdown.push_str(&format!("### {}\n\n", phase.phase));
        markdown.push_str(&format!(
            "- selected: `{}`\n- best candidate: `{}`\n- previous: `{}`\n- reason: `{}`\n- tie-break: `{}`\n- switched: `{}`\n- score delta abs: `{:.4}`\n- score delta ratio: `{:.2}%`\n- required switch margins: abs `{:.3}`, rel `{:.1}%`\n- hold remaining ms: `{}`\n\n",
            phase.selected_route_id,
            phase.best_route_id,
            phase.previous_route_id.as_deref().unwrap_or("-"),
            phase.decision_reason,
            phase.tie_break_reason.unwrap_or("-"),
            phase.switched,
            phase.score_delta_abs,
            phase.score_delta_ratio * 100.0,
            phase.required_abs_margin,
            phase.required_rel_margin * 100.0,
            phase
                .hold_remaining_ms
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string())
        ));
        markdown.push_str("| route | hops | score | base | transport | quality | q conf | warmup | instability ppm / factor | flap penalty | flap count | TTFB ms | total ms | ACK p95 ms | retrans ppm | stall ppm |\n");
        markdown.push_str(
            "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |\n",
        );
        for candidate in &phase.candidates {
            markdown.push_str(&format!(
                "| {} | {} | {:.4} | {:.4} | {:.4} | {:.4} | {:.3} | {:.3} | {} / {:.3} | {:.3} | {} | {} | {} | {} | {} | {} |\n",
                candidate.route_id,
                candidate.route_len,
                candidate.final_score,
                candidate.base,
                candidate.transport_agg,
                candidate.quality_agg,
                candidate.quality_confidence,
                candidate.warmup_confidence,
                candidate
                    .metric_instability_ppm
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                candidate.instability_factor,
                candidate.flap_penalty_factor,
                candidate.recent_flap_count,
                candidate
                    .recent_ttfb_ms
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                candidate
                    .recent_total_ms
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                candidate
                    .recent_ack_p95_ms
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                candidate
                    .recent_retransmit_rate_ppm
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                candidate
                    .recent_window_wait_ratio_ppm
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "-".to_string()),
            ));
        }
        markdown.push('\n');
    }
    markdown.push_str("## Result\n\n");
    markdown.push_str("The controlled policy run proves three things at once: cold-start luck is damped by warmup confidence, noisy routes are penalized by instability, and real degradation still causes a deterministic switch once the challenger is materially better. The anti-flap sub-scenario then shows a smaller B-over-A edge that is recorded, but still suppressed by hold time and hysteresis.\n");
    markdown
}

#[test]
fn controlled_route_quality_switch_and_hysteresis() -> anyhow::Result<()> {
    let raw_path = PathBuf::from(
        std::env::var("VPNNODE_ROUTE_QUALITY_CONTROLLED_RAW_PATH")
            .unwrap_or_else(|_| "docs/artifacts/route_quality_controlled_2026-03-21.jsonl".into()),
    );
    let report_path = PathBuf::from(
        std::env::var("VPNNODE_ROUTE_QUALITY_CONTROLLED_REPORT_PATH")
            .unwrap_or_else(|_| "docs/artifacts/route_quality_controlled_2026-03-21.md".into()),
    );
    prepare_output_path(&raw_path)?;
    prepare_output_path(&report_path)?;
    fs::write(&raw_path, b"").with_context(|| format!("truncate {}", raw_path.display()))?;

    let route_a = Route {
        hops: vec![NodeAddr::from(r("127.0.0.1:6101"))],
    };
    let route_b = Route {
        hops: vec![
            NodeAddr::from(r("127.0.0.1:6102")),
            NodeAddr::from(r("127.0.0.1:6103")),
        ],
    };
    let route_c = Route {
        hops: vec![
            NodeAddr::from(r("127.0.0.1:6104")),
            NodeAddr::from(r("127.0.0.1:6105")),
            NodeAddr::from(r("127.0.0.1:6106")),
        ],
    };
    let routes = vec![route_a.clone(), route_b.clone(), route_c.clone()];

    let mut store = RouteStore::new();
    store.add_route(route_a.clone(), 0.88);
    store.add_route(route_b.clone(), 0.86);
    store.add_route(route_c.clone(), 0.84);
    store.update_metrics(0, Some(80), true);
    store.update_metrics(1, Some(105), true);
    store.update_metrics(2, Some(135), true);

    store.record_local_observation(
        0,
        &LocalRouteObservation {
            success: true,
            status_code: Some(200),
            ttfb_ms: Some(310),
            total_ms: 640,
            response_bytes: 256 * 1024,
        },
    );
    store.record_local_observation(
        1,
        &LocalRouteObservation {
            success: true,
            status_code: Some(200),
            ttfb_ms: Some(560),
            total_ms: 940,
            response_bytes: 256 * 1024,
        },
    );
    store.record_local_observation(
        2,
        &LocalRouteObservation {
            success: true,
            status_code: Some(200),
            ttfb_ms: Some(820),
            total_ms: 1_320,
            response_bytes: 256 * 1024,
        },
    );
    assert!(store
        .record_response_quality_for_route(&route_a, &sample_feedback(1, 140, 0, 40, 700, 200),));
    assert!(store
        .record_response_quality_for_route(&route_b, &sample_feedback(2, 240, 0, 120, 980, 200),));
    assert!(store.record_response_quality_for_route(
        &route_c,
        &sample_feedback(3, 380, 8_000, 330, 1_350, 200),
    ));

    let t0 = Instant::now();
    let phase_a = capture_phase(&mut store, &routes, "A: healthy route A wins", t0)?;
    eprintln!(
        "phase=A selected={} reason={} best={} switched={}",
        phase_a.selected_route_id, phase_a.decision_reason, phase_a.best_route_id, phase_a.switched
    );
    assert_eq!(phase_a.selected_route_id, "A");

    let mut warmup_store = RouteStore::new();
    warmup_store.add_route(route_a.clone(), 0.86);
    warmup_store.add_route(route_b.clone(), 0.86);
    warmup_store.update_metrics(0, Some(90), true);
    warmup_store.update_metrics(1, Some(95), true);
    for _ in 0..6 {
        warmup_store.record_local_observation(
            0,
            &LocalRouteObservation {
                success: true,
                status_code: Some(200),
                ttfb_ms: Some(540),
                total_ms: 980,
                response_bytes: 256 * 1024,
            },
        );
        assert!(warmup_store.record_response_quality_for_route(
            &route_a,
            &sample_feedback(1, 240, 0, 120, 980, 200),
        ));
    }
    let phase_w0 = capture_phase(
        &mut warmup_store,
        &[route_a.clone(), route_b.clone()],
        "W0: warmed route A becomes current route",
        t0 + Duration::from_secs(40),
    )?;
    assert_eq!(phase_w0.selected_route_id, "A");

    warmup_store.record_local_observation(
        1,
        &LocalRouteObservation {
            success: true,
            status_code: Some(200),
            ttfb_ms: Some(140),
            total_ms: 320,
            response_bytes: 256 * 1024,
        },
    );
    assert!(warmup_store.record_response_quality_for_route(
        &route_b,
        &sample_feedback(2, 90, 0, 30, 350, 200),
    ));
    let phase_w1 = capture_phase(
        &mut warmup_store,
        &[route_a.clone(), route_b.clone()],
        "W1: one lucky cold-start sample does not steal selection",
        t0 + Duration::from_secs(44),
    )?;
    eprintln!(
        "phase=W1 selected={} reason={} previous={:?} switched={} delta_abs={:.4} delta_ratio={:.2}%",
        phase_w1.selected_route_id,
        phase_w1.decision_reason,
        phase_w1.previous_route_id,
        phase_w1.switched,
        phase_w1.score_delta_abs,
        phase_w1.score_delta_ratio * 100.0
    );
    assert_eq!(phase_w1.selected_route_id, "A");
    assert!(!phase_w1.switched);

    for _ in 0..3 {
        store.record_local_observation(
            0,
            &LocalRouteObservation {
                success: false,
                status_code: Some(504),
                ttfb_ms: Some(3_800),
                total_ms: 4_700,
                response_bytes: 0,
            },
        );
        store.record_failure(0, RouteFailureKind::Congestion);
        assert!(store.record_response_quality_for_route(
            &route_a,
            &sample_feedback(1, 1_250, 180_000, 3_600, 4_900, 504),
        ));
    }
    store.record_local_observation(
        1,
        &LocalRouteObservation {
            success: true,
            status_code: Some(200),
            ttfb_ms: Some(500),
            total_ms: 880,
            response_bytes: 256 * 1024,
        },
    );
    assert!(store
        .record_response_quality_for_route(&route_b, &sample_feedback(2, 220, 0, 100, 930, 200),));

    let phase_c = capture_phase(
        &mut store,
        &routes,
        "C: route A degraded, route B should take over",
        t0 + Duration::from_secs(20),
    )?;
    eprintln!(
        "phase=C selected={} reason={} previous={:?} switched={} delta_abs={:.4} delta_ratio={:.2}%",
        phase_c.selected_route_id,
        phase_c.decision_reason,
        phase_c.previous_route_id,
        phase_c.switched,
        phase_c.score_delta_abs,
        phase_c.score_delta_ratio * 100.0
    );
    assert_eq!(phase_c.selected_route_id, "B");
    assert_eq!(phase_c.decision_reason, "switch_margin_exceeded");
    assert!(phase_c.switched);

    let route_n0 = Route {
        hops: vec![NodeAddr::from(r("127.0.0.1:6201"))],
    };
    let route_n1 = Route {
        hops: vec![NodeAddr::from(r("127.0.0.1:6202"))],
    };
    let mut noisy_store = RouteStore::new();
    noisy_store.add_route(route_n0.clone(), 0.86);
    noisy_store.add_route(route_n1.clone(), 0.86);
    noisy_store.update_metrics(0, Some(100), true);
    noisy_store.update_metrics(1, Some(100), true);
    for _ in 0..5 {
        noisy_store.record_local_observation(
            0,
            &LocalRouteObservation {
                success: true,
                status_code: Some(200),
                ttfb_ms: Some(500),
                total_ms: 1_000,
                response_bytes: 256 * 1024,
            },
        );
        assert!(noisy_store.record_response_quality_for_route(
            &route_n0,
            &sample_feedback(1, 250, 8_000, 140, 1_000, 200),
        ));
    }
    for (ttfb_ms, total_ms, ack_ms, retrans_ppm) in [
        (220, 520, 120, 0),
        (940, 1_520, 540, 60_000),
        (260, 560, 150, 5_000),
        (980, 1_480, 580, 75_000),
        (280, 540, 160, 8_000),
    ] {
        noisy_store.record_local_observation(
            1,
            &LocalRouteObservation {
                success: true,
                status_code: Some(200),
                ttfb_ms: Some(ttfb_ms),
                total_ms,
                response_bytes: 256 * 1024,
            },
        );
        assert!(noisy_store.record_response_quality_for_route(
            &route_n1,
            &sample_feedback(1, ack_ms, retrans_ppm, 180, total_ms, 200),
        ));
    }
    let phase_n = capture_phase(
        &mut noisy_store,
        &[route_n0.clone(), route_n1.clone()],
        "N: noisy route is penalized even when average latency looks competitive",
        t0 + Duration::from_secs(58),
    )?;
    eprintln!(
        "phase=N selected={} reason={} best={} switched={}",
        phase_n.selected_route_id, phase_n.decision_reason, phase_n.best_route_id, phase_n.switched
    );
    assert_eq!(phase_n.selected_route_id, "A");
    assert!(
        phase_n.candidates[1]
            .metric_instability_ppm
            .unwrap_or_default()
            > phase_n.candidates[0].metric_instability_ppm.unwrap_or_default()
    );

    let mut flap_store = RouteStore::new();
    flap_store.add_route(route_a.clone(), 0.86);
    flap_store.add_route(route_b.clone(), 0.86);
    flap_store.update_metrics(0, Some(95), true);
    flap_store.update_metrics(1, Some(95), true);
    flap_store.record_local_observation(
        0,
        &LocalRouteObservation {
            success: true,
            status_code: Some(200),
            ttfb_ms: Some(520),
            total_ms: 900,
            response_bytes: 256 * 1024,
        },
    );
    assert!(flap_store
        .record_response_quality_for_route(&route_a, &sample_feedback(1, 250, 0, 120, 980, 200),));
    let flap_initial = capture_phase(
        &mut flap_store,
        &[route_a.clone(), route_b.clone()],
        "H0: current route established",
        t0 + Duration::from_secs(70),
    )?;
    assert_eq!(flap_initial.selected_route_id, "A");

    flap_store.record_local_observation(
        0,
        &LocalRouteObservation {
            success: false,
            status_code: Some(504),
            ttfb_ms: Some(980),
            total_ms: 1_180,
            response_bytes: 0,
        },
    );
    for _ in 0..5 {
        flap_store.record_local_observation(
            1,
            &LocalRouteObservation {
                success: true,
                status_code: Some(200),
                ttfb_ms: Some(430),
                total_ms: 780,
                response_bytes: 256 * 1024,
            },
        );
        assert!(flap_store.record_response_quality_for_route(
            &route_b,
            &sample_feedback(2, 210, 0, 85, 880, 200),
        ));
    }

    let phase_h1 = capture_phase(
        &mut flap_store,
        &[route_a.clone(), route_b.clone()],
        "H1: challenger slightly better, hold keeps current route",
        t0 + Duration::from_secs(74),
    )?;
    eprintln!(
        "phase=H1 selected={} reason={} previous={:?} switched={} hold_remaining_ms={:?}",
        phase_h1.selected_route_id,
        phase_h1.decision_reason,
        phase_h1.previous_route_id,
        phase_h1.switched,
        phase_h1.hold_remaining_ms
    );
    assert_eq!(phase_h1.selected_route_id, "A");
    assert_eq!(phase_h1.decision_reason, "hold_time_active");
    assert!(!phase_h1.switched);

    let phase_h2 = capture_phase(
        &mut flap_store,
        &[route_a.clone(), route_b.clone()],
        "H2: after hold, small delta still stays below hysteresis threshold",
        t0 + Duration::from_secs(95),
    )?;
    eprintln!(
        "phase=H2 selected={} reason={} previous={:?} switched={}",
        phase_h2.selected_route_id,
        phase_h2.decision_reason,
        phase_h2.previous_route_id,
        phase_h2.switched
    );
    assert_eq!(phase_h2.selected_route_id, "A");
    assert_eq!(phase_h2.decision_reason, "within_hysteresis_margin");
    assert!(!phase_h2.switched);

    let mut tie_store = RouteStore::new();
    tie_store.add_route(route_a.clone(), 0.85);
    tie_store.add_route(route_b.clone(), 0.85);
    tie_store.update_metrics(0, Some(100), true);
    tie_store.update_metrics(1, Some(100), true);
    let tie_phase = capture_phase(
        &mut tie_store,
        &[route_a.clone(), route_b.clone()],
        "T: tie-break prefers shorter route",
        t0 + Duration::from_secs(60),
    )?;
    eprintln!(
        "phase=T selected={} reason={} tie_break={:?}",
        tie_phase.selected_route_id, tie_phase.decision_reason, tie_phase.tie_break_reason
    );
    assert_eq!(tie_phase.selected_route_id, "A");
    assert_eq!(tie_phase.tie_break_reason, Some("shorter_route_tie_break"));

    let phases = vec![
        phase_a,
        phase_w0,
        phase_w1,
        phase_c,
        phase_n,
        flap_initial,
        phase_h1,
        phase_h2,
        tie_phase,
    ];
    for phase in &phases {
        append_jsonl(&raw_path, phase)?;
    }
    fs::write(
        &report_path,
        render_report(RouteStore::selection_policy(), &phases),
    )
    .with_context(|| format!("write {}", report_path.display()))?;
    Ok(())
}
