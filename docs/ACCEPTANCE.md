# Author engine v0 end-user acceptance

**Status: final CLI and worker acceptance passed against integrated code source `3da81cee`.** These scripts are black-box checks against a built executable, not an engine mock. Fixtures and expectations in `author-engine-v0/fixtures` are test inputs, never evidence by themselves.

## Run

Use the pinned Nix environment for the dynamically linked Z3 runtime:

```sh
mise exec -- nix develop --command python3 -m unittest discover -s scripts -p 'test_*.py' -v
mise exec -- nix develop --command python3 scripts/acceptance.py /path/to/entail
mise exec -- nix develop --command python3 scripts/worker_acceptance.py /path/to/entail
```

Every subprocess has a finite timeout. The Python helper tests and either script's `--self-test` exercise only local assertion/framing/process utilities; none counts as application acceptance. Record the executable path/revision, build command, platform, exact commands, exit codes, pass counts, and failures in the evidence log below. Do not mark criteria passed on source inspection or helper-only output.

`acceptance.py` checks all valid and invalid fixtures, stable diagnostic codes and exit 2, analysis exit 0, exact obligation kind/status/category, mandatory domain feasibility, one reachability result per rule, and each expected rule/constraint compliance result by exact IDs (never provenance substrings). It also verifies report identity/hashes/assumptions/open items, proposed-policy context and `complete=true` without claiming business verification. The conditional fixture is run both with an explicit empty assumption list and with `SponsorsAlreadyApproved`. Concrete scenarios check unknown approval remains unspecified, unsafe expiry reports the exact violated constraint, unsafe SAT witness replays through `evaluate`, and the arbitrary-precision result is a typed exact integer. CLI malformed-input and missing-file cases must return structured errors with exit 2. The payload fixture must show a conflict between unequal payloads of the same `Grant` constructor.

The worker acceptance script checks the documented `author-worker/0.1` framing and stable protocol error codes, fragmented frames, operation envelopes, valid/invalid compile, analysis and evaluation, exact release behavior, advertised handle capacity without eviction, malformed JSON recovery, invalid-version and invalid-length handling, kill/wait, restart and handle invalidation. It rejects a capacity bound above its explicitly safe 128-model probe rather than weakening the criterion. Every worker frame read and process wait is bounded.

## Criteria map

| Spec criterion | Evidence in scripts | Status |
| --- | --- | --- |
| §9.1 Partial model: `behavior_unspecified` | `partial`, exact coverage category | Passed, integrated run |
| §9.2 Unsafe Grant violates expiry independently of guard | `unsafe-grant`, exact rule/constraint result and evaluator replay | Passed, integrated run |
| §9.3 Guard repair removes violation but leaves uncovered inputs | `repaired-partial`, each per-rule constraint plus coverage | Passed, integrated run |
| §9.4 Complete proposed policy | `complete-proposed`: exact coverage/conflict, report complete, proposed IDs/open items; rejects `business_verified=true` | Passed, integrated run |
| §9.5 Simultaneous Grant/Deny conflicts | `conflict-tag`: exact conflict category and evaluator replay | Passed, integrated run |
| §9.6 Identical outcomes deduplicate | `duplicate-output`: compatible | Passed, integrated run |
| §9.7 Same constructor with distinct payloads conflicts | `conflict-payload`: paired Grant rules and distinct replayed payloads | Passed, integrated run |
| §9.8 Assumptions are opt-in and visible | Baseline with explicit `[]`, selected assumption run, report IDs and conditional result | Passed, integrated run |
| §9.9 Impossible domain | `empty-domain`: feasibility empty; every dependent result skipped, no vacuous success | Passed, integrated run |
| §9.10 Unknown is an ordinary enum, not approval | `unknown-approval` concrete evaluation is unspecified | Passed, integrated run |
| §9.11 Arbitrary precision | Typed evaluated integer exactly `18446744073709551616`; no floating-point/text-only match | Passed, integrated run |
| §9.12 Invalid references/units/date/cycle/node | All invalid fixtures, exact diagnostic and exit 2 | Passed, integrated run |
| §9.13 SAT witness replay | Typed `ExpiryWithinContract` violation witness is evaluated through CLI; exact violation must recur | Passed, integrated run |
| §9.14 Limits/restart | Worker advertised capacity, no live-handle eviction, kill/wait/restart invalidates old handle | Passed, integrated run |
| §7 framed worker protocol | Worker script exercises hello/compile/analyze/evaluate/release and transport errors | Passed, integrated run |
| CLI malformed input and file errors | Acceptance script requires structured JSON error and exit 2 | Passed, integrated run |

## Evidence log

Final integrated acceptance was run from `/home/josh/projects/entail2-integration` against code source revision `3da81cee5fe87544eb3a5b870706881abb07b089`. Built the executable in the pinned Nix environment:

```sh
cd /home/josh/projects/entail2-integration
mise exec -- nix develop --command cargo build --workspace --bin entail --locked
```

Build exited 0. Run both black-box suites inside the pinned Nix environment because the executable dynamically links the pinned Z3 library:

```sh
cd /home/josh/projects/entail2-integration
mise exec -- nix develop --command python3 scripts/acceptance.py target/release/entail
# exit 0: PASS: 20 fixture/scenario/CLI acceptance groups

mise exec -- nix develop --command python3 scripts/worker_acceptance.py target/release/entail
# exit 0: PASS: real worker protocol/lifecycle acceptance
```

The final Python helper suite (Python 3.14.7 via Nix) passed 12 tests through `python3 -m unittest discover -s scripts -p 'test_*.py' -v`; both `--self-test` commands passed. These local helper results are not counted as CLI/worker acceptance.

Coverage includes normal findings exiting 0, invalid models/files/JSON exiting 2, the fixtures/scenarios, and worker protocol/lifecycle acceptance. A deterministic backend-fault injection seam is not exposed by the public CLI, so CLI exit 3 was **not induced** by black-box acceptance; do not count that branch as black-box tested. Backend error mapping is tested through the analysis seam. Run the executable inside `nix develop` for its pinned `libz3.so.5.1` runtime.
