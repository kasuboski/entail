# V0 delivery evidence

**Status: incomplete.** This document separates implementation/test evidence from specification expectations. A fixture's expected result is a contract, not observed engine evidence. A source/API review or successful build is not end-to-end acceptance.

## Requirements and evidence map

| Requirement | Evidence source | Integrated evidence / status |
| --- | --- | --- |
| Strict JSON model validation, typed expressions, exact integers/dates, derived DAG, hashes, and concrete evaluation | `author_model` unit tests; schema/invalid fixtures; CLI acceptance | Initial model compiler/evaluator is integrated. Four crate tests passed in the initial integration run (fixture compilation, unsafe witness replay, big integer, and metadata-vs-semantic hash). Independent review found unresolved validation and diagnostic defects; see blockers. CLI acceptance is pending. |
| Feasibility, reachability, coverage, conflict, compliance, prerequisites, assumptions, and result provenance | `author_analysis` injected-backend tests and analysis API; CLI acceptance | Pending analysis implementation and integrated tests. |
| Exact Z3 lowering/decoding, timeout/resource behavior, and differential checks | `author_z3` backend tests, including exhaustive/differential cases | Pending Z3 implementation. Foundation SAT/UNSAT smoke test is not evidence of model encoding. |
| Witness replay before findings; budgets, cache keys/invalidation, and incomplete-result behavior | Analysis replay/cache/budget tests; worker behavior | Pending. Do not infer these from a satisfiable solver result. |
| CLI validate/analyze/evaluate behavior and exit codes | `scripts/acceptance.py` against the integrated executable | Acceptance assets are integrated; no usable integrated CLI run recorded. |
| Framed persistent worker, lifecycle, capacity, restart, malformed input, and recovery | `scripts/worker_acceptance.py` against the integrated executable | Three isolated transport framing tests passed; complete worker operations/restart acceptance is pending. |
| Reproducible workload generation and measured performance/cache evidence | `scripts/benchmark.py`, `docs/BENCHMARKS.md`, archived report | Workload generator/harness is integrated. No end-to-end measurements or budget/cache claims recorded. |
| Reproducible tooling, pinned native solver, lint policy, and MSRV | `BUILD.md`; `mise run check` | Rust 1.85 `cargo check --workspace --all-targets --all-features --locked` passed after integrating model/worker dependencies. The full check currently fails formatting and strict Clippy for imported component code; see blockers. |

### Current test evidence

At integration revision `052b66d` (model compiler plus framed transport), `cargo nextest run --workspace --all-features --no-tests=pass` ran 8 tests and passed: four model tests, three transport tests, and one native Z3 SAT/UNSAT smoke test. This is component-level evidence only; analysis, production Z3 encoding, CLI, worker lifecycle, and black-box acceptance were not yet present.

After the lockfile update, the following exact MSRV command passed:

```sh
mise x rust@1.85.0 -- nix develop --command cargo check --workspace --all-targets --all-features --locked
```

The complete required gate is:

```sh
mise run check
```

Do not mark the release gate passed until this completes successfully on the integrated revisions and the acceptance/worker scripts run against the actual executable. No end-to-end acceptance or benchmark result is currently claimed.

## Known blockers and open evidence

- Initial `author_model` implementation does not pass the workspace's unchanged pedantic Clippy policy (including documentation, arithmetic, `Self`, and function-size findings). Independent review also identified target-specific outcome-field validation, year-zero date rejection, schema parity, and open-item source validation concerns. Repairs are owned by the model/hardening agent; no integration-side lint weakening is acceptable.
- The imported framed transport checkpoint is not rustfmt/Clippy clean under workspace policy. Its interrupted-read loop and length handling also need strict-lint fixes. This belongs with the worker owner; keep root Clippy policies intact.
- `author_analysis`, complete `author_z3` encoding, CLI/worker dispatch and lifecycle, external acceptance runs, and measured budget/cache/performance evidence remain pending.
- Before declaring feature completion, reconcile the full specification's process/restart, differential, budgets/cache/provenance, witness replay, and complete-policy behavioral tests. Document any genuinely deferred requirement explicitly rather than implying completion.

## Business correctness limitation

A passing validator, SAT/UNSAT result, fixture, or benchmark only says something about the encoded model and tested execution. The engine does not establish that source material is true or correctly interpreted, that requirements are complete, or that a proposed policy is lawful, fair, safe, or appropriate. Proposed rules participate in analysis; assumptions are conditional; open items and model gaps remain material. V0 models a decision at one snapshot and does not prove lifecycle behavior or execute business effects.
