# Author engine v0 end-user acceptance

**Status: pending real application runs.** These scripts are black-box checks against the integration executable, not an engine mock. The fixtures and expected outcomes live in `author-engine-v0/fixtures`; they are test inputs, never evidence by themselves.

## Run

```sh
python3 -m unittest scripts.test_acceptance -v
python3 scripts/acceptance.py /path/to/entail
python3 scripts/worker_acceptance.py /path/to/entail
```

Every subprocess has a finite timeout. The Python helper tests and either script's `--self-test` exercise only local assertion/framing/process utilities; none counts as application acceptance. Record the executable path/revision, build command, platform, exact commands, exit codes, pass counts, and failures in the evidence log below. Do not mark criteria passed on source inspection or helper-only output.

`acceptance.py` checks all valid and invalid fixtures, stable diagnostic codes and exit 2, analysis exit 0, exact obligation kind/status/category, mandatory domain feasibility, one reachability result per rule, and each expected rule/constraint compliance result by exact IDs (never provenance substrings). It also verifies report identity/hashes/assumptions/open items, proposed-policy context and `complete=true` without claiming business verification. The conditional fixture is run both with an explicit empty assumption list and with `SponsorsAlreadyApproved`. Concrete scenarios check unknown approval remains unspecified, unsafe expiry reports the exact violated constraint, unsafe SAT witness replays through `evaluate`, and the arbitrary-precision result is a typed exact integer. CLI malformed-input and missing-file cases must return structured errors with exit 2. The payload fixture must show a conflict between unequal payloads of the same `Grant` constructor.

The worker acceptance script checks the documented `author-worker/0.1` framing and stable protocol error codes, fragmented frames, operation envelopes, valid/invalid compile, analysis and evaluation, exact release behavior, advertised handle capacity without eviction, malformed JSON recovery, invalid-version and invalid-length handling, kill/wait, restart and handle invalidation. It rejects a capacity bound above its explicitly safe 128-model probe rather than weakening the criterion. Every worker frame read and process wait is bounded.

## Criteria map

| Spec criterion | Evidence in scripts | Status |
| --- | --- | --- |
| §9.1 Partial model: `behavior_unspecified` | `partial`, exact coverage category | Pending real run |
| §9.2 Unsafe Grant violates expiry independently of guard | `unsafe-grant`, exact rule/constraint result and evaluator replay | Pending real run |
| §9.3 Guard repair removes violation but leaves uncovered inputs | `repaired-partial`, each per-rule constraint plus coverage | Pending real run |
| §9.4 Complete proposed policy | `complete-proposed`: exact coverage/conflict, report complete, proposed IDs/open items; rejects `business_verified=true` | Pending real run |
| §9.5 Simultaneous Grant/Deny conflicts | `conflict-tag`: exact conflict category | Pending real run |
| §9.6 Identical outcomes deduplicate | `duplicate-output`: compatible | Pending real run |
| §9.7 Same constructor with distinct payloads conflicts | `conflict-payload`: expected conflict constructor and unequal Grant payload evidence | Pending real run |
| §9.8 Assumptions are opt-in and visible | Baseline with explicit `[]`, selected assumption run, report IDs and conditional result | Pending real run |
| §9.9 Impossible domain | `empty-domain`: feasibility empty; every dependent result skipped, no vacuous success | Pending real run |
| §9.10 Unknown is an ordinary enum, not approval | `unknown-approval` concrete evaluation is unspecified | Pending real run |
| §9.11 Arbitrary precision | Typed evaluated integer exactly `18446744073709551616`; no floating-point/text-only match | Pending real run |
| §9.12 Invalid references/units/date/cycle/node | All invalid fixtures, exact diagnostic and exit 2 | Pending real run |
| §9.13 SAT witness replay | Typed `ExpiryWithinContract` violation witness is evaluated through CLI; exact violation must recur | Pending real run |
| §9.14 Limits/restart | Worker advertised capacity, no live-handle eviction, kill/wait/restart invalidates old handle | Pending real run |
| §7 framed worker protocol | Worker script exercises hello/compile/analyze/evaluate/release and transport errors | Pending real run |
| CLI malformed input and file errors | Acceptance script requires structured JSON error and exit 2 | Pending real run |

## Evidence log

No real executable run has yet been recorded. The integration target was checked at task start and contained Cargo build artifacts but no `target/debug/entail` executable. This is not a pass or a final blocker: build/run the integration executable once the application is committed. If the app implementation fails a check, retain the assertion and report the exact command, observed output and a minimal reproducer to the integration owner; do not weaken the criteria to fit the implementation.
