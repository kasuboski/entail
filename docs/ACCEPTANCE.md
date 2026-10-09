# Author engine v0 end-user acceptance

**Status: pending.** No application implementation or `entail` binary exists in this branch. The assets below are black-box checks against the real CLI/worker; helper self-tests do not constitute application evidence. Do not report any criterion as passed until a built binary has been run and its output/exit status recorded.

## Running the checks

```sh
python3 scripts/acceptance.py /path/to/entail
python3 scripts/worker_acceptance.py /path/to/entail
```

Both scripts invoke the supplied executable directly. They do not substitute a mock, interpret the model themselves, or claim that fixtures prove the application works. They find results dynamically in JSON envelopes and assert stable check status/finding category, declaration IDs, diagnostic codes, selected assumptions, and process error fields rather than comparing whole-result snapshots. `--self-test` only tests Python framing/traversal/process helpers and is explicitly not application acceptance.

Fixture definitions and expected semantic outcomes are in `author-engine-v0/fixtures/expectations.json`; scenario inputs are in `author-engine-v0/scenarios/`. The fixture expectations are contracts, not recorded engine results. Worker request shapes and framing are defined in the v0 spec and `author-engine-v0/protocol/`.

## Spec acceptance-criteria map

| Spec criterion | End-user check / evidence | Status |
| --- | --- | --- |
| §9.1 Partial constraints compile; coverage is `behavior_unspecified` | `acceptance.py`: `partial` validate/analyze against fixture expectation | Pending binary run |
| §9.2 Unsafe grant violates expiry independently of guard | `acceptance.py`: `unsafe-grant`; compliance for `ExpiryWithinContract` must be a `violation` | Pending binary run |
| §9.3 Guarded repair removes compliance violation but leaves uncovered inputs | `acceptance.py`: `repaired-partial`; compliance and coverage categories | Pending binary run |
| §9.4 Complete proposed policy is covered and compatible, not confirmed business policy | `acceptance.py`: `complete-proposed`; structured check results and open items remain visible in output | Pending binary run |
| §9.5 Simultaneous Grant/Deny conflict | `acceptance.py`: `conflict-tag`; `outcome_conflict=conflict` | Pending binary run |
| §9.6 Identical overlapping outcomes deduplicate | `acceptance.py`: `duplicate-output`; `outcome_conflict=compatible` | Pending binary run |
| §9.7 Same constructor with different payloads conflicts | `acceptance.py`: `conflict-payload`; conflict finding must identify `Grant` | Pending binary run |
| §9.8 Assumptions are opt-in and reported | `acceptance.py`: `conditional-assumption` analyzed both without assumption and with `SponsorsAlreadyApproved`; conditional scope and selected ID checked | Pending binary run |
| §9.9 Empty domain skips dependent checks, never claims success | `acceptance.py`: `empty-domain`; `empty_domain` plus skipped dependent checks | Pending binary run |
| §9.10 Unknown approval is an ordinary enum value, not approval | `acceptance.py`: `unknown-approval` concrete evaluation expects `unspecified` | Pending binary run |
| §9.11 Arbitrary precision survives JSON/evaluator/solver boundary | `acceptance.py`: `big-integer` analysis plus concrete scenario expects exact `18446744073709551616` | Pending binary run |
| §9.12 Reference, units, dates, cycles, unsupported node fail before solving | `acceptance.py`: all five invalid fixtures; exit 2 and expected stable diagnostic code | Pending binary run |
| §9.13 SAT witnesses replay in evaluator; mismatch is engine error | Not deterministically forced by public fixture without a test fault-injection interface. The unsafe compliance witness is checked as a finding; witness replay's internal test remains implementation evidence required by plan §3/Verification. | Pending implementation evidence |
| §9.14 Timeouts, obligation limits, restart don't produce false success | Worker script verifies process restart invalidates handles and recompile works. CLI/worker budget truncation and timeout behavior require a deterministic configured backend or fixture hook and remain an implementation-level pending test. | Pending implementation evidence |
| §7 worker frame: 4-byte big-endian, UTF-8 JSON, partial reads/writes | `worker_acceptance.py`: real worker; fragmented one-byte hello request and framed responses | Pending binary run |
| Worker hello, compile, analyze, evaluate, release lifecycle | `worker_acceptance.py`: same persistent process runs all five operations and checks response envelopes | Pending binary run |
| Invalid compile is normal compile result, not transport error | `worker_acceptance.py`: unknown-reference compile returns diagnostics and no handle | Pending binary run |
| Malformed JSON and invalid protocol version are protocol errors | `worker_acceptance.py`: malformed request / unsupported version preserve request ID where recoverable and return error envelope | Pending binary run |
| Invalid frame length closes worker nonzero | `worker_acceptance.py`: isolated worker gets length above reported/default frame bound; must exit nonzero | Pending binary run |
| Capacity does not silently evict live handles | `worker_acceptance.py`: probes live-model capacity and verifies existing handle remains usable if capacity is observed; bounds probe to 128 models | Pending binary run; high capacity may exceed probe |
| OS termination is reaped; restart rejects old handle and accepts recompilation | `worker_acceptance.py`: terminate/wait, restart, `unknown_handle`, then compile/release | Pending binary run |
| CLI validate/analyze/evaluate output JSON and exit codes | `acceptance.py`: validates invalid fixtures with exit 2 and analyzes/evaluates valid fixtures with exit 0 | Pending binary run |

## Evidence log

No `entail` implementation is present yet. Record binary path, source revision, build command, platform, command lines, exit codes and a concise result summary here after actual runs. Keep captured output externally or attach it to the implementation/CI job; do not convert these pending rows into passes based only on helper self-tests or source inspection.
