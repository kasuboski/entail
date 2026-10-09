# V0 delivery evidence

**Status: final production gates and application acceptance passed at source `0698b2a`.** This document distinguishes tests of engine behavior from evidence about solver performance. Passing results establish behavior for the encoded model and tested execution; they do not certify a business policy.

## Requirements and evidence map

| Requirement | Evidence | Final integrated result |
| --- | --- | --- |
| Strict model validation, typed expressions, exact integers/dates, derived DAG, hashes, evaluation and diagnostics | `author_model` unit tests; invalid fixtures; immutable API doc test | Covered by model tests including exact nested diagnostic pointers, optional-metadata serialization round trips, and `CompiledModel` immutability (`compile_fail`). Regression test checks every invalid fixture's authoritative diagnostic code; the bundle matches baseline `95d0c26` byte-for-byte. |
| Feasibility, reachability, coverage, conflicts, compliance, assumptions, provenance and replay | `author_analysis` tests; black-box CLI acceptance | Covered by replay/cache/budget/error tests; coverage provenance includes all rules and refreshes on cached/skipped results. |
| Exact Z3 lowering/decoding, resource limits, detached native statistics and differential behavior | `author_z3` backend tests | Covered by native solver, boundary and finite semantic-oracle tests across typed values/expressions. Z3 5.1.0 is pinned. |
| CLI findings/errors and documented exit behavior | `scripts/acceptance.py` against the release executable | **Passed:** 20 fixture/scenario/CLI groups. Findings return 0; invalid model/file/JSON cases return 2 with structured errors. The public CLI has no backend-fault injection seam, so a black-box exit-3 case was not induced; backend error mapping is exercised at the analysis seam. |
| Persistent worker framing, budgets, capacity, malformed-input recovery, release, restart and stale handles | `scripts/worker_acceptance.py`; Rust worker process tests | **Passed:** external protocol/lifecycle acceptance and in-process process tests. Malformed-JSON request-ID recovery uses only a Serde visitor/`IgnoredAny`, with nested and malformed-prefix cases tested. |
| Reproducible workload generation and measured release analysis/cache/native-statistics/RSS evidence | `scripts/benchmark.py`; [`BENCHMARKS.md`](BENCHMARKS.md); archived report | **Passed:** eight workloads, five repetitions each, no worker-family errors; explicit cache/edit/assumption probes; detached solver statistics, engine compile time and sampled Linux RSS recorded. Bounded cases that exhaust the 24-obligation smoke cap are reported as incomplete, not as successful. This is one-host smoke evidence, not an SLO. |
| Formatting, strict lint, MSRV, Rust tests and public API compile-fail check | `mise run check`; `cargo test --workspace --doc --locked` | **Passed:** formatting, strict all-target/all-feature Clippy, Rust 1.85 locked check, **67/67** nextest tests; doc tests include two passing type-correct compile-fail tests, including `CompiledModel` immutability. |
| Python acceptance and benchmark helpers | Python unittest discovery; helper self-tests; generator check | **Passed:** 12 Python helper tests; both acceptance/worker helper self-tests; benchmark fixture generation/structural validation. Helper-only tests are not counted as application acceptance. |

## Exact verification commands and results

Commands were run from `/home/josh/projects/entail2` at source `0698b2a`, with the Nix development shell providing the pinned native Z3 runtime:

```sh
mise run check
# passed at source 0698b2a: fmt, strict Clippy, Rust 1.85 locked check, nextest 67/67

mise exec -- nix develop --command cargo test --workspace --doc --locked
# passed: 2 type-correct compile_fail doc tests; all crate doc-test targets passed

mise exec -- nix develop --command cargo build --workspace --bin entail --locked
# passed

mise exec -- nix develop --command python3 scripts/acceptance.py target/release/entail
# PASS: 20 fixture/scenario/CLI acceptance groups

mise exec -- nix develop --command python3 scripts/worker_acceptance.py target/release/entail
# PASS: real worker protocol/lifecycle acceptance

mise exec -- nix develop --command python3 -m unittest discover -s scripts -p 'test_*.py' -v
# 12 passed

mise exec -- nix develop --command python3 scripts/acceptance.py --self-test
# passed (helpers only)
mise exec -- nix develop --command python3 scripts/worker_acceptance.py --self-test
# passed (helpers only)
mise exec -- nix develop --command python3 scripts/benchmark.py
# generated and structurally validated fixtures; no measurement claimed by this command

mise exec -- nix develop --command cargo build --release --locked --bin entail
# passed: optimized release executable built
```

The release worker measurement command was:

```sh
mise exec -- nix develop --command python3 scripts/benchmark.py \
  --binary target/release/entail --runs 5 --profile release \
  --rust-version 'rustc 1.98.0 (88d9e12ae 2026-08-18)'
```

Full per-run reports, detached native statistic maps, cache probes, compile timings, environment and RSS sampling details are archived in [`benchmarks/smoke-release-final-2026-10-09.json`](../benchmarks/smoke-release-final-2026-10-09.json). That earlier measured executable was built from source `3da81cee5fe87544eb3a5b870706881abb07b089`; it predates later diagnostic and malformed-JSON recovery changes. Treat it as historical measurement evidence, not a full measurement of source `0698b2a`. Native-statistics/RSS/cache data are retained, and no performance rebenchmark is claimed for this final source. See [`BENCHMARKS.md`](BENCHMARKS.md) for interpretation.

## Remaining limits

- V0 operates on explicitly authored JSON model snapshots; there is no production Gleam integration, DSL, effects execution, or Lean proof layer.
- The engine does not establish source truth, completeness of requirements, or whether a policy is lawful, fair, safe, or appropriate. Proposed rules participate; assumptions remain opt-in and conditional; open items remain visible.
- Exit code 3 for a backend internal error is mapped by the CLI but is not induced through black-box acceptance because the public CLI has no deterministic backend-fault injection seam.
- Exact formula AST node count is not exposed. Native Z3 statistics are preserved, with `assertion_count` labeled only as a proxy; sampled Linux `VmHWM` is process high-water RSS, not a per-query allocation measurement.
- Benchmark results cover one Linux host, five repetitions and the worker path under explicit smoke budgets. They are not latency or memory guarantees, and CLI analysis was not part of this final follow-up measurement.

## Business correctness limitation

A passing validator, SAT/UNSAT result, fixture, acceptance suite, or benchmark only says something about the encoded model and tested execution. It cannot establish that the model faithfully represents reality or that the represented decision is correct. Use results as evidence for human review, not as business authorization.
