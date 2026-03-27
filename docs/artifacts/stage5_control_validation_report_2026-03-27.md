# Stage 5 Control Validation v1 Report

## Summary

- Reference `A` is baseline v2 exact-route reduced fleet from [f2847ba693da9be0d0da05df9563bbf458f7701a](https://github.com/zzcodegit/inters/commit/f2847ba693da9be0d0da05df9563bbf458f7701a).
- Candidate `B` is current control logic from `f9583ff` with exact-route compatibility from baseline v2, and no extra control tuning beyond that compatibility patch.
- Both sides were measured on the same reduced fleet:
  - `31.192.232.26:30000`
  - `45.197.133.115:30001`
  - `185.144.28.95:30002`
  - `155.212.135.200:30003`
  - `155.212.135.95:30004`
  - `155.212.135.95:30006`
- Required routes were measured under the existing reset-per-route harness:
  - `1-hop`
  - `2-hop`
  - `3-hop`
  - `5-hop`

## Version A Artifacts

- build: [stage5_control_validation_baseline_build_2026-03-27.log](/C:/neinternet/vpnnode_stage5_control_validation_v1/docs/artifacts/stage5_control_validation_baseline_build_2026-03-27.log)
- deploy: [stage5_control_validation_baseline_deploy_2026-03-27.log](/C:/neinternet/vpnnode_stage5_control_validation_v1/docs/artifacts/stage5_control_validation_baseline_deploy_2026-03-27.log)
- perf matrix raw: [stage5_control_validation_baseline_remote_perf_matrix_2026-03-27.jsonl](/C:/neinternet/vpnnode_stage5_control_validation_v1/docs/artifacts/stage5_control_validation_baseline_remote_perf_matrix_2026-03-27.jsonl)
- perf matrix report: [stage5_control_validation_baseline_remote_perf_matrix_2026-03-27.md](/C:/neinternet/vpnnode_stage5_control_validation_v1/docs/artifacts/stage5_control_validation_baseline_remote_perf_matrix_2026-03-27.md)
- perf stage log: [stage5_control_validation_baseline_remote_perf_stage_2026-03-27.log](/C:/neinternet/vpnnode_stage5_control_validation_v1/docs/artifacts/stage5_control_validation_baseline_remote_perf_stage_2026-03-27.log)
- perf stage local: [stage5_control_validation_baseline_remote_perf_stage_2026-03-27.local.jsonl](/C:/neinternet/vpnnode_stage5_control_validation_v1/docs/artifacts/stage5_control_validation_baseline_remote_perf_stage_2026-03-27.local.jsonl)
- perf stage client: [stage5_control_validation_baseline_remote_perf_stage_2026-03-27.client.jsonl](/C:/neinternet/vpnnode_stage5_control_validation_v1/docs/artifacts/stage5_control_validation_baseline_remote_perf_stage_2026-03-27.client.jsonl)
- perf stage exit: [stage5_control_validation_baseline_remote_perf_stage_2026-03-27.exit.jsonl](/C:/neinternet/vpnnode_stage5_control_validation_v1/docs/artifacts/stage5_control_validation_baseline_remote_perf_stage_2026-03-27.exit.jsonl)

## Version B Artifacts

- build: [stage5_control_validation_control_build_2026-03-27.log](/C:/neinternet/vpnnode_stage5_control_validation_v1/docs/artifacts/stage5_control_validation_control_build_2026-03-27.log)
- deploy: [stage5_control_validation_control_deploy_2026-03-27.log](/C:/neinternet/vpnnode_stage5_control_validation_v1/docs/artifacts/stage5_control_validation_control_deploy_2026-03-27.log)
- perf matrix raw: [stage5_control_validation_control_remote_perf_matrix_2026-03-27.jsonl](/C:/neinternet/vpnnode_stage5_control_validation_v1/docs/artifacts/stage5_control_validation_control_remote_perf_matrix_2026-03-27.jsonl)
- perf matrix report: [stage5_control_validation_control_remote_perf_matrix_2026-03-27.md](/C:/neinternet/vpnnode_stage5_control_validation_v1/docs/artifacts/stage5_control_validation_control_remote_perf_matrix_2026-03-27.md)
- perf stage log: [stage5_control_validation_control_remote_perf_stage_2026-03-27.log](/C:/neinternet/vpnnode_stage5_control_validation_v1/docs/artifacts/stage5_control_validation_control_remote_perf_stage_2026-03-27.log)
- perf stage local: [stage5_control_validation_control_remote_perf_stage_2026-03-27.local.jsonl](/C:/neinternet/vpnnode_stage5_control_validation_v1/docs/artifacts/stage5_control_validation_control_remote_perf_stage_2026-03-27.local.jsonl)
- perf stage client: [stage5_control_validation_control_remote_perf_stage_2026-03-27.client.jsonl](/C:/neinternet/vpnnode_stage5_control_validation_v1/docs/artifacts/stage5_control_validation_control_remote_perf_stage_2026-03-27.client.jsonl)
- perf stage exit: [stage5_control_validation_control_remote_perf_stage_2026-03-27.exit.jsonl](/C:/neinternet/vpnnode_stage5_control_validation_v1/docs/artifacts/stage5_control_validation_control_remote_perf_stage_2026-03-27.exit.jsonl)

## Comparison

- summary JSON: [stage5_control_validation_compare_2026-03-27.json](/C:/neinternet/vpnnode_stage5_control_validation_v1/docs/artifacts/stage5_control_validation_compare_2026-03-27.json)
- summary Markdown: [stage5_control_validation_compare_2026-03-27.md](/C:/neinternet/vpnnode_stage5_control_validation_v1/docs/artifacts/stage5_control_validation_compare_2026-03-27.md)

Headline result:

- `1-hop` regressed materially
- `2-hop` improved
- `3-hop` regressed
- `5-hop` reduced ACK tail and stall, but total latency still regressed strongly

## Cleanup

- service cleanup: [stage5_control_validation_remote_cleanup_services_2026-03-27.log](/C:/neinternet/vpnnode_stage5_control_validation_v1/docs/artifacts/stage5_control_validation_remote_cleanup_services_2026-03-27.log)
- exit state: [stage5_control_validation_remote_cleanup_exit_state_2026-03-27.log](/C:/neinternet/vpnnode_stage5_control_validation_v1/docs/artifacts/stage5_control_validation_remote_cleanup_exit_state_2026-03-27.log)
- exit verify: [stage5_control_validation_remote_cleanup_verify_2026-03-27.log](/C:/neinternet/vpnnode_stage5_control_validation_v1/docs/artifacts/stage5_control_validation_remote_cleanup_verify_2026-03-27.log)
- relay4/relay6 state: [stage5_control_validation_remote_cleanup_host15521213595_final_2026-03-27.log](/C:/neinternet/vpnnode_stage5_control_validation_v1/docs/artifacts/stage5_control_validation_remote_cleanup_host15521213595_final_2026-03-27.log)
- local cleanup: [stage5_control_validation_local_cleanup_2026-03-27.log](/C:/neinternet/vpnnode_stage5_control_validation_v1/docs/artifacts/stage5_control_validation_local_cleanup_2026-03-27.log)

## Final Verdict

`CONTROL = REJECTED`

Reason:

- `1-hop` total latency regressed by more than the allowed `5%`
- `3-hop` total latency regressed versus baseline
- `5-hop` congestion behavior is not decorative, but it still does not convert into an acceptable overall result against the reduced-fleet reference
