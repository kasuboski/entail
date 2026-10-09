# V0 delivery evidence

**Status: incomplete.** This document separates implementation/test evidence from specification expectations. A fixture's expected result is a contract, not observed engine evidence. A source/API review or successful build is not end-to-end acceptance.

## Requirements and evidence map

| Requirement | Evidence source | Integrated evidence / status |
| --- | --- | --- |
| Strict JSON model validation, typed expressions, exact integers/dates, derived DAG, hashes, and concrete evaluation | `author_model` unit tests; schema/invalid fixtures; CLI acceptance | Model compiler/evaluator and schema/resource hardening are integrated. Model tests cover fixture compilation, evaluation, exact values/hashes, target-scoped fields, schema parity, and bounded preflight. A separate targeted model Clippy/diagnostics repair remains pending. CLI acceptance is pending. |
| Feasibility, reachability, coverage, conflict, compliance, prerequisites, assumptions, and result provenance | `author_analysis` injected-backend tests and analysis API; CLI acceptance | Analysis planner, bounded obligation scheduling, cache/report hardening, witness replay, and injected-backend tests are integrated. |
| Exact Z3 lowering/decoding, timeout/resource behavior, and differential checks | `author_z3` backend tests, including exhaustive/differential cases | Native Z3 encoding/decoding and differential/boundary tests are integrated. These library tests are not CLI acceptance. |
| Witness replay before findings; budgets, cache keys/invalidation, and incomplete-result behavior | Analysis replay/cache/budget tests; worker behavior | Implemented and covered by solver-independent analysis tests; process-level worker enforcement remains pending. Do not infer these from a satisfiable solver result. |
| CLI validate/analyze/evaluate behavior and exit codes | `scripts/acceptance.py` against the integrated executable | Acceptance assets are integrated; no usable integrated CLI run recorded. |
| Framed persistent worker, lifecycle, capacity, restart, malformed input, and recovery | `scripts/worker_acceptance.py` against the integrated executable | Three isolated transport framing tests passed; complete worker operations/restart acceptance is pending. |
| Reproducible workload generation and measured performance/cache evidence | `scripts/benchmark.py`, `docs/BENCHMARKS.md`, archived report | Workload generator/harness is integrated. Its Python 3 runtime is supplied by the pinned Nix development shell. No end-to-end measurements or budget/cache claims recorded. |
| Reproducible tooling, pinned native solver, lint policy, and MSRV | `BUILD.md`; `mise run check` | Rust 1.85 `cargo check --workspace --all-targets --all-features --locked` passes. The 34-test workspace nextest suite passes. Formatting passes after integration formatting cleanup; strict workspace Clippy remains blocked by model and transport findings. |

### Current test evidence

At integration revision `43d7fce`, `nix develop --command cargo nextest run --workspace --all-features --no-tests=pass` ran 34 tests and passed: model validation/evaluation, analysis planning and replay/cache/budget behavior, transport framing, and Z3 backend/differential tests. CLI/worker lifecycle and black-box acceptance are not yet present.

After the lockfile update, the following exact MSRV command passed:

```sh
mise x rust@1.85.0 -- nix develop --command cargo check --workspace --all-targets --all-features --locked
```

The complete required gate is:

```sh
mise run check
```

Do not mark the release gate passed until this completes successfully on the integrated revisions and the acceptance/worker scripts run against the actual executable. Acceptance and worker helper self-tests passed under `nix develop` (these test only Python framing/process helpers, not the application). The benchmark generator ran and structurally validated its fixtures; no measurements were collected because no integrated CLI/worker is available. No end-to-end acceptance or benchmark result is currently claimed.

## Known blockers and open evidence

- Integrated model review repairs address target-specific outcome-field validation, year-zero date rejection, schema parity, and bounded preflight concerns. The model still fails the workspace's unchanged pedantic Clippy policy (documentation, arithmetic, `Self`, function-size, and related findings); the targeted model quality/diagnostics agent owns those repairs. No integration-side lint weakening is acceptable.
- The imported framed transport checkpoint's format issues were cleaned up in integration, but it still fails strict Clippy for public `# Errors` docs, interrupted-read control flow, safe slicing, and checked arithmetic/conversion. The worker owner has analogous fixes in progress; keep root Clippy policies intact.
- The analysis and Z3 library implementations are integrated and their tests pass. CLI/worker dispatch and lifecycle, external application acceptance, and measured budget/cache/performance evidence remain pending.
- Before declaring feature completion, reconcile the full specification's process/restart, differential, budgets/cache/provenance, witness replay, and complete-policy behavioral tests. Document any genuinely deferred requirement explicitly rather than implying completion.

## Business correctness limitation

A passing validator, SAT/UNSAT result, fixture, or benchmark only says something about the encoded model and tested execution. The engine does not establish that source material is true or correctly interpreted, that requirements are complete, or that a proposed policy is lawful, fair, safe, or appropriate. Proposed rules participate in analysis; assumptions are conditional; open items and model gaps remain material. V0 models a decision at one snapshot and does not prove lifecycle behavior or execute business effects.
