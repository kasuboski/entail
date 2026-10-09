# Author engine v0 end-user acceptance

**Status: black-box CLI and worker acceptance passed at integration revision `758d32d`.** These are real subprocess checks against the integration executable, not engine mocks. Fixtures and expected outcomes in `author-engine-v0/fixtures` remain contracts, not evidence by themselves.

## Run

Run inside the pinned Nix development shell so subprocesses inherit the native Z3 runtime path:

```sh
nix develop --command cargo build --workspace --bin entail --locked
nix develop --command python3 -m unittest scripts.test_acceptance -v
nix develop --command python3 scripts/acceptance.py target/debug/entail
nix develop --command python3 scripts/worker_acceptance.py target/debug/entail
```

Each subprocess has a finite timeout. Helper tests and either script's `--self-test` exercise only local assertion/framing/process utilities; they do not count as application acceptance.

`acceptance.py` checks valid and invalid fixtures, stable diagnostic codes/exit 2, analysis exit 0, exact typed obligation categories, mandatory domain feasibility, reachability per rule, exact rule/constraint compliance IDs, report identity/hashes/assumptions/open items, proposed-policy context and `complete=true` without claiming business verification. It exercises conditional assumptions both explicitly empty and selected, scenario evaluation, unsafe-witness replay, arbitrary-precision output, and structured CLI errors. Its payload case ties the conflicting typed rule pair to independently inspected model constructors and replays the returned witness to ensure it produces distinct `Grant` payloads.

The worker script covers framed I/O, operation envelopes, valid/invalid compilation, analysis/evaluation/release, live-handle capacity without eviction, malformed JSON recovery, protocol/version/frame errors, process kill/wait, restart, and stale-handle rejection. It rejects capacity bounds beyond its safe 128-model probe. Worker process waits and frame reads are bounded.

## Criteria map

| Spec criterion | Evidence | Status |
| --- | --- | --- |
| §9.1 Partial model: `behavior_unspecified` | `partial`, typed coverage category | Passed |
| §9.2 Unsafe Grant violates expiry independently of guard | `unsafe-grant`, exact rule/constraint result and evaluator replay | Passed |
| §9.3 Guard repair removes violation but leaves uncovered inputs | `repaired-partial`, per-rule compliance and coverage | Passed |
| §9.4 Complete proposed policy | `complete-proposed`: coverage/conflict, complete report, proposed IDs/open items; rejects `business_verified=true` | Passed |
| §9.5 Simultaneous Grant/Deny conflicts | `conflict-tag`, typed conflict obligation | Passed |
| §9.6 Identical outcomes deduplicate | `duplicate-output`, compatible outcome | Passed |
| §9.7 Same constructor with distinct payloads conflicts | `conflict-payload`, typed pair plus independent evaluator replay | Passed |
| §9.8 Assumptions are opt-in and visible | Explicit empty and selected-assumption runs, report IDs and conditional result | Passed |
| §9.9 Impossible domain | Empty feasibility; dependent obligations skipped without vacuous success | Passed |
| §9.10 Unknown is an ordinary enum, not approval | `unknown-approval` evaluates to unspecified | Passed |
| §9.11 Arbitrary precision | Typed evaluated integer exactly `18446744073709551616` | Passed |
| §9.12 Invalid references/units/date/cycle/node | Invalid fixtures return exact diagnostics and exit 2 | Passed |
| §9.13 SAT witness replay | Typed `ExpiryWithinContract` witness replayed through CLI evaluator | Passed |
| §9.14 Limits/restart | Capacity/no-eviction and kill/wait/restart invalidation | Passed |
| §7 framed worker protocol | Real worker hello/compile/analyze/evaluate/release and transport error cases | Passed |
| CLI malformed input/file errors | Structured JSON errors and exit 2 | Passed |

## Evidence log

At integration revision `758d32d` on x86_64 Linux, the following build passed:

```text
nix develop --command cargo build --workspace --bin entail --locked
exit 0
```

Against `target/debug/entail`:

```text
nix develop --command python3 scripts/acceptance.py target/debug/entail
exit 0: PASS: 20 fixture/scenario/CLI acceptance groups

nix develop --command python3 scripts/worker_acceptance.py target/debug/entail
exit 0: PASS: real worker protocol/lifecycle acceptance

nix develop --command python3 -m unittest scripts.test_acceptance -v
exit 0: 7 helper tests passed
```

The full checks and component-level suite are recorded in [`DELIVERY.md`](DELIVERY.md). These results establish only that the tested formal models and process protocol behaved as specified in this acceptance suite; they do not certify any business policy as complete, lawful, fair, or correct.
