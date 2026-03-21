# Route-quality scoring follow-up

## What was verified

- local baseline stayed green
- accepted remote WAN matrix stayed green
- accepted remote perf runner stayed green
- accepted remote stage runner stayed green
- new adaptive runner `scripts/run_route_quality_remote.sh` passed on the real WAN topology

Topology used:

- `relay-1`: `45.197.133.115:30001`
- `relay-2`: `185.144.28.95:30002`
- `exit`: `31.192.232.26:30000`

## Key evidence

Adaptive route-quality run:

- log: `docs/artifacts/route_quality_remote_2026-03-21.log`
- raw measurements: `docs/artifacts/route_quality_remote_2026-03-21.jsonl`
- client stage trace: `docs/artifacts/route_quality_remote_2026-03-21.client.jsonl`
- report: `docs/artifacts/route_quality_remote_2026-03-21.md`

Regression checks:

- matrix log: `docs/artifacts/remote_wan_matrix_2026-03-21_route_scoring.log`
- perf log: `docs/artifacts/remote_perf_matrix_2026-03-21_route_scoring.log`
- perf report: `docs/artifacts/remote_perf_matrix_2026-03-21_route_scoring.md`
- stage log: `docs/artifacts/remote_perf_stage_matrix_2026-03-21_route_scoring.log`
- stage report: `docs/artifacts/remote_perf_stage_matrix_2026-03-21_route_scoring.md`

Build and deploy:

- target add: `docs/artifacts/route_quality_target_add_2026-03-21.log`
- cross-build with `rust-lld`: `docs/artifacts/route_quality_cross_build_lld_2026-03-21.log`
- deploy: `docs/artifacts/route_quality_deploy_2026-03-21.log`

## What the adaptive run proved

From `docs/artifacts/route_quality_remote_2026-03-21.md`:

- multi-candidate score events: `7`
- selections with quality metrics attached: `6`
- feedback events received from exit: `27`
- non-shortest selections observed: `7`

That last line is the important proof point: the system did not collapse back to
"shortest path wins". In this live sample it selected the 3-hop path over the
shorter 1-hop and 2-hop candidates.

Representative scored snapshot:

- 3-hop final score: `0.7640`
- 1-hop final score: `0.3020`
- 2-hop final score: `0.2988`

The selected 3-hop candidate had attached quality metrics in the selection log:

- `recent_total_ms=1382`
- `recent_ttfb_ms=484`
- `recent_ack_p95_ms=246`
- `recent_retransmit_rate_ppm=0`
- `recent_window_wait_ratio_ppm=850574`

So the winning route was chosen with response-path quality data attached, not on hop-count luck.

## Why the result is honest

The accepted exact-route perf matrix was rerun independently and stayed green.
That matrix is still the cleanest apples-to-apples view of isolated 1-hop/2-hop/3-hop costs.

The new adaptive run answers a different question:

- not "which exact route is fastest in isolated replay"
- but "does the client keep a live candidate set, score it, and choose a route by quality instead of shortest-path bias"

That is exactly what the stage trace now shows.

## Remaining limitation

The current scoring policy is quality-aware, but not omniscient. It reacts
strongly to recent failure signals, which means early negative samples can keep
shorter routes down-ranked for a while even when they later look healthier in an
isolated exact-route benchmark.

So the result of this step is:

- yes, route choice is now visibly quality-driven and auditable
- yes, non-shortest WAN routes can win
- no, the current policy should not yet be presented as mathematically optimal under every traffic window
