import argparse
import json
from collections import Counter, defaultdict
from pathlib import Path
from statistics import mean


def load_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def load_jsonl(path: Path):
    if not path.exists():
        return []
    items = []
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line:
            continue
        items.append(json.loads(line))
    return items


def average(values):
    values = [value for value in values if value is not None]
    if not values:
        return None
    return mean(values)


def normalize_open_message_error(error: str) -> str:
    if error.startswith("packet too old for replay window"):
        return "packet too old for replay window"
    if error.startswith("duplicate packet detected"):
        return "duplicate packet detected"
    return error


def count_route_changes(decisions):
    previous = None
    changes = 0
    for item in decisions:
        selected = (
            item.get("selection_policy", {})
            .get("selected_route")
        )
        if not selected:
            continue
        if previous is not None and previous != selected:
            changes += 1
        previous = selected
    return changes


def summarize_profile(prefix: Path):
    profile = load_json(Path(f"{prefix}.profile.json"))
    measurements = load_jsonl(Path(f"{prefix}.jsonl"))
    stage = load_jsonl(Path(f"{prefix}.stage.jsonl"))
    decisions = load_jsonl(Path(f"{prefix}.decisions.jsonl"))

    by_mode = defaultdict(list)
    for item in measurements:
        by_mode[item["mode"]].append(item)

    chaos_counts = Counter()
    client_open_failures = Counter()
    client_duplicate_drops = Counter()
    client_late_events = Counter()
    client_terminal_events = Counter()
    exit_open_failures = Counter()
    exit_duplicate_drops = Counter()
    exit_ack_events = Counter()
    exit_exact = []
    exit_adaptive = []
    for item in stage:
        component = item.get("component")
        stage_name = item.get("stage")
        if component == "transport" and stage_name in {
            "chaos_drop",
            "chaos_duplicate",
            "chaos_delay",
        }:
            chaos_counts[stage_name] += 1
        elif component == "client" and stage_name == "open_message_failed":
            client_open_failures[
                normalize_open_message_error(item.get("error", "<missing>"))
            ] += 1
        elif component == "client" and stage_name == "duplicate_packet_dropped":
            client_duplicate_drops[stage_name] += 1
        elif component == "client" and stage_name in {
            "late_payload_after_completion",
            "late_close_after_completion",
            "payload_after_local_completion_during_settlement",
            "response_timeout",
        }:
            client_late_events[stage_name] += 1
        elif component == "client" and stage_name in {
            "terminal_payload_after_local_completion",
            "duplicate_terminal_payload_after_local_completion",
            "terminal_close_after_local_completion",
            "duplicate_terminal_close_after_local_completion",
            "terminal_close_before_response_start_queued",
            "terminal_close_before_local_completion_queued",
            "duplicate_close_stream_suppressed",
            "duplicate_payload_after_local_completion",
            "duplicate_payload_after_local_completion_repeat",
            "response_transport_local_completion",
            "response_transport_settlement_started",
            "response_transport_terminal_payload_observed",
            "response_transport_terminal_payload_duplicate",
            "response_transport_terminal_close_observed",
            "response_transport_terminal_close_duplicate",
            "response_transport_settlement_completed",
        }:
            client_terminal_events[stage_name] += 1
        elif component == "exit" and stage_name == "open_message_failed":
            exit_open_failures[
                normalize_open_message_error(item.get("error", "<missing>"))
            ] += 1
        elif component == "exit" and stage_name == "duplicate_packet_dropped":
            exit_duplicate_drops[stage_name] += 1
        elif component == "exit" and stage_name in {
            "cumulative_ack_advanced",
            "ack_gap_detected",
            "stale_ack_ignored",
            "ack_regression_ignored",
            "inflight_cleanup_by_ack_range",
        }:
            exit_ack_events[stage_name] += 1
        elif component == "exit" and stage_name == "stream_complete":
            site = item.get("site", "")
            if site.startswith("chaos-exact-3hop-run"):
                exit_exact.append(item)
            elif site.startswith("chaos-adaptive-run"):
                exit_adaptive.append(item)

    decision_reasons = Counter()
    selected_routes = Counter()
    for item in decisions:
        policy = item.get("selection_policy", {})
        decision_reasons[policy.get("decision_reason", "<missing>")] += 1
        if policy.get("selected_route"):
            selected_routes[policy["selected_route"]] += 1

    def mode_summary(mode_name):
        rows = by_mode.get(mode_name, [])
        return {
            "count": len(rows),
            "avg_connect_ms": average([row.get("connect_time_ms") for row in rows]),
            "avg_ttfb_ms": average([row.get("ttfb_ms") for row in rows]),
            "avg_total_ms": average([row.get("total_time_ms") for row in rows]),
            "avg_throughput_bps": average(
                [row.get("effective_throughput_bps") for row in rows]
            ),
        }

    def exit_summary(rows):
        return {
            "count": len(rows),
            "avg_ack_p95_ms": average([row.get("ack_latency_ms_p95") for row in rows]),
            "avg_total_retransmits": average([row.get("total_retransmits") for row in rows]),
            "avg_retransmit_rate": average([row.get("retransmit_rate") for row in rows]),
            "avg_window_wait_total_ms": average(
                [row.get("window_wait_total_ms") for row in rows]
            ),
            "avg_stream_duration_ms": average([row.get("stream_duration_ms") for row in rows]),
            "avg_overlay_gap_ms": average(
                [
                    max(
                        0,
                        (row.get("first_overlay_send_ms") or 0)
                        - (row.get("first_target_byte_ms") or 0),
                    )
                    for row in rows
                ]
            ),
        }

    exact_summary = mode_summary("exact-3hop")
    adaptive_summary = mode_summary("adaptive")
    summary = {
        "profile": profile,
        "measurements": {
            "exact-3hop": exact_summary,
            "adaptive": adaptive_summary,
        },
        "exit_exact": exit_summary(exit_exact),
        "exit_adaptive": exit_summary(exit_adaptive),
        "chaos_counts": dict(chaos_counts),
        "client_open_failures": dict(client_open_failures),
        "client_duplicate_drops": dict(client_duplicate_drops),
        "client_late_events": dict(client_late_events),
        "client_terminal_events": dict(client_terminal_events),
        "exit_open_failures": dict(exit_open_failures),
        "exit_duplicate_drops": dict(exit_duplicate_drops),
        "exit_ack_events": dict(exit_ack_events),
        "decision_reasons": dict(decision_reasons),
        "selected_routes": dict(selected_routes),
        "decision_events": len(decisions),
        "route_changes": count_route_changes(decisions),
    }
    return summary


def fmt(value, digits=2):
    if value is None:
        return "n/a"
    return f"{value:.{digits}f}"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--run-prefix", required=True)
    parser.add_argument("--profiles", nargs="+", required=True)
    args = parser.parse_args()

    run_prefix = Path(args.run_prefix)
    summaries = {}
    for profile_name in args.profiles:
        summaries[profile_name] = summarize_profile(Path(f"{run_prefix}_{profile_name}"))

    base = summaries.get("none", {})
    base_exact_total = (
        base.get("measurements", {})
        .get("exact-3hop", {})
        .get("avg_total_ms")
    )
    base_exact_ttfb = (
        base.get("measurements", {})
        .get("exact-3hop", {})
        .get("avg_ttfb_ms")
    )

    out = []
    out.append(f"# Chaos Matrix Summary: {run_prefix.name}")
    out.append("")
    out.append("## Profiles")
    out.append("")
    for profile_name in args.profiles:
        profile = summaries[profile_name]["profile"]
        out.append(
            f"- `{profile_name}`: loss_ppm={profile['loss_ppm']} duplicate_ppm={profile['duplicate_ppm']} reorder_ppm={profile['reorder_ppm']} base_delay_ms={profile['base_delay_ms']} jitter_ms={profile['jitter_ms']} reorder_extra_delay_ms={profile['reorder_extra_delay_ms']} skip_packets={profile['skip_packets']}"
        )
    out.append("")
    out.append("## Exact 3-Hop Compare")
    out.append("")
    out.append(
        "| profile | avg TTFB ms | delta vs none | avg total ms | delta vs none | avg throughput Bps | avg ack p95 ms | avg retransmits | avg retransmit rate | avg window wait ms |"
    )
    out.append(
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |"
    )
    for profile_name in args.profiles:
        exact = summaries[profile_name]["measurements"]["exact-3hop"]
        exit_exact = summaries[profile_name]["exit_exact"]
        total_delta = None
        ttfb_delta = None
        if base_exact_total is not None and exact["avg_total_ms"] is not None:
            total_delta = exact["avg_total_ms"] - base_exact_total
        if base_exact_ttfb is not None and exact["avg_ttfb_ms"] is not None:
            ttfb_delta = exact["avg_ttfb_ms"] - base_exact_ttfb
        out.append(
            f"| {profile_name} | {fmt(exact['avg_ttfb_ms'])} | {fmt(ttfb_delta)} | {fmt(exact['avg_total_ms'])} | {fmt(total_delta)} | {fmt(exact['avg_throughput_bps'])} | {fmt(exit_exact['avg_ack_p95_ms'])} | {fmt(exit_exact['avg_total_retransmits'])} | {fmt(exit_exact['avg_retransmit_rate'], 4)} | {fmt(exit_exact['avg_window_wait_total_ms'])} |"
        )
    out.append("")
    out.append("## Adaptive Selector")
    out.append("")
    out.append(
        "| profile | decision events | route changes | decision reasons | selected routes | client late events | client terminal events | exit ack events | client duplicate drops | exit duplicate drops | client open failures | exit open failures |"
    )
    out.append("| --- | ---: | ---: | --- | --- | --- | --- | --- | --- | --- | --- | --- |")
    for profile_name in args.profiles:
        summary = summaries[profile_name]
        decision_reasons = ", ".join(
            f"{key}x{value}" for key, value in sorted(summary["decision_reasons"].items())
        ) or "n/a"
        selected_routes = ", ".join(
            f"{key}x{value}" for key, value in sorted(summary["selected_routes"].items())
        ) or "n/a"
        client_late = ", ".join(
            f"{key}x{value}" for key, value in sorted(summary["client_late_events"].items())
        ) or "n/a"
        client_terminal = ", ".join(
            f"{key}x{value}"
            for key, value in sorted(summary["client_terminal_events"].items())
        ) or "n/a"
        exit_ack = ", ".join(
            f"{key}x{value}"
            for key, value in sorted(summary["exit_ack_events"].items())
        ) or "n/a"
        client_duplicate_drops = ", ".join(
            f"{key}x{value}"
            for key, value in sorted(summary["client_duplicate_drops"].items())
        ) or "n/a"
        exit_duplicate_drops = ", ".join(
            f"{key}x{value}"
            for key, value in sorted(summary["exit_duplicate_drops"].items())
        ) or "n/a"
        client_open = ", ".join(
            f"{key}x{value}"
            for key, value in sorted(summary["client_open_failures"].items())
        ) or "n/a"
        exit_open = ", ".join(
            f"{key}x{value}" for key, value in sorted(summary["exit_open_failures"].items())
        ) or "n/a"
        out.append(
            f"| {profile_name} | {summary['decision_events']} | {summary['route_changes']} | {decision_reasons} | {selected_routes} | {client_late} | {client_terminal} | {exit_ack} | {client_duplicate_drops} | {exit_duplicate_drops} | {client_open} | {exit_open} |"
        )
    out.append("")
    out.append("## Chaos Actions")
    out.append("")
    for profile_name in args.profiles:
        chaos_counts = summaries[profile_name]["chaos_counts"]
        counts = ", ".join(
            f"{key}x{value}" for key, value in sorted(chaos_counts.items())
        ) or "no injected actions"
        out.append(f"- `{profile_name}`: {counts}")
    out.append("")
    out.append("## First Bottleneck Ranking")
    out.append("")
    ranked = []
    for profile_name in args.profiles:
        if profile_name == "none":
            continue
        summary = summaries[profile_name]
        pressure = (
            sum(summary["client_late_events"].values()) * 1000
            + sum(summary["client_open_failures"].values()) * 500
            + sum(summary["exit_open_failures"].values()) * 500
            + int((summary["exit_exact"]["avg_retransmit_rate"] or 0.0) * 10_000)
            + int(summary["exit_exact"]["avg_window_wait_total_ms"] or 0)
        )
        ranked.append((pressure, profile_name))
    ranked.sort(reverse=True)
    for pressure, profile_name in ranked:
        summary = summaries[profile_name]
        out.append(
            f"- `{profile_name}` pressure={pressure}: late_events={sum(summary['client_late_events'].values())}, client_open_failures={sum(summary['client_open_failures'].values())}, exit_open_failures={sum(summary['exit_open_failures'].values())}, retransmit_rate={fmt(summary['exit_exact']['avg_retransmit_rate'], 4)}, window_wait_ms={fmt(summary['exit_exact']['avg_window_wait_total_ms'])}"
        )
    out.append("")

    md_path = Path(f"{run_prefix}.md")
    md_path.write_text("\n".join(out) + "\n", encoding="utf-8")
    json_path = Path(f"{run_prefix}.summary.json")
    json_path.write_text(json.dumps(summaries, indent=2), encoding="utf-8")


if __name__ == "__main__":
    main()
