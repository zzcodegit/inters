# Recovery-Coupled Control Route Proof

## Exact Routes

All runs below used a full reset immediately before the isolated routeproof run.

Reset logs:

- `docs/artifacts/recovery_coupled_control_routeproof_reset_1hop_2026-03-28.log`
- `docs/artifacts/recovery_coupled_control_routeproof_reset_2hop_2026-03-28.log`
- `docs/artifacts/recovery_coupled_control_routeproof_reset_3hop_2026-03-28.log`
- `docs/artifacts/recovery_coupled_control_routeproof_reset_5hop_2026-03-28.log`

## `1-hop`

Declared route:

- `client -> 31.192.232.26:30000 -> target`

Proof:

- run log: `docs/artifacts/recovery_coupled_control_routeproof_run_1hop_2026-03-28.log`
- exit peer: `Udp://217.173.65.131:*` in `docs/artifacts/recovery_coupled_control_routeproof_1hop_exit_journal_2026-03-28.log`
- idle relays:
  - `relay1`: `docs/artifacts/recovery_coupled_control_routeproof_1hop_relay1_journal_2026-03-28.log`
  - `relay2`: `docs/artifacts/recovery_coupled_control_routeproof_1hop_relay2_journal_2026-03-28.log`
  - `relay3`: `docs/artifacts/recovery_coupled_control_routeproof_1hop_relay3_journal_2026-03-28.log`
  - `relay4`: `docs/artifacts/recovery_coupled_control_routeproof_1hop_relay4_journal_2026-03-28.log`
  - `relay6`: `docs/artifacts/recovery_coupled_control_routeproof_1hop_relay6_journal_2026-03-28.log`

## `2-hop`

Declared route:

- `client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`

Proof:

- run log: `docs/artifacts/recovery_coupled_control_routeproof_run_2hop_2026-03-28.log`
- exit peer: `Udp://45.197.133.115:30001` in `docs/artifacts/recovery_coupled_control_routeproof_2hop_exit_journal_2026-03-28.log`
- active relay:
  - `relay1`: `docs/artifacts/recovery_coupled_control_routeproof_2hop_relay1_journal_2026-03-28.log`
- idle relays:
  - `relay2`: `docs/artifacts/recovery_coupled_control_routeproof_2hop_relay2_journal_2026-03-28.log`
  - `relay3`: `docs/artifacts/recovery_coupled_control_routeproof_2hop_relay3_journal_2026-03-28.log`
  - `relay4`: `docs/artifacts/recovery_coupled_control_routeproof_2hop_relay4_journal_2026-03-28.log`
  - `relay6`: `docs/artifacts/recovery_coupled_control_routeproof_2hop_relay6_journal_2026-03-28.log`

## `3-hop`

Declared route:

- `client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`

Proof:

- run log: `docs/artifacts/recovery_coupled_control_routeproof_run_3hop_2026-03-28.log`
- exit peer: `Udp://185.144.28.95:30002` in `docs/artifacts/recovery_coupled_control_routeproof_3hop_exit_journal_2026-03-28.log`
- active relays:
  - `relay1`: `docs/artifacts/recovery_coupled_control_routeproof_3hop_relay1_journal_2026-03-28.log`
  - `relay2`: `docs/artifacts/recovery_coupled_control_routeproof_3hop_relay2_journal_2026-03-28.log`
- idle relays:
  - `relay3`: `docs/artifacts/recovery_coupled_control_routeproof_3hop_relay3_journal_2026-03-28.log`
  - `relay4`: `docs/artifacts/recovery_coupled_control_routeproof_3hop_relay4_journal_2026-03-28.log`
  - `relay6`: `docs/artifacts/recovery_coupled_control_routeproof_3hop_relay6_journal_2026-03-28.log`

## `5-hop`

Declared route:

- `client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 155.212.135.200:30003 -> 155.212.135.95:30004 -> 31.192.232.26:30000 -> target`

Proof:

- run log: `docs/artifacts/recovery_coupled_control_routeproof_run_5hop_2026-03-28.log`
- exit peer: `Udp://155.212.135.95:30004` in `docs/artifacts/recovery_coupled_control_routeproof_5hop_exit_journal_2026-03-28.log`
- active relays:
  - `relay1`: `docs/artifacts/recovery_coupled_control_routeproof_5hop_relay1_journal_2026-03-28.log`
  - `relay2`: `docs/artifacts/recovery_coupled_control_routeproof_5hop_relay2_journal_2026-03-28.log`
  - `relay3`: `docs/artifacts/recovery_coupled_control_routeproof_5hop_relay3_journal_2026-03-28.log`
  - `relay4`: `docs/artifacts/recovery_coupled_control_routeproof_5hop_relay4_journal_2026-03-28.log`
- idle relay:
  - `relay6`: `docs/artifacts/recovery_coupled_control_routeproof_5hop_relay6_journal_2026-03-28.log`

## Conclusion

The reduced exact-route fleet remains correct under the current branch:

- exit peer always matches the declared previous hop
- non-route relays remain idle
- no hidden hop was observed in `1-hop`, `2-hop`, `3-hop`, or `5-hop`
