#!/usr/bin/env python3
import argparse
import json
from pathlib import Path


def load_runs(path):
    runs = []
    with open(path, "r", encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if not line:
                continue
            runs.append(json.loads(line))
    return runs


def mean(runs, key):
    values = [float(run.get(key, 0.0) or 0.0) for run in runs]
    return sum(values) / len(values) if values else 0.0


def total(runs, key):
    return sum(int(run.get(key, 0) or 0) for run in runs)


def route_distribution(runs):
    counts = {}
    for run in runs:
        for route, count in (run.get("route_counts") or {}).items():
            counts[route] = counts.get(route, 0) + int(count)
    return counts


def render(name, runs):
    best_selected = total(runs, "best_selected_count")
    selections = total(runs, "selection_count")
    best_ratio = (best_selected / selections * 100.0) if selections else 0.0
    route_counts = route_distribution(runs)
    top_route = max(route_counts, key=route_counts.get) if route_counts else "-"
    return {
        "name": name,
        "series_runs": len(runs),
        "selections": selections,
        "best_selected": best_selected,
        "best_ratio": best_ratio,
        "switches": total(runs, "switch_count"),
        "selected_route_changes": total(runs, "selected_route_change_count"),
        "best_route_changes": total(runs, "best_route_change_count"),
        "avg_score_stdev": mean(runs, "selected_score_stdev"),
        "avg_total_cv": mean(runs, "total_time_cv") * 100.0,
        "avg_ttfb_cv": mean(runs, "ttfb_cv") * 100.0,
        "top_route": top_route,
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--before-json", required=True)
    parser.add_argument("--after-json", required=True)
    parser.add_argument("--out-md", required=True)
    args = parser.parse_args()

    before_runs = load_runs(args.before_json)
    after_runs = load_runs(args.after_json)
    before = render("Before", before_runs)
    after = render("After", after_runs)

    lines = []
    lines.append("# Route-quality Stability Compare")
    lines.append("")
    lines.append("## Summary")
    lines.append("")
    lines.append("| metric | before | after |")
    lines.append("| --- | --- | --- |")
    for label, key, fmt in [
        ("series runs", "series_runs", "{}"),
        ("total selections", "selections", "{}"),
        ("best-score pick ratio %", "best_ratio", "{:.2f}"),
        ("hard switches", "switches", "{}"),
        ("selected route changes", "selected_route_changes", "{}"),
        ("best-route changes", "best_route_changes", "{}"),
        ("avg selected-score stdev", "avg_score_stdev", "{:.4f}"),
        ("avg total-time CV %", "avg_total_cv", "{:.2f}"),
        ("avg TTFB CV %", "avg_ttfb_cv", "{:.2f}"),
    ]:
        lines.append(
            f"| {label} | {fmt.format(before[key])} | {fmt.format(after[key])} |"
        )
    lines.append("")
    lines.append("## Route Distribution")
    lines.append("")
    lines.append(f"- before top route: `{before['top_route']}`")
    lines.append(f"- after top route: `{after['top_route']}`")
    lines.append("")
    lines.append("## Reading")
    lines.append("")
    lines.append(
        "This comparison focuses on stability signals, not only raw throughput. Lower selected-score stdev and lower latency CV indicate that early noise has less leverage over the selector. If hard switches were already zero in the baseline WAN sample, then the meaningful improvement is reduced score jitter and more confidence-aware reasons rather than an impossible reduction below zero."
    )
    lines.append("")

    out_path = Path(args.out_md)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text("\n".join(lines) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
