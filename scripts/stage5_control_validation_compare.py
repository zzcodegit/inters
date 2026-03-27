#!/usr/bin/env python3
import json
import math
from pathlib import Path
from statistics import mean


ARTIFACTS = Path("docs/artifacts")
DATE_TAG = "2026-03-27"
ROUTES = ("1hop", "2hop", "3hop", "5hop")


def read_jsonl(path: Path):
    rows = []
    if not path.exists():
        raise FileNotFoundError(path)
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line:
            continue
        rows.append(json.loads(line))
    return rows


def read_json_stream(path: Path):
    if not path.exists():
        raise FileNotFoundError(path)
    text = path.read_text(encoding="utf-8", errors="replace")
    decoder = json.JSONDecoder()
    idx = 0
    rows = []
    while idx < len(text):
        while idx < len(text) and text[idx].isspace():
            idx += 1
        if idx >= len(text):
            break
        obj, end = decoder.raw_decode(text, idx)
        rows.append(obj)
        idx = end
    return rows


def percentile(values, pct):
    values = sorted(v for v in values if v is not None)
    if not values:
        return None
    if len(values) == 1:
        return values[0]
    rank = (len(values) - 1) * pct
    low = math.floor(rank)
    high = math.ceil(rank)
    if low == high:
        return values[int(rank)]
    frac = rank - low
    return values[low] * (1.0 - frac) + values[high] * frac


def fmt_ms(value):
    return "-" if value is None else f"{value:.2f}"


def fmt_num(value, digits=2):
    return "-" if value is None else f"{value:.{digits}f}"


def load_perf(prefix: str):
    raw_path = ARTIFACTS / f"stage5_control_validation_{prefix}_remote_perf_matrix_{DATE_TAG}.jsonl"
    rows = read_jsonl(raw_path)
    result = {}
    for route in ROUTES:
        scenario = f"remote-{route}"
        route_rows = [
            row for row in rows if row.get("scenario") == scenario and row.get("error") is None
        ]
        totals = [row.get("total_time_ms") for row in route_rows if row.get("total_time_ms") is not None]
        result[route] = {
            "runs": len(route_rows),
            "total_avg_ms": mean(totals) if totals else None,
            "total_p95_ms": percentile(totals, 0.95),
            "total_max_ms": max(totals) if totals else None,
        }
    return result


def load_stage(prefix: str):
    exit_path = ARTIFACTS / f"stage5_control_validation_{prefix}_remote_perf_stage_{DATE_TAG}.exit.jsonl"
    rows = read_json_stream(exit_path)
    result = {}
    for route in ROUTES:
        site_prefix = f"stage-remote-{route}-run"
        route_rows = [
            row
            for row in rows
            if row.get("component") == "exit"
            and row.get("stage") == "stream_complete"
            and str(row.get("site", "")).startswith(site_prefix)
        ]
        ack_p95 = [row.get("ack_latency_ms_p95") for row in route_rows if row.get("ack_latency_ms_p95") is not None]
        retransmit_rate = [row.get("retransmit_rate") for row in route_rows if row.get("retransmit_rate") is not None]
        avg_inflight = [row.get("avg_inflight") for row in route_rows if row.get("avg_inflight") is not None]
        max_inflight = [row.get("max_inflight") for row in route_rows if row.get("max_inflight") is not None]
        pacing = [row.get("pacing_interval_ms_avg") for row in route_rows if row.get("pacing_interval_ms_avg") is not None]
        cap = [row.get("effective_inflight_cap_avg") for row in route_rows if row.get("effective_inflight_cap_avg") is not None]
        window_wait = [row.get("window_wait_total_ms") for row in route_rows if row.get("window_wait_total_ms") is not None]
        result[route] = {
            "runs": len(route_rows),
            "ack_p95_ms": mean(ack_p95) if ack_p95 else None,
            "retransmit_rate": mean(retransmit_rate) if retransmit_rate else None,
            "avg_inflight": mean(avg_inflight) if avg_inflight else None,
            "max_inflight": max(max_inflight) if max_inflight else None,
            "pacing_interval_ms_avg": mean(pacing) if pacing else None,
            "effective_inflight_cap_avg": mean(cap) if cap else None,
            "congestion_events": sum(int(row.get("congestion_events", 0) or 0) for row in route_rows),
            "blocked_by_cap": sum(int(row.get("send_blocked_by_effective_cap", 0) or 0) for row in route_rows),
            "window_wait_total_ms": mean(window_wait) if window_wait else None,
        }
    return result


def classify_delta(control_value, baseline_value, positive_good=False, tolerance=0.0):
    if control_value is None or baseline_value is None:
        return "n/a"
    delta = control_value - baseline_value
    if abs(delta) <= tolerance:
        return "neutral"
    if positive_good:
        return "better" if delta > 0 else "worse"
    return "better" if delta < 0 else "worse"


baseline_perf = load_perf("baseline")
control_perf = load_perf("control")
baseline_stage = load_stage("baseline")
control_stage = load_stage("control")

compare = {}
for route in ROUTES:
    bperf = baseline_perf[route]
    cperf = control_perf[route]
    bstage = baseline_stage[route]
    cstage = control_stage[route]
    total_delta_pct = None
    if bperf["total_avg_ms"] not in (None, 0) and cperf["total_avg_ms"] is not None:
        total_delta_pct = ((cperf["total_avg_ms"] - bperf["total_avg_ms"]) / bperf["total_avg_ms"]) * 100.0
    compare[route] = {
        "baseline": {**bperf, **bstage},
        "control": {**cperf, **cstage},
        "delta_total_pct": total_delta_pct,
        "total_class": classify_delta(cperf["total_avg_ms"], bperf["total_avg_ms"]),
        "ack_class": classify_delta(cstage["ack_p95_ms"], bstage["ack_p95_ms"]),
        "retransmit_class": classify_delta(
            cstage["retransmit_rate"], bstage["retransmit_rate"]
        ),
        "stall_class": classify_delta(
            cstage["window_wait_total_ms"], bstage["window_wait_total_ms"]
        ),
    }


accepted = True
reasons = []

route1_delta = compare["1hop"]["delta_total_pct"]
if route1_delta is None or route1_delta > 5.0:
    accepted = False
    reasons.append("1-hop total_time_ms regressed by more than 5% or is unavailable")

route2_delta = compare["2hop"]["delta_total_pct"]
if route2_delta is None or route2_delta > 0.0:
    accepted = False
    reasons.append("2-hop total_time_ms regressed versus baseline")

route3_delta = compare["3hop"]["delta_total_pct"]
if route3_delta is None or route3_delta > 0.0:
    accepted = False
    reasons.append("3-hop total_time_ms is worse than baseline")

route5 = compare["5hop"]
route5_helped = any(
    [
        route5["retransmit_class"] == "better",
        route5["ack_class"] == "better",
        route5["stall_class"] == "better",
    ]
)
if not route5_helped:
    accepted = False
    reasons.append("5-hop shows no improvement in retransmit rate, ACK p95, or stall")

verdict = "ACCEPTED" if accepted else "REJECTED"

json_output = {
    "verdict": verdict,
    "reasons": reasons,
    "routes": compare,
}
(ARTIFACTS / f"stage5_control_validation_compare_{DATE_TAG}.json").write_text(
    json.dumps(json_output, indent=2),
    encoding="utf-8",
)

lines = [
    "# Stage 5 Control Validation v1",
    "",
    "## Scope",
    "",
    "- A: baseline v2 exact-route reduced fleet",
    "- B: current control exact-route-compatible branch",
    "- routes: `1-hop`, `2-hop`, `3-hop`, `5-hop`",
    "- each route was run under the existing reset-per-route harness",
    "",
    "## Metrics Table",
    "",
    "| Route | Version | total avg ms | total p95 ms | total max ms | ack p95 ms | retransmit_rate | inflight avg | inflight max | pacing interval ms | effective cap avg | congestion_events | blocked_by_cap |",
    "| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
]

for route in ROUTES:
    for version in ("baseline", "control"):
        row = compare[route][version]
        lines.append(
            f"| `{route}` | `{version}` | "
            f"{fmt_ms(row['total_avg_ms'])} | {fmt_ms(row['total_p95_ms'])} | {fmt_ms(row['total_max_ms'])} | "
            f"{fmt_ms(row['ack_p95_ms'])} | {fmt_num(row['retransmit_rate'], 4)} | "
            f"{fmt_num(row['avg_inflight'])} | {fmt_num(row['max_inflight'])} | "
            f"{fmt_num(row['pacing_interval_ms_avg'])} | {fmt_num(row['effective_inflight_cap_avg'])} | "
            f"{row['congestion_events']} | {row['blocked_by_cap']} |"
        )

lines += [
    "",
    "## Route-by-Route Outcome",
    "",
]

for route in ROUTES:
    row = compare[route]
    delta = row["delta_total_pct"]
    delta_str = "-" if delta is None else f"{delta:+.2f}%"
    lines += [
        f"### `{route}`",
        "",
        f"- total: `{row['total_class']}` ({delta_str})",
        f"- ack p95: `{row['ack_class']}`",
        f"- retransmit_rate: `{row['retransmit_class']}`",
        f"- stall: `{row['stall_class']}`",
        "",
    ]

lines += [
    "## Honest Readout",
    "",
]

for route in ROUTES:
    total_class = compare[route]["total_class"]
    if total_class == "better":
        verdict_word = "better"
    elif total_class == "worse":
        verdict_word = "worse"
    else:
        verdict_word = "neutral"
    lines.append(f"- `{route}`: total behavior is `{verdict_word}` versus baseline")

lines += [
    "",
    "## Verdict",
    "",
    f"`CONTROL = {verdict}`",
]

if reasons:
    lines += [
        "",
        "Reasons:",
        "",
    ]
    lines.extend(f"- {reason}" for reason in reasons)

(ARTIFACTS / f"stage5_control_validation_compare_{DATE_TAG}.md").write_text(
    "\n".join(lines) + "\n",
    encoding="utf-8",
)
