# Author decision engine v0 implementation plan

Implement the Rust engine described by `author-engine-v0-spec.md`. The first useful result is a CLI that accepts a partial business-decision model, identifies an unsafe rule with a replayable counterexample, and distinguishes a mathematically complete proposed policy from an approved business method. Then expose the same APIs through a persistent worker for Gleam.

This plan covers the engine and integration contract. It does not implement Author's discovery loop, a business executor, lifecycle model checking, a NIF or Lean proofs.

## Decisions to carry into implementation

- Versioned JSON AST is the initial formal language; schema and semantic validation both apply.
- JSON is a frontend encoding of the typed model. A future text frontend maps one-to-one to the same constructs and preserves metadata; it introduces no separate semantics.
- Keep the spec's readable block style stable: `constraint` with `when outcome is`, `require`, and `source`; `rule` with `when`, `conclude`, and `status`. Contractor outputs are `Grant(expiry)` and `Deny(reason)`, including in JSON fixtures.
- Exact typed snapshot semantics; every applicable rule contributes an outcome.
- Requirements remain independent from rule guards and analysis assumptions.
- Partial models and open items are first-class; lack of behavior is not a denial.
- Z3 is the first solver; a small evaluator independently replays returned witnesses.
- Rust owns compilation and analysis; Gleam owns orchestration and persistent state.
- One supervised OS worker per active analysis, with a framed JSON protocol.
- Hard cancellation terminates and reaps the worker; no asynchronous cancel protocol in v0.

## Milestone 1: establish the model and native build

Create the Cargo workspace and the four components from the specification. Implement the serializable model types, tagged expression variants and declaration metadata. Deserialize strictly; reject unknown fields, tags and language versions. Add a native Z3 smoke build and a single satisfiable/unsatisfiable query to establish packaging early.

Record exact crate, toolchain and native solver versions in Cargo lockfiles and build documentation. Pick a reproducible local dynamic-linking or vendored route based on that smoke build. Do not spend the milestone promising static binaries for every platform.

Deliverables: typed raw model, parser diagnostics, schema-alignment tests, native dependency instructions and reference fixture parsing.

Exit criteria: valid fixtures deserialize; schema-invalid fixtures are rejected; native solving builds on the development platform; unsupported model versions fail predictably. The README names tested platforms and versions.

## Milestone 2: semantic compiler and concrete evaluator

Resolve declarations and references. Validate enum membership, exact dates, integer encodings, units, expression types, outcome field completeness and expression scopes. Construct and topologically sort the derived-expression graph. Enforce limits without constructing enormous intermediate trees. Produce a typed model with source locations and hashes.

Implement concrete scenario evaluation from the typed model. Validate scenario fields, calculate derived expressions, apply domain and explicitly selected assumptions, collect all matched rules, deduplicate exactly equal outcomes and check independent constraints. Keep excluded, unspecified, unique and conflicting behavior distinct.

Deliverables: compiler, evaluator, stable diagnostics, source mapping, exact value serialization and model hashes.

Exit criteria: invalid references, types, dates, units and cycles produce specific diagnostics; `Unknown` behaves as a declared enum value; duplicate outcomes are deduplicated; different payloads are distinct; a date after contract end violates the requirement. Values larger than 64 bits round-trip exactly. Proposed declarations and open items survive compilation.

## Milestone 3: implement the check catalog

Define solver-independent obligations for base-domain feasibility, rule reachability, coverage, outcome conflicts and rule compliance. Implement the prerequisite and vacuity behavior before optimizing. Compile the typed expressions to Z3 without narrowing integers or changing rule semantics.

Add exact witness decoding and evaluator replay for every SAT finding. Report UNKNOWN and timeouts explicitly. Attach declaration IDs and source references. Provide standalone `validate`, `analyze` and `evaluate` commands with JSON output.

Deliverables: working end-to-end CLI, Z3 backend, check reports and the contractor reference walkthrough.

Exit criteria: unsafe fixture yields an expiry counterexample; repaired partial fixture has no expiry violation but remains uncovered; complete proposed fixture is covered and conflict-free; conflict fixtures distinguish constructor and payload conflicts; impossible-domain fixture skips dependent checks. Selecting an assumption changes scope only when requested. Every SAT finding passes replay.

This is the first usable engine milestone. Integration can use the CLI temporarily, but the final v0 integration target is the worker.

## Milestone 4: budgets and analysis measurement

Add per-obligation and whole-analysis budgets, bounded lazy obligation generation and partial-result reporting. Record compilation and solve times separately. Add semantic-result caching with exact context keys; rebuild presentation metadata for the current content. Never treat cached inconclusive results as proof.

Measure before adding elaborate pruning. A semantic dependency graph is useful, but removing unrelated expressions or assumptions must preserve query meaning. Reuse Z3 scopes where useful and do not assume incremental mode is always faster.

Deliverables: resource controls, analysis statistics, cache behavior and a reproducible benchmark harness.

Benchmark families:

| Family | What it measures |
| --- | --- |
| Contractor reference cases | Correctness and ordinary end-to-end latency |
| Independent threshold rules at 10, 100 and 1000 rules | Model size versus solver cost |
| Heavily overlapping guards | Conflict-query growth |
| Many same-constructor payloads | Data-sensitive conflict handling |
| Coupled Boolean conditions | Difficult search despite small input text |
| Shared derived DAGs | Resistance to expression duplication |
| Near model limits | Bounded planning and memory behavior |
| Repeated one-rule edits | Cache invalidation and incremental loop behavior |

Report p50/p95 wall-clock times over repeated runs, peak worker memory where measurable, query counts, formula sizes, cache hit rate and inconclusive rate. Record hardware, build profile and solver versions. These measurements establish initial interactive budgets; the specification's defaults are provisional limits, not benchmark claims.

Exit criteria: no unfinished obligation is marked satisfied; completed results survive a later timeout; pair generation respects limits without quadratic allocation; changes to assumptions or semantics invalidate relevant caches; changes to source notes update result provenance without stale claims.

## Milestone 5: persistent worker and Gleam boundary

Implement the framed worker protocol and the five operations. Keep stdout clean, process partial reads/writes correctly, and implement worker-local handles with explicit release and capacity errors. Run one request at a time. Expose supported versions and capabilities through `hello`.

Supply protocol fixtures and a transport-independent client contract suitable for a typed Gleam wrapper. Do not expose Z3 contexts or Rust pointers. The Gleam integration should supervise the OS process, queue requests, terminate and reap it on a hard deadline, and recompile durable model JSON after restart.

Deliverables: worker executable, request/response examples, protocol tests and integration documentation. A production Gleam client can be a follow-up task; the Rust engine is complete when its protocol is independently exercised.

Exit criteria: compile/analyze/evaluate/release work across one long-lived process; multiple framed requests work; oversized and malformed frames fail predictably; restart invalidates handles; parent-side termination leaves no native worker behind. A deliberately slow request does not require a BEAM NIF.

## Verification that matters

Test the semantic boundaries rather than snapshotting every line of implementation.

1. Differentially compare solver findings with exhaustive concrete evaluation over small finite models. For symbolic integer models, bound the inputs explicitly for this comparison; do not confuse that bounded test with the unbounded solver claim.
2. Deliberately omit a rule guard and verify an independent constraint catches the violation.
3. Deliberately add an impossible domain and verify the engine suppresses vacuous success.
4. Exercise overlapping identical outputs, different output tags and different payloads.
5. Generate supported typed expressions and compare concrete evaluation with solver evaluation under fixed assignments, including large integers and date boundaries.
6. Verify that metadata changes cannot alter mathematical rule meaning, but do update review/provenance reporting.
7. Inject backend UNKNOWN, timeout, cancellation and malformed witness results to test reporting without relying on solver-specific timing luck.
8. Test process termination, exit handling and exact protocol framing.

No test can establish that Author extracted the intended business policy from prose. Human-approved scenarios and source interpretation belong to the Author evaluation layer. V0 solver UNSAT results rely on the solver and translation; independently checked proof certificates are outside the acceptance criteria.

## Acceptance demonstration

An implementer should be able to show this sequence from the supplied fixtures:

1. Compile the source-linked partial contractor model; see that behavior and lifecycle enforcement remain unspecified.
2. Analyze the unsafe grant proposal; obtain an approval-present case whose requested expiry exceeds contract end.
3. Analyze the repaired grant; see compliance with the two constraints and an uncovered input.
4. Analyze the complete proposed policy; see snapshot coverage and uniqueness alongside the unresolved sufficiency question and lifecycle requirement.
5. Add a contradictory denial rule; obtain the exact conflicting input and rule IDs.
6. Select an approval assumption explicitly; see conditional scope in the result.
7. Analyze the impossible-domain fixture; see `empty_domain` and skipped dependent checks.
8. Run the same model through the worker, then restart it and demonstrate handle invalidation and recompilation.

## Deferred work and triggers

| Extension | Trigger for considering it |
| --- | --- |
| Human-readable DSL | Add a renderer/parser over the same typed model when useful; test normalized structure and metadata round trips, with no text-only behavior |
| Dynamic state model | A real method needs lifecycle properties that snapshots cannot establish |
| Aggregate operations | Representative methods require them; define verified encodings and collection scope |
| Lean certificates | Proof assurance requirements justify certificate and translation verification work |
| NIF adapter | Profiling shows transport overhead matters and scheduling/cancellation are designed |
| More concurrency | Analysis queues justify a bounded worker pool |
| Rich explanation minimization | Raw counterexamples or contradiction groups impede analyst review |

## Remaining implementation decisions

No product semantics block the first version. The first build milestone must settle native Z3 packaging and tested platforms. Benchmarking must settle practical budget defaults. Those decisions can be made within this plan without changing the language or delaying the semantic prototype.

## Suggested implementation-agent brief

Implement the v0 Rust decision-analysis engine from this specification and schema. Use the provided fixtures as behavioral contracts, not as the only tests. Preserve partial-model semantics, exact arithmetic, independent constraints and explicit assumptions. Reach the end-to-end CLI milestone before worker integration or optimization. Do not add a business executor, custom DSL, NIF, arbitrary SMT endpoint or Lean backend. Record any semantics ambiguity before choosing behavior; routine crate and build choices are yours to make. Demonstrate the acceptance walkthrough, meaningful differential tests, budget behavior and worker recovery.
