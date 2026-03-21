#!/usr/bin/env python3
import argparse
import glob
import json
from collections import Counter, defaultdict
from pathlib import Path


def load_stage_trace(path):
    candidates = {}
    selections = []
    with open(path, "r", encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if not line:
                continue
            event = json.loads(line)
            stage = event.get("stage")
            if stage == "route_candidates_scored":
                candidates[(event.get("stream_id"), event.get("attempt"))] = event.get(
                    "candidates", []
                )
            elif stage == "route_selected":
                key = (event.get("stream_id"), event.get("attempt"))
                snapshot = candidates.get(key, [])
                best_route = None
                if snapshot and event.get("best_idx") is not None:
                    for candidate in snapshot:
                        if candidate.get("candidate_idx") == event.get("best_idx"):
                            best_route = candidate.get("route_chain")
                            break
                if best_route is None and snapshot:
                    best_route = max(
                        snapshot,
                        key=lambda candidate: candidate.get("final_score", 0.0),
                    ).get("route_chain")
                selections.append(
                    {
                        "stream_id": event.get("stream_id"),
                        "attempt": event.get("attempt"),
                        "selected_route": event.get("route_chain"),
                        "route_len": event.get("route_len"),
                        "best_route": best_route,
                        "score": event.get("score", 0.0),
                        "decision_reason": event.get("decision_reason", "unknown"),
                        "tie_break_reason": event.get("tie_break_reason"),
                        "switched": bool(event.get("switched", False)),
                        "score_delta_abs": event.get("score_delta_abs"),
                        "score_delta_ratio": event.get("score_delta_ratio"),
                        "required_abs_margin": event.get("required_abs_margin"),
                        "required_rel_margin": event.get("required_rel_margin"),
                        "hold_remaining_ms": event.get("hold_remaining_ms"),
                        "candidates": snapshot,
                    }
                )
    return selections


def summarize_run(path):
    selections = load_stage_trace(path)
    route_counts = Counter(selection["selected_route"] for selection in selections)
    reason_counts = Counter(selection["decision_reason"] for selection in selections)
    best_selected = sum(
        1
        for selection in selections
        if selection["best_route"] and selection["best_route"] == selection["selected_route"]
    )
    return {
        "run_name": Path(path).stem,
        "path": path,
        "selection_count": len(selections),
        "route_counts": dict(route_counts),
        "reason_counts": dict(reason_counts),
        "switch_count": sum(1 for selection in selections if selection["switched"]),
        "best_selected_count": best_selected,
        "best_selected_ratio": (
            best_selected / len(selections) if selections else 0.0
        ),
        "first_selection": selections[0] if selections else None,
    }


def render_markdown(runs):
    overall_route_counts = Counter()
    overall_reason_counts = Counter()
    total_selections = 0
    total_best_selected = 0
    total_switches = 0
    for run in runs:
        overall_route_counts.update(run["route_counts"])
        overall_reason_counts.update(run["reason_counts"])
        total_selections += run["selection_count"]
        total_best_selected += run["best_selected_count"]
        total_switches += run["switch_count"]

    lines = []
    lines.append("# Remote WAN Route-Quality Series")
    lines.append("")
    lines.append("## Summary")
    lines.append("")
    lines.append(
        f"- series runs: `{len(runs)}`"
    )
    lines.append(f"- total selections: `{total_selections}`")
    lines.append(f"- selections that picked current best-score route: `{total_best_selected}`")
    ratio = (total_best_selected / total_selections * 100.0) if total_selections else 0.0
    lines.append(f"- best-score pick ratio: `{ratio:.2f}%`")
    lines.append(f"- total switches: `{total_switches}`")
    lines.append("- note: selection counts include the initial readiness/probe stream for each independent run")
    lines.append("")
    lines.append("Selection distribution:")
    lines.append("")
    for route, count in overall_route_counts.most_common():
        lines.append(f"- `{route}`: `{count}`")
    lines.append("")
    lines.append("Decision reasons:")
    lines.append("")
    for reason, count in overall_reason_counts.most_common():
        lines.append(f"- `{reason}`: `{count}`")
    lines.append("")
    lines.append("## Per-run Distribution")
    lines.append("")
    lines.append(
        "| run | selections | best-score picks | switches | selected route distribution | decision reasons |"
    )
    lines.append("| --- | --- | --- | --- | --- | --- |")
    for run in runs:
        route_dist = ", ".join(
            f"`{route}` x{count}" for route, count in sorted(run["route_counts"].items())
        )
        reason_dist = ", ".join(
            f"`{reason}` x{count}" for reason, count in sorted(run["reason_counts"].items())
        )
        lines.append(
            f"| {run['run_name']} | {run['selection_count']} | {run['best_selected_count']}/{run['selection_count']} | {run['switch_count']} | {route_dist or '-'} | {reason_dist or '-'} |"
        )
    lines.append("")
    lines.append("## First Selection Snapshot Per Run")
    lines.append("")
    for run in runs:
        first = run["first_selection"]
        if not first:
            continue
        lines.append(f"### {run['run_name']}")
        lines.append("")
        lines.append(
            f"- selected route: `{first['selected_route']}`"
        )
        lines.append(f"- best-score route: `{first['best_route']}`")
        lines.append(f"- decision reason: `{first['decision_reason']}`")
        lines.append(f"- switched: `{first['switched']}`")
        lines.append(
            f"- score delta abs / rel: `{(first['score_delta_abs'] or 0.0):.4f}` / `{((first['score_delta_ratio'] or 0.0) * 100.0):.2f}%`"
        )
        lines.append(
            "| route len | candidate route | final score | recent total ms | recent ACK p95 ms | retransmit ppm | stall ppm |"
        )
        lines.append("| --- | --- | --- | --- | --- | --- | --- |")
        for candidate in first["candidates"]:
            lines.append(
                "| {} | `{}` | {:.4f} | {} | {} | {} | {} |".format(
                    candidate.get("route_len", "-"),
                    candidate.get("route_chain", "<missing>"),
                    float(candidate.get("final_score", 0.0)),
                    candidate.get("recent_total_ms", "-"),
                    candidate.get("recent_ack_p95_ms", "-"),
                    candidate.get("recent_retransmit_rate_ppm", "-"),
                    candidate.get("recent_window_wait_ratio_ppm", "-"),
                )
            )
        lines.append("")
    lines.append("## Conclusion")
    lines.append("")
    lines.append(
        "The WAN series shows whether route choice is caused by the live quality scoreboard rather than pure shortest-path bias. When `decision_reason` is `current_still_best` or `switch_margin_exceeded`, the selected route equals the top-scored route. When `decision_reason` is `hold_time_active` or `within_hysteresis_margin`, the scoreboard still records the challenger, but the selector intentionally suppresses a flap."
    )
    lines.append("")
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--stage-glob", required=True)
    parser.add_argument("--json-out", required=True)
    parser.add_argument("--md-out", required=True)
    args = parser.parse_args()

    paths = sorted(glob.glob(args.stage_glob))
    if not paths:
        raise SystemExit(f"no stage traces matched {args.stage_glob}")

    runs = [summarize_run(path) for path in paths]

    Path(args.json_out).parent.mkdir(parents=True, exist_ok=True)
    with open(args.json_out, "w", encoding="utf-8") as fh:
        for run in runs:
            fh.write(json.dumps(run, ensure_ascii=True) + "\n")

    Path(args.md_out).parent.mkdir(parents=True, exist_ok=True)
    with open(args.md_out, "w", encoding="utf-8") as fh:
        fh.write(render_markdown(runs) + "\n")


if __name__ == "__main__":
    main()
