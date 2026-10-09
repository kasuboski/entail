# V0 delivery evidence

**Status: integrated and application acceptance passed; performance/cache evidence remains partial.** This document separates observed implementation/test results from specification expectations. Fixture expectations are contracts, not evidence by themselves.

## Requirements and evidence map

| Requirement | Evidence source | Integrated evidence / status |
| --- | --- | --- |
| Strict JSON model validation, typed expressions, exact integers/dates, derived DAG, hashes, and concrete evaluation | `author_model` tests; schema/invalid fixtures; CLI acceptance | Model compiler/evaluator, schema/resource hardening, diagnostic context and expanded exactness tests integrated. Workspace tests and external fixture acceptance pass. |
| Feasibility, reachability, coverage, conflict, compliance, prerequisites, assumptions, result provenance | `author_analysis` tests/API; CLI acceptance | Analysis planner, bounded obligation scheduling, cache/report hardening, witness replay and injected-backend tests integrated; black-box fixture acceptance passes. |
| Exact Z3 lowering/decoding, timeout/resource behavior, and differential checks | `author_z3` backend tests | Native Z3 encoding/decoding, boundary tests and finite-domain differential checks pass. Native solver is pinned at Z3 5.1.0 through Nix. |
| Witness replay before findings; budgets, cache keys/invalidation, incomplete-result behavior | Analysis replay/cache/budget tests; worker tests | Covered by solver-independent tests; worker budget clamping and process lifecycle are exercised. No cache-hit or budget-performance measurements are claimed. |
| CLI validate/analyze/evaluate behavior and exit codes | `scripts/acceptance.py` against the built executable | **Passed:** 20 fixture/scenario/CLI acceptance groups, including witness replay and structured errors. |
| Framed persistent worker, lifecycle, capacity, restart, malformed input, and recovery | `scripts/worker_acceptance.py`; Rust worker process tests | **Passed:** external worker protocol/lifecycle acceptance plus in-process integration tests for capacity, kill/wait/restart and stale handles. |
| Reproducible workload generation and measured performance/cache evidence | `scripts/benchmark.py`, `docs/BENCHMARKS.md` | Smoke benchmark completed: five workload families, three samples each, no command failures. It measures debug CLI validation wall time including process startup. Cache rate, query/formula counts, memory, and solver metadata are unavailable; no cache or performance-budget conclusion is made. |
| Reproducible tooling, pinned native solver, lint policy, and MSRV | `BUILD.md`; `mise run check` | **Passed** at `758d32d`: format, strict workspace Clippy, Rust 1.85 locked check, and all-target/all-feature nextest. Root Clippy policy and pins were preserved. |

## Exact verification evidence

At integration revision `758d32d`, the executable was built with:

```sh
nix develop --command cargo build --workspace --bin entail --locked
```

The complete workspace gate passed:

```text
mise run check
format: passed
lint: passed (Clippy all targets/features, warnings denied)
msrv: passed (Rust 1.85, cargo check all targets/features, locked)
test: passed (46 tests, 0 failed, 0 skipped)
```

Black-box CLI/worker and Python helper tests passed against `target/debug/entail`:

```text
nix develop --command python3 scripts/acceptance.py target/debug/entail
PASS: 20 fixture/scenario/CLI acceptance groups

nix develop --command python3 scripts/worker_acceptance.py target/debug/entail
PASS: real worker protocol/lifecycle acceptance

nix develop --command python3 -m unittest scripts.test_acceptance -v
7 helper tests passed
```

Benchmark evidence (debug executable; x86_64 Linux, Rust 1.98.0, Python 3.14.7; three samples per family):

```text
nix develop --command python3 scripts/benchmark.py --binary target/debug/entail --runs 3
```

| Workload | p50 wall ms | p95 wall ms | Failures |
| --- | ---: | ---: | ---: |
| `independent_thresholds_10` | 2.909 | 3.528 | 0 |
| `overlapping_guards` | 5.961 | 6.350 | 0 |
| `same_constructor_payload` | 6.816 | 6.885 | 0 |
| `coupled_boolean` | 4.006 | 4.062 | 0 |
| `shared_derived_dag` | 9.514 | 10.082 | 0 |

These are small smoke measurements, not release-performance claims. The harness could not report profile, Z3 version, peak memory, formula size, cache rate, inconclusive rate, or query count. The binary's analysis report does expose solver version; the harness does not collect it. Do not infer cache behavior or analysis latency from these CLI-validation timings.

## Remaining evidence gaps

- Run/record the full benchmark suite (including larger workloads) with a release build and analysis operations, not only validation. Keep solver version, command/profile, and platform with the report.
- The benchmark harness does not currently expose or measure cache hit rate/invalidation, solver query/formula counts, peak memory, or budget exhaustion rates. Until those are instrumented and measured, there is no quantitative performance/cache evidence or validated interactive budget claim.
- The engine validates and analyzes authored models; it does not establish source truth, completeness of requirements, or that a proposed policy is lawful, fair, safe, or appropriate. Assumptions remain conditional, proposed rules participate, open items remain visible, and V0 does not prove lifecycle behavior or execute effects.

## Business correctness limitation

A passing validator, SAT/UNSAT result, fixture, acceptance suite, or benchmark only says something about the encoded model and tested execution. It cannot establish that the model faithfully represents reality or that the represented decision is correct. Use the results as evidence for human review, not as business authorization.
