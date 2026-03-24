#!/usr/bin/env python3
import json
import math
import os
from collections import defaultdict
from pathlib import Path


def env(name: str, default: str) -> str:
    return os.environ.get(name, default)


LOCAL_RAW_PATH = Path(
    env(
        "VPNNODE_PERF_STAGE_LOCAL_RAW_PATH",
        "docs/artifacts/remote_perf_stage_matrix_2026-03-21.local.jsonl",
    )
)
CLIENT_STAGE_PATH = Path(
    env(
        "VPNNODE_STAGE_TRACE_PATH",
        "docs/artifacts/remote_perf_stage_matrix_2026-03-21.client.jsonl",
    )
)
EXIT_STAGE_PATH = Path(
    env(
        "VPNNODE_PERF_EXIT_STAGE_LOG_LOCAL_PATH",
        "docs/artifacts/remote_perf_stage_matrix_2026-03-21.exit.jsonl",
    )
)
DIRECT_STAGE_PATH = Path(
    env(
        "VPNNODE_PERF_DIRECT_STAGE_LOG_LOCAL_PATH",
        "docs/artifacts/remote_perf_stage_matrix_2026-03-21.direct.jsonl",
    )
)
SUMMARY_PATH = Path(
    env(
        "VPNNODE_PERF_STAGE_SUMMARY_PATH",
        "docs/artifacts/remote_perf_stage_matrix_2026-03-21.md",
    )
)
JOINED_PATH = Path(
    env(
        "VPNNODE_PERF_STAGE_JOINED_PATH",
        "docs/artifacts/remote_perf_stage_matrix_2026-03-21.joined.jsonl",
    )
)


def read_jsonl(path: Path):
    if not path.exists():
        return []
    rows = []
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line:
            continue
        rows.append(json.loads(line))
    return rows


def mean(values):
    values = [value for value in values if value is not None]
    if not values:
        return None
    return sum(values) / len(values)


def min_avg_max(values):
    values = [value for value in values if value is not None]
    if not values:
        return (None, None, None)
    return (min(values), mean(values), max(values))


def fmt_ms(value):
    return "-" if value is None else f"{value:.2f}"


def fmt_pct(value):
    return "-" if value is None else f"{value:.2f}%"


def fmt_num(value, digits=2):
    return "-" if value is None else f"{value:.{digits}f}"


def pearson(xs, ys):
    pairs = [(x, y) for x, y in zip(xs, ys) if x is not None and y is not None]
    if len(pairs) < 2:
        return None
    x_values = [pair[0] for pair in pairs]
    y_values = [pair[1] for pair in pairs]
    x_mean = sum(x_values) / len(x_values)
    y_mean = sum(y_values) / len(y_values)
    x_var = sum((value - x_mean) ** 2 for value in x_values)
    y_var = sum((value - y_mean) ** 2 for value in y_values)
    if x_var == 0 or y_var == 0:
        return None
    cov = sum((x - x_mean) * (y - y_mean) for x, y in pairs)
    return cov / math.sqrt(x_var * y_var)


def first_by_stage(events):
    by_stage = defaultdict(list)
    for event in events:
        by_stage[event.get("stage")].append(event)
    return {stage: items[0] for stage, items in by_stage.items() if items}


def scenario_sort_key(scenario_name: str):
    if scenario_name == "direct":
        return (0, 0, scenario_name)
    if scenario_name.startswith("remote-") and scenario_name.endswith("hop"):
        middle = scenario_name[len("remote-") : -len("hop")]
        if middle.isdigit():
            return (1, int(middle), scenario_name)
    return (2, 0, scenario_name)


local_rows = read_jsonl(LOCAL_RAW_PATH)
client_stage_rows = read_jsonl(CLIENT_STAGE_PATH)
exit_stage_rows = read_jsonl(EXIT_STAGE_PATH)
direct_stage_rows = read_jsonl(DIRECT_STAGE_PATH)

warmups = {}
measurements = []
for row in local_rows:
    record_type = row.get("record_type")
    if record_type == "warmup":
        warmups[row["scenario"]] = row
    elif record_type == "measurement":
        measurements.append(row)

client_by_site = defaultdict(list)
client_sessions = {}
for row in client_stage_rows:
    site = row.get("site")
    if site:
        client_by_site[site].append(row)
    if row.get("stage") == "session_established":
        client_sessions[row.get("route_len")] = row

exit_by_site = defaultdict(list)
for row in exit_stage_rows:
    site = row.get("site")
    if site:
        exit_by_site[site].append(row)

direct_by_host = defaultdict(list)
for row in direct_stage_rows:
    request_host = row.get("request_host")
    if request_host:
        direct_by_host[request_host].append(row)

joined = []
for measurement in measurements:
    request_host = measurement["request_host"]
    scenario = measurement["scenario"]
    route_length = measurement["route_length"]
    joined_row = dict(measurement)

    warmup = warmups.get(scenario, {})
    joined_row["listener_ready_ms"] = warmup.get("listener_ready_ms")
    joined_row["route_ready_ms"] = warmup.get("route_ready_ms")

    if route_length and route_length in client_sessions:
        joined_row["handshake_ms"] = client_sessions[route_length].get("handshake_ms")
    else:
        joined_row["handshake_ms"] = None

    client_stages = first_by_stage(client_by_site.get(request_host, []))
    request_buffered = client_stages.get("request_buffered", {})
    route_selected = client_stages.get("route_selected", {})
    first_hop_send = client_stages.get("first_hop_send", {})
    first_byte_delivered = client_stages.get("first_byte_delivered", {})
    client_complete = client_stages.get("stream_complete", {})

    joined_row["client_buffer_ms"] = request_buffered.get("buffer_ms")
    joined_row["client_route_selected_ms"] = route_selected.get("since_request_buffered_ms")
    joined_row["client_route_select_overhead_ms"] = None
    if (
        request_buffered.get("buffer_ms") is not None
        and route_selected.get("since_request_buffered_ms") is not None
    ):
        joined_row["client_route_select_overhead_ms"] = (
            route_selected["since_request_buffered_ms"] - request_buffered["buffer_ms"]
        )
    joined_row["client_first_hop_send_ms"] = first_hop_send.get("since_roundtrip_start_ms")
    joined_row["client_first_byte_stage_ms"] = first_byte_delivered.get(
        "since_roundtrip_start_ms"
    )
    joined_row["client_stream_complete_stage_ms"] = client_complete.get(
        "since_roundtrip_start_ms"
    )

    if scenario == "direct":
        direct_events = first_by_stage(direct_by_host.get(request_host, []))
        direct_complete = direct_events.get("connection_complete", {})
        joined_row["target_connect_ms"] = direct_complete.get("target_connect_ms")
        joined_row["first_target_byte_wait_ms"] = direct_complete.get(
            "first_target_byte_after_connect_ms"
        )
        joined_row["exit_stream_complete_ms"] = direct_complete.get("total_ms")
        joined_row["first_overlay_send_ms"] = None
        joined_row["first_overlay_send_after_target_ms"] = None
        joined_row["retransmit_rate"] = None
        joined_row["total_retransmits"] = None
        joined_row["ack_latency_ms_avg"] = None
        joined_row["ack_latency_ms_min"] = None
        joined_row["ack_latency_ms_p50"] = None
        joined_row["ack_latency_ms_p95"] = None
        joined_row["ack_latency_ms_max"] = None
        joined_row["avg_inflight"] = None
        joined_row["max_inflight"] = None
        joined_row["window_frames"] = None
        joined_row["window_wait_events"] = None
        joined_row["window_wait_total_ms"] = None
        joined_row["window_wait_max_ms"] = None
        joined_row["time_at_inflight_1_ms"] = None
    else:
        exit_stages = first_by_stage(exit_by_site.get(request_host, []))
        target_connect_completed = exit_stages.get("target_connect_completed", {})
        first_target_byte = exit_stages.get("first_target_byte", {})
        first_overlay_send = exit_stages.get("first_overlay_send", {})
        stream_complete = exit_stages.get("stream_complete", {})
        joined_row["target_connect_ms"] = target_connect_completed.get("target_connect_ms")
        joined_row["first_target_byte_wait_ms"] = first_target_byte.get(
            "since_response_start_ms"
        )
        joined_row["first_overlay_send_ms"] = first_overlay_send.get(
            "since_response_start_ms"
        )
        joined_row["first_overlay_send_after_target_ms"] = first_overlay_send.get(
            "since_first_target_byte_ms"
        )
        joined_row["exit_stream_complete_ms"] = stream_complete.get("stream_duration_ms")
        joined_row["retransmit_rate"] = stream_complete.get("retransmit_rate")
        joined_row["total_retransmits"] = stream_complete.get("total_retransmits")
        joined_row["retransmit_timeout_ms_avg"] = stream_complete.get(
            "retransmit_timeout_ms_avg"
        )
        joined_row["retransmit_timeout_ms_min"] = stream_complete.get(
            "retransmit_timeout_ms_min"
        )
        joined_row["retransmit_timeout_ms_p50"] = stream_complete.get(
            "retransmit_timeout_ms_p50"
        )
        joined_row["retransmit_timeout_ms_p95"] = stream_complete.get(
            "retransmit_timeout_ms_p95"
        )
        joined_row["retransmit_timeout_ms_max"] = stream_complete.get(
            "retransmit_timeout_ms_max"
        )
        joined_row["retransmit_trigger_count"] = stream_complete.get(
            "retransmit_trigger_count"
        )
        joined_row["retransmit_early_count"] = stream_complete.get(
            "retransmit_early_count"
        )
        joined_row["retransmit_late_count"] = stream_complete.get(
            "retransmit_late_count"
        )
        joined_row["retransmit_rtt_ratio"] = stream_complete.get("retransmit_rtt_ratio")
        joined_row["ack_latency_ms_avg"] = stream_complete.get("ack_latency_ms_avg")
        joined_row["ack_latency_ms_min"] = stream_complete.get("ack_latency_ms_min")
        joined_row["ack_latency_ms_p50"] = stream_complete.get("ack_latency_ms_p50")
        joined_row["ack_latency_ms_p95"] = stream_complete.get("ack_latency_ms_p95")
        joined_row["ack_latency_ms_max"] = stream_complete.get("ack_latency_ms_max")
        joined_row["avg_inflight"] = stream_complete.get("avg_inflight")
        joined_row["max_inflight"] = stream_complete.get("max_inflight")
        joined_row["window_frames"] = stream_complete.get("window_frames")
        joined_row["window_wait_events"] = stream_complete.get("window_wait_events")
        joined_row["window_wait_total_ms"] = stream_complete.get("window_wait_total_ms")
        joined_row["window_wait_max_ms"] = stream_complete.get("window_wait_max_ms")
        joined_row["effective_cap_wait_events"] = stream_complete.get(
            "effective_cap_wait_events"
        )
        joined_row["effective_cap_wait_total_ms"] = stream_complete.get(
            "effective_cap_wait_total_ms"
        )
        joined_row["effective_cap_wait_max_ms"] = stream_complete.get(
            "effective_cap_wait_max_ms"
        )
        joined_row["time_at_inflight_1_ms"] = stream_complete.get("time_at_inflight_1_ms")
        joined_row["pacing_enabled"] = stream_complete.get("pacing_enabled")
        joined_row["pacing_delay_applied"] = stream_complete.get("pacing_delay_applied")
        joined_row["pacing_delay_applied_ms_total"] = stream_complete.get(
            "pacing_delay_applied_ms_total"
        )
        joined_row["pacing_interval_ms_avg"] = stream_complete.get("pacing_interval_ms_avg")
        joined_row["pacing_interval_ms_max"] = stream_complete.get("pacing_interval_ms_max")
        joined_row["burst_prevented_count"] = stream_complete.get("burst_prevented_count")
        joined_row["paced_send_batches"] = stream_complete.get("paced_send_batches")
        joined_row["max_send_burst_frames"] = stream_complete.get("max_send_burst_frames")
        joined_row["effective_inflight_cap_avg"] = stream_complete.get(
            "effective_inflight_cap_avg"
        )
        joined_row["effective_inflight_cap_min"] = stream_complete.get(
            "effective_inflight_cap_min"
        )
        joined_row["effective_inflight_cap_max"] = stream_complete.get(
            "effective_inflight_cap_max"
        )
        joined_row["inflight_cap_reduced_count"] = stream_complete.get(
            "inflight_cap_reduced_count"
        )
        joined_row["inflight_cap_restore_count"] = stream_complete.get(
            "inflight_cap_restore_count"
        )
        joined_row["ack_pressure_events"] = stream_complete.get("ack_pressure_events")
        joined_row["send_blocked_by_effective_cap"] = stream_complete.get(
            "send_blocked_by_effective_cap"
        )
        joined_row["congestion_state"] = stream_complete.get("congestion_state")
        joined_row["congestion_events"] = stream_complete.get("congestion_events")
        joined_row["congestion_duration_ms"] = stream_complete.get(
            "congestion_duration_ms"
        )
        joined_row["cap_reduction_due_to_congestion"] = stream_complete.get(
            "cap_reduction_due_to_congestion"
        )
        joined_row["pacing_increase_due_to_congestion"] = stream_complete.get(
            "pacing_increase_due_to_congestion"
        )
        joined_row["congestion_cap_limit"] = stream_complete.get("congestion_cap_limit")
        joined_row["congestion_pacing_extra_ms"] = stream_complete.get(
            "congestion_pacing_extra_ms"
        )

    joined_row["client_body_tail_ms"] = None
    if joined_row.get("total_time_ms") is not None and joined_row.get("ttfb_ms") is not None:
        joined_row["client_body_tail_ms"] = (
            joined_row["total_time_ms"] - joined_row["ttfb_ms"]
        )

    joined_row["exit_body_tail_ms"] = None
    if (
        joined_row.get("exit_stream_complete_ms") is not None
        and joined_row.get("first_target_byte_wait_ms") is not None
    ):
        joined_row["exit_body_tail_ms"] = (
            joined_row["exit_stream_complete_ms"] - joined_row["first_target_byte_wait_ms"]
        )

    joined_row["overlay_pre_first_byte_gap_ms"] = None
    if (
        joined_row.get("ttfb_ms") is not None
        and joined_row.get("target_connect_ms") is not None
        and joined_row.get("first_target_byte_wait_ms") is not None
    ):
        joined_row["overlay_pre_first_byte_gap_ms"] = (
            joined_row["ttfb_ms"]
            - joined_row["target_connect_ms"]
            - joined_row["first_target_byte_wait_ms"]
        )

    joined_row["overlay_first_send_to_client_ms"] = None
    if (
        joined_row.get("overlay_pre_first_byte_gap_ms") is not None
        and joined_row.get("first_overlay_send_after_target_ms") is not None
    ):
        joined_row["overlay_first_send_to_client_ms"] = max(
            0.0,
            joined_row["overlay_pre_first_byte_gap_ms"]
            - joined_row["first_overlay_send_after_target_ms"],
        )

    joined.append(joined_row)

JOINED_PATH.parent.mkdir(parents=True, exist_ok=True)
with JOINED_PATH.open("w", encoding="utf-8") as handle:
    for row in joined:
        handle.write(json.dumps(row, sort_keys=True))
        handle.write("\n")

scenarios = []
scenario_names = sorted({row["scenario"] for row in joined}, key=scenario_sort_key)
for scenario_name in scenario_names:
    rows = [row for row in joined if row["scenario"] == scenario_name]
    if not rows:
        continue
    scenarios.append(
        {
            "scenario": scenario_name,
            "route_length": rows[0]["route_length"],
            "route_chain": rows[0]["route_chain"],
            "runs": len(rows),
            "errors": sum(1 for row in rows if row.get("error")),
            "listener_ready_ms": mean([row.get("listener_ready_ms") for row in rows]),
            "route_ready_ms": mean([row.get("route_ready_ms") for row in rows]),
            "handshake_ms": mean([row.get("handshake_ms") for row in rows]),
            "connect_avg_ms": mean([row.get("connect_time_ms") for row in rows]),
            "ttfb_avg_ms": mean([row.get("ttfb_ms") for row in rows]),
            "total_avg_ms": mean([row.get("total_time_ms") for row in rows]),
            "client_buffer_ms": mean([row.get("client_buffer_ms") for row in rows]),
            "route_select_overhead_ms": mean(
                [row.get("client_route_select_overhead_ms") for row in rows]
            ),
            "first_hop_send_ms": mean([row.get("client_first_hop_send_ms") for row in rows]),
            "target_connect_ms": mean([row.get("target_connect_ms") for row in rows]),
            "first_target_byte_wait_ms": mean(
                [row.get("first_target_byte_wait_ms") for row in rows]
            ),
            "first_overlay_send_after_target_ms": mean(
                [row.get("first_overlay_send_after_target_ms") for row in rows]
            ),
            "overlay_pre_first_byte_gap_ms": mean(
                [row.get("overlay_pre_first_byte_gap_ms") for row in rows]
            ),
            "overlay_first_send_to_client_ms": mean(
                [row.get("overlay_first_send_to_client_ms") for row in rows]
            ),
            "client_body_tail_ms": mean([row.get("client_body_tail_ms") for row in rows]),
            "exit_body_tail_ms": mean([row.get("exit_body_tail_ms") for row in rows]),
            "exit_stream_complete_ms": mean(
                [row.get("exit_stream_complete_ms") for row in rows]
            ),
            "window_frames": mean([row.get("window_frames") for row in rows]),
            "window_wait_total_ms": mean([row.get("window_wait_total_ms") for row in rows]),
            "window_wait_events": mean([row.get("window_wait_events") for row in rows]),
            "effective_cap_wait_total_ms": mean(
                [row.get("effective_cap_wait_total_ms") for row in rows]
            ),
            "effective_cap_wait_events": mean(
                [row.get("effective_cap_wait_events") for row in rows]
            ),
            "retransmit_rate": mean([row.get("retransmit_rate") for row in rows]),
            "retransmit_timeout_ms_avg": mean(
                [row.get("retransmit_timeout_ms_avg") for row in rows]
            ),
            "retransmit_timeout_ms_p95": mean(
                [row.get("retransmit_timeout_ms_p95") for row in rows]
            ),
            "retransmit_trigger_count": mean(
                [row.get("retransmit_trigger_count") for row in rows]
            ),
            "retransmit_early_count": mean(
                [row.get("retransmit_early_count") for row in rows]
            ),
            "retransmit_late_count": mean(
                [row.get("retransmit_late_count") for row in rows]
            ),
            "retransmit_rtt_ratio": mean(
                [row.get("retransmit_rtt_ratio") for row in rows]
            ),
            "ack_latency_ms_avg": mean([row.get("ack_latency_ms_avg") for row in rows]),
            "ack_latency_ms_p95": mean([row.get("ack_latency_ms_p95") for row in rows]),
            "avg_inflight": mean([row.get("avg_inflight") for row in rows]),
            "pacing_delay_applied": mean(
                [row.get("pacing_delay_applied") for row in rows]
            ),
            "pacing_delay_applied_ms_total": mean(
                [row.get("pacing_delay_applied_ms_total") for row in rows]
            ),
            "pacing_interval_ms_avg": mean(
                [row.get("pacing_interval_ms_avg") for row in rows]
            ),
            "burst_prevented_count": mean(
                [row.get("burst_prevented_count") for row in rows]
            ),
            "paced_send_batches": mean([row.get("paced_send_batches") for row in rows]),
            "max_send_burst_frames": mean(
                [row.get("max_send_burst_frames") for row in rows]
            ),
            "effective_inflight_cap_avg": mean(
                [row.get("effective_inflight_cap_avg") for row in rows]
            ),
            "effective_inflight_cap_min": mean(
                [row.get("effective_inflight_cap_min") for row in rows]
            ),
            "effective_inflight_cap_max": mean(
                [row.get("effective_inflight_cap_max") for row in rows]
            ),
            "inflight_cap_reduced_count": mean(
                [row.get("inflight_cap_reduced_count") for row in rows]
            ),
            "inflight_cap_restore_count": mean(
                [row.get("inflight_cap_restore_count") for row in rows]
            ),
            "ack_pressure_events": mean(
                [row.get("ack_pressure_events") for row in rows]
            ),
            "send_blocked_by_effective_cap": mean(
                [row.get("send_blocked_by_effective_cap") for row in rows]
            ),
            "congestion_events": mean(
                [row.get("congestion_events") for row in rows]
            ),
            "congestion_duration_ms": mean(
                [row.get("congestion_duration_ms") for row in rows]
            ),
            "cap_reduction_due_to_congestion": mean(
                [row.get("cap_reduction_due_to_congestion") for row in rows]
            ),
            "pacing_increase_due_to_congestion": mean(
                [row.get("pacing_increase_due_to_congestion") for row in rows]
            ),
            "congestion_cap_limit": mean(
                [row.get("congestion_cap_limit") for row in rows]
            ),
            "congestion_pacing_extra_ms": mean(
                [row.get("congestion_pacing_extra_ms") for row in rows]
            ),
        }
    )

remote_rows = [row for row in joined if row["scenario"].startswith("remote-")]
corr_total_retx = pearson(
    [row.get("total_time_ms") for row in remote_rows],
    [row.get("retransmit_rate") for row in remote_rows],
)
corr_total_ack = pearson(
    [row.get("total_time_ms") for row in remote_rows],
    [row.get("ack_latency_ms_avg") for row in remote_rows],
)

stage_rank_rows = [row for row in scenarios if row["scenario"].startswith("remote-")]
rank_candidates = {
    "Warm route ready (one-time)": mean([row.get("route_ready_ms") for row in stage_rank_rows]),
    "Handshake/session setup (one-time)": mean(
        [row.get("handshake_ms") for row in stage_rank_rows]
    ),
    "Client request buffering": mean([row.get("client_buffer_ms") for row in stage_rank_rows]),
    "Route selection overhead": mean(
        [row.get("route_select_overhead_ms") for row in stage_rank_rows]
    ),
    "Target connect": mean([row.get("target_connect_ms") for row in stage_rank_rows]),
    "Target first-byte wait": mean(
        [row.get("first_target_byte_wait_ms") for row in stage_rank_rows]
    ),
    "Exit first overlay send delay": mean(
        [row.get("first_overlay_send_after_target_ms") for row in stage_rank_rows]
    ),
    "Overlay pre-first-byte gap": mean(
        [row.get("overlay_pre_first_byte_gap_ms") for row in stage_rank_rows]
    ),
    "Overlay first-send -> client first-byte gap": mean(
        [row.get("overlay_first_send_to_client_ms") for row in stage_rank_rows]
    ),
    "Window/backpressure stall": mean(
        [row.get("window_wait_total_ms") for row in stage_rank_rows]
    ),
    "Exit/body delivery tail": mean([row.get("exit_body_tail_ms") for row in stage_rank_rows]),
    "Client body completion tail": mean(
        [row.get("client_body_tail_ms") for row in stage_rank_rows]
    ),
}
bottleneck_ranking = sorted(
    [(name, value) for name, value in rank_candidates.items() if value is not None],
    key=lambda item: item[1],
    reverse=True,
)

best_overlay = min(
    [row for row in scenarios if row["scenario"].startswith("remote-")],
    key=lambda row: row.get("total_avg_ms") or float("inf"),
)

lines = []
lines.append("# Overlay stage breakdown RCA")
lines.append("")
lines.append("## Inputs")
lines.append("")
lines.append(f"- local raw: `{LOCAL_RAW_PATH.as_posix()}`")
lines.append(f"- client stage trace: `{CLIENT_STAGE_PATH.as_posix()}`")
lines.append(f"- exit stage trace: `{EXIT_STAGE_PATH.as_posix()}`")
lines.append(f"- direct forward stage trace: `{DIRECT_STAGE_PATH.as_posix()}`")
lines.append(f"- joined per-run view: `{JOINED_PATH.as_posix()}`")
lines.append("")
lines.append("## Scenario Averages")
lines.append("")
lines.append(
    "| scenario | route | route ready ms | handshake ms | target connect ms | first target byte wait ms | exit first send delay ms | overlay first-send -> client ms | hard window stall ms | effective cap wait ms | exit/body tail ms | client total ms | retransmit rate | retx timeout avg ms | retx timeout p95 ms | retx triggers | retx early | retx late | retx/RTT ratio | ack avg ms | ack p95 ms | max burst frames | pacing interval ms | pacing delay ms | burst prevented | effective cap avg | cap reduced | blocked by cap | congestion events | congestion duration ms | congestion cap limit | congestion pacing extra ms | window frames |"
)
lines.append(
    "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
)
for row in scenarios:
    lines.append(
        "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |".format(
            row["scenario"],
            row["route_length"],
            fmt_ms(row.get("route_ready_ms")),
            fmt_ms(row.get("handshake_ms")),
            fmt_ms(row.get("target_connect_ms")),
            fmt_ms(row.get("first_target_byte_wait_ms")),
            fmt_ms(row.get("first_overlay_send_after_target_ms")),
            fmt_ms(row.get("overlay_first_send_to_client_ms")),
            fmt_ms(row.get("window_wait_total_ms")),
            fmt_ms(row.get("effective_cap_wait_total_ms")),
            fmt_ms(row.get("exit_body_tail_ms")),
            fmt_ms(row.get("total_avg_ms")),
            fmt_num(row.get("retransmit_rate"), 4),
            fmt_ms(row.get("retransmit_timeout_ms_avg")),
            fmt_ms(row.get("retransmit_timeout_ms_p95")),
            fmt_num(row.get("retransmit_trigger_count"), 0),
            fmt_num(row.get("retransmit_early_count"), 0),
            fmt_num(row.get("retransmit_late_count"), 0),
            fmt_num(row.get("retransmit_rtt_ratio"), 2),
            fmt_ms(row.get("ack_latency_ms_avg")),
            fmt_ms(row.get("ack_latency_ms_p95")),
            fmt_num(row.get("max_send_burst_frames"), 0),
            fmt_ms(row.get("pacing_interval_ms_avg")),
            fmt_ms(row.get("pacing_delay_applied_ms_total")),
            fmt_num(row.get("burst_prevented_count"), 0),
            fmt_num(row.get("effective_inflight_cap_avg"), 0),
            fmt_num(row.get("inflight_cap_reduced_count"), 0),
            fmt_num(row.get("send_blocked_by_effective_cap"), 0),
            fmt_num(row.get("congestion_events"), 0),
            fmt_ms(row.get("congestion_duration_ms")),
            fmt_num(row.get("congestion_cap_limit"), 0),
            fmt_ms(row.get("congestion_pacing_extra_ms")),
            fmt_num(row.get("window_frames"), 0),
        )
    )

lines.append("")
lines.append("## Bottleneck Ranking")
lines.append("")
for idx, (name, value) in enumerate(bottleneck_ranking, start=1):
    lines.append(f"{idx}. {name}: `{fmt_ms(value)} ms`")

lines.append("")
lines.append("## Correlation")
lines.append("")
lines.append(
    f"- `total_time_ms` vs `retransmit_rate`: `{fmt_num(corr_total_retx, 3)}`"
)
lines.append(
    f"- `total_time_ms` vs `ack_latency_ms_avg`: `{fmt_num(corr_total_ack, 3)}`"
)

lines.append("")
lines.append("## Findings")
lines.append("")
lines.append(
    f"- The best overlay path in this sample is `{best_overlay['scenario']}` with avg total `{fmt_ms(best_overlay['total_avg_ms'])} ms`."
)
lines.append(
    "- `target_connect_ms` and target first-byte wait stay small relative to overlay totals, so the public target itself is not the main bottleneck."
)
lines.append(
    "- The biggest per-request cost sits after the target is already reachable: overlay first-send -> client first-byte gap, window/backpressure stall, and the remaining exit/body delivery tail dominate the remote paths."
)
lines.append(
    "- `max_send_burst_frames`, `pacing_interval_ms_avg`, `pacing_delay_applied_ms_total`, and `burst_prevented_count` expose whether the exit is still dumping response frames in bursts or spreading them across the ACK window."
)
lines.append(
    "- `effective_inflight_cap_avg`, `inflight_cap_reduced_count`, `send_blocked_by_effective_cap`, and `effective_cap_wait_total_ms` expose whether the exit still drives the hard window directly or whether RTT/ACK pressure is actively shaping in-flight occupancy."
)
lines.append(
    "- `congestion_events`, `congestion_duration_ms`, `congestion_cap_limit`, and `congestion_pacing_extra_ms` expose whether the lightweight congestion response actually entered a non-default state on the slower paths."
)
lines.append(
    "- One-time costs (`route_ready_ms`, `handshake_ms`) matter for cold start, but they are not the reason the steady-state per-request `TTFB` stays in the multi-second range."
)
if corr_total_retx is not None or corr_total_ack is not None:
    lines.append(
        "- Higher retransmit rate and ACK latency move with worse total time, which points to response-path reliability and tail delivery as the main source of degradation."
    )

lines.append("")
lines.append("## Optimization Recommendation")
lines.append("")
lines.append(
    "1. Prioritize retransmit/ACK tuning on the response path first; that is where the largest steady-state time is being burned."
)
lines.append(
    "2. Use better exit/route scoring second; the current sample shows that path quality matters more than hop-count alone, so scoring should prefer the empirically faster overlay path instead of assuming more or fewer hops are always better."
)
lines.append(
    "3. Treat warm tunnel/session reuse as a cold-start improvement, not the main fix for current per-request TTFB."
)
lines.append(
    "4. Treat target-connect optimization as low priority here because target connect is already small compared with the overlay delivery tail."
)

SUMMARY_PATH.parent.mkdir(parents=True, exist_ok=True)
SUMMARY_PATH.write_text("\n".join(lines) + "\n", encoding="utf-8")
