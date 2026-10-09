# Author decision engine v0 specification

Status: implementation-ready draft. Version: 0.1. Date: 2026-10-09.

Author discovers a business method from documents and conversation. This engine accepts a formal model of a single decision, validates it, and checks its proposed behavior against independently represented requirements. It returns counterexamples, conditional results and unresolved authoring gaps. It does not discover requirements or execute business effects.

The engine is a Rust library with a thin CLI and a persistent worker process. Gleam owns the Author loop, storage, analyst interaction and supervision. The first formal language is a versioned JSON AST defined by `schema/model-v0.schema.json`. Human-readable syntax can be added over the same typed model later.

## 1. Version boundary

V0 analyzes a deterministic decision at a single snapshot of its inputs. It supports partial models, scalar typed inputs, named derived expressions, deterministic outcome constructors, independent constraints, explicit assumptions and unresolved items. Snapshot analysis is not a proof of lifecycle behavior.

Include: Boolean logic, finite enums, exact integers with units, calendar dates, linear arithmetic, conditional expressions, coverage, reachability, outcome conflicts, constraint compliance and scenario evaluation.

Exclude: source discovery, arbitrary natural-language compilation, external calls, effects, state transitions, concurrency, recursive derivations, user-written loops, unbounded collections, unrestricted quantifiers, floating point, nonlinear multiplication, general division, code generation and a business runtime. Preserve excluded requirements as open items; do not silently discard them.

The JSON language is the formal specification language, not a transport for opaque strings that another LLM interprets. Unknown node kinds, undeclared references and unsupported operations are compilation errors.

### Representation and text mapping

The typed model is the semantic authority. JSON is its first external encoding. JSON Schema saves a custom parser and provides structural generation constraints; it does not replace name resolution, type checking or semantic validation. A small text parser would be a manageable frontend, not a different engine architecture.

Use the following stable readable notation for model explanations and a future text frontend. This is the reference excerpt, not a supported v0 input or a complete grammar. Enum definitions and source records live in the full model.

```text
decision ContractorAccess

input sponsor_approval: ApprovalStatus
input contract_end: Date
input requested_expiry: Date

outcome Grant(expiry: Date)
outcome Deny(reason: DenialReason)

constraint ApprovalRequired
  when outcome is Grant(expiry)
  require sponsor_approval == Approved
  source policy.section_2

constraint ExpiryWithinContract
  when outcome is Grant(expiry)
  require expiry <= contract_end
  source policy.section_3

rule GrantApprovedRequest
  when sponsor_approval == Approved
   and requested_expiry <= contract_end
  conclude Grant(requested_expiry)
  status proposed

unresolved OtherPrerequisites
  "Are approval and a valid expiry sufficient to grant access?"
```

Do not replace this with shorthand arrows, `for Grant(...)`, renamed fields or payload-free denial examples. The readable vocabulary and JSON model must describe the same constructors and fields. `status proposed` maps to `origin.review = "proposed"`. A `source policy.section_2` label refers to the declared source record with that locator; the JSON origin stores its stable source ID. Full provenance annotations must be representable by the eventual text grammar without changing this statement style.

The following mapping is the frontend design contract.

| Typed construct / JSON shape | Corresponding text form |
| --- | --- |
| Enum with named variants | `type ApprovalStatus = Approved \| NotApproved \| Unknown` |
| Typed input | `input sponsor_approval: ApprovalStatus` |
| Outcome with typed fields | `outcome Grant(expiry: Date)` and `outcome Deny(reason: DenialReason)` |
| Equality expression | `sponsor_approval == Approved` |
| `and` expression | `condition_a and condition_b` |
| Rule guard and outcome constructor | A `rule` block with `when`, `conclude` and `status` lines |
| Outcome-targeted constraint | A `constraint` block with `when outcome is Grant(expiry)`, `require expiry <= contract_end` and `source` lines |
| Named assumption | `assumption SponsorsAlreadyApproved require sponsor_approval == Approved` |
| Open item | An `unresolved` block followed by the quoted question; its category is explicit metadata in the full model |

Both frontends must resolve to identical typed nodes, with identical rule aggregation, missing-case behavior and assumption selection. Text must preserve source statements, declaration IDs, origins, review status, notes and open items using explicit declarations/annotations. They cannot be encoded only as discardable comments. Integer units and exact date values must remain explicit. Text source spans may differ from JSON Pointers but must reference the same declarations.

When a text frontend is implemented, require JSON -> typed model -> canonical text -> typed model and text -> typed model -> canonical JSON -> typed model tests. Equality is equality of normalized typed structure including metadata, excluding parser source spans and insignificant declaration ordering; original whitespace and JSON key order need not round-trip. Use parentheses or a defined precedence to preserve expression trees. No text-only priority, implicit defaults or extra expressions are allowed. A new backend never consumes unparsed text.

V0 implements JSON ingestion. A readable renderer and then a parser can be added without changing semantics; the engine core API accepts the typed model rather than JSON-specific values. The exact text grammar is deferred, not the semantic mapping requirement.

## 2. Model structure

Required top-level fields: `language_version`, `model_id`, `sources`, `enums`, `inputs`, `derived`, `outcomes`, `domain`, `assumptions`, `rules`, `constraints`, `open_items`. The version is exactly `author-model/0.1`. Declaration arrays may be empty. A model with no inputs or no outcomes is a permitted draft, with analysis prerequisites reported as incomplete.

`model_id` is a caller-supplied identifier. All declaration IDs, except source IDs and enum variants, share one namespace and must be unique. IDs match `[A-Za-z][A-Za-z0-9_]*`. Source IDs are unique in their own namespace; variants are unique within an enum. Inputs are flat fields. Nested business records can be flattened by Author only through explicit named inputs, retaining an open item if that abstraction omits relevant structure.

Every formal declaration has `origin`: `kind` (`source`, `proposal`, or `assumption`), `source_ids`, `review` (`proposed` or `confirmed`), and optional `note`. `source` origins require at least one resolvable source ID. Review status does not change semantics or automatically suppress proposed rules. All declared rules participate. A result separately reports whether participating declarations are proposed.

Assumption declarations must use origin kind `assumption`, and that kind is forbidden on other formal declarations. Assumptions become active only through their explicitly selected IDs. Metadata cannot turn a requirement or rule into an analysis assumption.

Sources contain `id`, exact `text`, and optional `locator`. The engine checks references, not whether a passage entails a formal interpretation. A source-linked declaration can still be wrong.

### Types and values

| Type | Representation and semantics |
| --- | --- |
| Boolean | `{"kind":"bool"}`; exactly true or false |
| Enum | `{"kind":"enum","name":"ApprovalStatus"}`; a named nonempty finite set of variants |
| Integer | `{"kind":"int","unit":"count"}`; arbitrary-precision mathematical integer; unit required |
| Date | `{"kind":"date"}`; Gregorian date, years 0001 through 9999 |

Integer literals use canonical decimal strings, never JSON numbers: `0` or an optional minus followed by a nonzero digit and remaining digits. This avoids narrowing in a Gleam or JavaScript caller. No leading zeros, `-0`, exponent notation or overflow to machine integers. Unit tags are opaque identifiers: equal tags are required for arithmetic and comparisons. The engine does not infer conversions.

Dates use valid canonical `YYYY-MM-DD` strings. Lower dates to exact day ordinals with the corresponding valid range constraints. Date comparison and equality are available; date arithmetic and timestamp conversion are not. The engine cannot infer time zones or what "end of day" means.

Uncertainty is modeled explicitly using enum variants such as `Approved`, `NotApproved`, and `Unknown`. `Unknown` is an ordinary declared value. It is not Boolean false, an authoring hole or a request for the solver to choose a hidden actual value. V0 has no nullable or optional type; optional fields can use an explicit enum plus separate data and domain conditions.

Outcomes have an ID, typed fields and origin. Zero fields are valid. Two outcome values are equal exactly when their constructor IDs and every typed field value are equal. Overlapping rules that produce identical values are not a conflict.

### Expressions

Every expression is a tagged JSON object; see the schema for exact shapes.

| Kind | Fields | Requirement |
| --- | --- | --- |
| `bool` | `value` | Boolean literal |
| `int` | `value`, `unit` | Exact integer literal |
| `date` | `value` | Valid calendar date |
| `enum` | `enum`, `variant` | Declared variant |
| `ref` | `path` | `input.ID`, `derived.ID`, or `outcome.FIELD` |
| `not` | `arg` | Boolean argument |
| `and`, `or` | `args` | One or more Boolean arguments |
| `eq`, `ne` | `left`, `right` | Same type; no implicit coercion |
| `lt`, `le`, `gt`, `ge` | `left`, `right` | Same-unit integers or dates |
| `add`, `sub` | `left`, `right` | Same-unit integers |
| `scale` | `factor`, `arg` | Canonical integer constant times integer expression; retains unit |
| `if` | `condition`, `then`, `else` | Boolean condition and same-typed branches |

Derived declarations have an explicit type and expression. References may be forward references, but the complete dependency graph must be acyclic. Derived expressions refer only to inputs and other derived declarations. Never inline shared expressions into exponentially larger trees; retain a DAG or named SMT bindings.

Domain predicates, assumptions and rule guards are Boolean and refer only to inputs and derived declarations. A rule's `then` contains an outcome ID and an expression for each field, with no extra or missing fields. Its fields also refer only to inputs and derived declarations. Rules cannot refer to another rule's firing or outcome.

A constraint names exactly one `for_outcome` constructor. Its Boolean `must` expression can also reference that constructor's fields through `outcome.FIELD`. An absent field is a compile error, even if a branch appears to make it unreachable. All expressions are total over their typed domain.

## 3. Decision semantics

Let `T(x)` encode the input types, including enum membership and valid date bounds. Let `D(x)` be the conjunction of all declared domain predicates; an empty conjunction is true. Analysis explicitly selects assumption IDs. Let `A(x)` be their conjunction; no assumptions are selected by default.

The analysis domain is `B(x) = T(x) AND D(x) AND A(x)`.

For each rule i, `g_i(x)` is its guard and `y_i(x)` its deterministic outcome value. The proposed decision relation is:

`M(x, y) = OR_i (g_i(x) AND y = y_i(x))`.

All applicable rules contribute. There is no rule order, implicit priority, first-match behavior, fallback or requirement-based filtering. No applicable rule means no specified outcome. Multiple distinct values mean incompatible behavior for this deterministic decision.

For an outcome y, `Req(x, y)` is the conjunction of every constraint targeted at its constructor. Other constructors are not subject to that constraint. The compiler MUST NOT add requirements to guards, the input domain, or solver assumptions when checking compliance. Doing so could conceal an incorrect rule.

Domain predicates restrict input scope, not outputs. They require the same visible provenance as other statements. Automatically run the base-domain feasibility check and report all domain restrictions alongside the selected assumptions. A restriction such as "every input is approved" must not be concealed as an inferred type rule.

Open items contain an ID, text, source IDs and an `affects` category: `input_domain`, `outcome_schema`, `rule_policy`, `constraint`, `lifecycle`, or `interpretation`. They have no execution semantics. All remain visible in reports. Open items do not force a solver result to become unknown, but they prevent a claim that the business method is complete. V0 never computes an overall "business verified" Boolean.

## 4. Check catalog

For violation queries below, SAT means a finding exists; UNSAT means no such finding under the exact model and analysis context. UNKNOWN, timeout, cancellation, skipped and engine error are distinct. Solver success alone must not be reported as business correctness.

| Check | Solver question | SAT result | UNSAT result |
| --- | --- | --- | --- |
| Domain feasibility | `B(x)` | `feasible`, with witness | `empty_domain` |
| Rule reachability i | `B(x) AND g_i(x)` | `reachable` | `unreachable` |
| Coverage | `B(x) AND NOT OR_i g_i(x)` | `uncovered`, with witness | `covered` |
| Outcome conflict i,j | `B(x) AND g_i(x) AND g_j(x) AND y_i(x) != y_j(x)` | `conflict`, with witness | `compatible` |
| Rule compliance i,c | `B(x) AND g_i(x) AND NOT c(x,y_i(x))`, only for the rule's outcome | `violation`, with witness | `compliant` |

Always run domain feasibility first. If it is UNSAT, skip dependent checks with reason `empty_domain`; do not report coverage, compatibility or compliance success. If feasibility is inconclusive, dependent queries may still produce SAT findings, but mark any UNSAT conclusions `feasibility_unconfirmed`.

A structurally empty draft returns `incomplete_model` for dependent checks. With declared inputs and outcomes but no rules, coverage returns `behavior_unspecified` rather than requesting arbitrary uncovered witnesses. Compliance requires an applicable constraint. A constructor with none produces `no_requirements_for_outcome`, not a success claim. A model can have zero inputs as a constant decision only when it declares outcomes and rules; then the input assignment is the single empty tuple.

Compliance results include rule reachability status. `compliant` for an unreachable rule is labeled `vacuous_unreachable_rule`. Matching outcome tags is only a first filter; payload differences can cause conflict even when tags match.

Scenario evaluation requires a complete concrete assignment to declared inputs. Validate types, compute derived expressions and evaluate domain and selected assumptions. An excluded scenario is reported as `outside_domain`, not as denied. Otherwise return matched rule IDs, distinct outcome values and violated constraint IDs. Classify the result as `unspecified`, `unique`, or `conflicting`. An optional expected outcome produces `agrees`, `disagrees`, or `unspecified`; a conflicting model remains conflicting even if one outcome matches the expectation. Do not introduce a solver dependency for concrete evaluation.

Assumption sensitivity is an opt-in orchestration operation: repeat an identified check after removing selected assumptions. Report the changed context and results, not a minimum or causal explanation of assumption dependence. Minimum explanations are outside v0.

All witness values are typed business values. Re-evaluate SAT witnesses through the typed model evaluator, using the same domain, assumptions and obligation. A failed replay is `engine_error/witness_mismatch`, never a valid finding. Decode all inputs using solver model completion where needed. Preserve exact integers and canonical dates. A witness is not promised to be smallest, most realistic or unique.

## 5. Results and provenance

Every analysis result includes `language_version`, `engine_version`, `solver_version`, `model_id`, `content_hash`, `semantic_hash`, `selected_assumption_ids`, domain predicate IDs, check options, participating proposed declarations, and open items.

Each obligation result includes a stable obligation ID, check kind, relevant declaration IDs, `status`, optional finding category, witness, relevant source IDs, conditional or vacuity labels, elapsed time and solver statistics when available. Status is one of `satisfied`, `finding`, `unknown`, `timeout`, `cancelled`, `skipped`, or `error`. Feasibility and reachability use `satisfied` for their SAT positive answer; violation-oriented checks use `finding` for SAT.

Results that use selected assumptions are visibly conditional. An UNSAT answer is trusted according to the correctness of our translation and Z3. V0 does not emit Lean-checked proof certificates. Do not describe SMT results as independently checked proofs. An optional future certificate backend must validate the same obligation and preserve its assumptions.

Diagnostic codes are stable identifiers with declaration ID, JSON Pointer where possible, message and source IDs. Required classes: syntax/schema, duplicate name, unknown reference, unknown variant, type mismatch, unit mismatch, invalid date, unsupported construct, derived cycle, missing/extra outcome field, source-reference error, version mismatch and resource limit. Error messages should help Author repair the model without exposing raw solver internals by default.

`content_hash` is SHA-256 of canonical JSON for the entire submitted model. Canonicalize object keys and preserve array order. `semantic_hash` covers the validated typed model, including declaration IDs, types, domains, assumptions, rule and constraint expressions; omit source text, notes, review status and open-item prose. Explicitly serialize this representation; never hash Rust debug output. Cache obligation answers by semantic hash, obligation signature, selected assumptions, engine/encoding version and solver configuration. Always reconstruct report provenance and readiness from the current content. Do not reuse UNKNOWN/timeout results as completed proofs.

## 6. Rust components

Use one Cargo workspace. Keep the semantic library independent of BEAM, process transport and Z3 objects.

| Component | Responsibility |
| --- | --- |
| `author_model` | JSON types, parser/schema-aligned validation, typed expression DAG, evaluator, provenance, hashes |
| `author_analysis` | Check planning, prerequisite handling, obligation definitions, result mapping, backend trait |
| `author_z3` | Native Z3 bindings, exact lowering, model decoding, solver limits and interruption |
| `author_worker` | CLI, framed protocol, model handles, request lifecycle and worker diagnostics |

A compiler returns either a validated model plus diagnostics or fatal diagnostics. Valid partial models are accepted. Derived dependencies are topologically sorted. Identify unsupported expressions before solver invocation. Maintain a narrow solver-independent obligation representation; do not build a general theorem-prover abstraction with multiple backends in v0.

Use Z3 through the Rust `z3` crate. Pin Rust dependencies and a tested native Z3 version during the first build milestone. The exact package versions and linking mode are recorded after a native smoke build, not guessed in this specification. Ship a documented local dependency route first. A fully static portable binary is not a v0 acceptance requirement.

## 7. Worker and CLI contract

The worker is a separate persistent OS process supervised by Gleam. One worker executes one request at a time. Parallel jobs use separate workers; the initial pool can contain one worker. Compiled models and native contexts are worker-local. Rust core APIs can remain synchronous.

Stdin/stdout frames: 4-byte unsigned big-endian payload length followed by UTF-8 JSON bytes. Stdout contains only frames; logging goes to stderr. Default maximum frame is 8 MiB. Read/write complete frames, tolerate partial pipe reads and writes, and reject unknown protocol versions. A malformed JSON frame yields a protocol error if its request ID is recoverable; an invalid length closes the worker with a nonzero exit.

Request envelope: `{"protocol_version":"author-worker/0.1","request_id":"r1","op":"compile","params":{...}}`.

| Operation | Params | Response payload |
| --- | --- | --- |
| `hello` | empty object | Versions, supported language/checks, configured limits |
| `compile` | `model` JSON | Opaque model handle, hashes, compile diagnostics and readiness |
| `analyze` | `handle`, `checks`, `assumption_ids`, `budget` | Context and ordered per-obligation results |
| `evaluate` | `handle`, `inputs`, `assumption_ids`, optional `expected_outcome` | Concrete evaluation and expectation comparison |
| `release` | `handle` | Whether the handle existed and was released |

Success envelope: protocol version, request ID and `result`. Failure envelope: protocol version, request ID and `error` with stable code and message. `compile` validation errors are returned in a normal compile result with no handle; transport/internal failures use `error`. Unknown or expired handles return `unknown_handle`. Handles are invalid after process restart and must not be persisted as durable business state.

`checks` is a list drawn from `domain_feasibility`, `rule_reachability`, `coverage`, `outcome_conflict`, and `constraint_compliance`. Empty means only mandatory feasibility. The worker deduplicates names and plans obligations deterministically by IDs. No arbitrary SMT, scripts or user-defined checker code is accepted.

Budget fields: `total_timeout_ms`, `per_obligation_timeout_ms`, `max_obligations`, and `solver_resource_limit` (optional, nonzero). Proposed defaults are 5000 ms total, 1000 ms per obligation and 10000 obligations. These are product budgets, not performance measurements. Larger explicit budgets require no special user approval. Clamp them to configured worker limits and report effective values. Preserve completed results when later work reaches a limit; report unstarted work as skipped with its reason and overall `complete=false`.

V0 has no in-band cancellation operation: the request loop is serial. Solver timeout and resource budgets interrupt normal analysis. For a hard deadline or user cancellation, the Gleam supervisor terminates and restarts the OS process, explicitly ensures it is reaped, and recompiles models as needed. Closing a port must not be assumed to kill a native process. A future asynchronous cancel protocol is a separate extension.

CLI commands: `validate MODEL`, `analyze MODEL --checks ... --assumptions ... --budget-ms ...`, `evaluate MODEL SCENARIO`, and `worker`. Standalone commands print JSON. Exit codes: 0 means the command completed normally, 2 means invalid input, 3 means internal/backend failure. Findings and inconclusive analysis are structured results, not process failures. The result's `complete` field distinguishes an exhausted analysis from a completed one.

## 8. Resource and performance behavior

Initial model limits: 256 inputs, 2048 rules, 2048 constraints, 50000 expression nodes and expression depth 128. These are configurable guards, not claims that every accepted model is cheap to solve. Limits apply before expensive construction. Cap retained compiled models with a configurable worker limit; when full, return `capacity_exceeded` until a handle is released. Do not silently invalidate active handles.

Pairwise conflict generation can be quadratic. Generate pairs lazily, skip provably identical outputs and use cheap sound disjointness checks. Stop at obligation budgets without allocating the complete pair list. Rule-compliance generation considers only matching outcome constructors. Preserve shared expressions, cache completed obligations, and reuse common solver assertions where profiling supports it. Soundness takes precedence over pruning.

No latency guarantee precedes benchmarking. Record compilation time separately from solving, peak worker memory where available, query count, cache behavior, formula size and timeout rate. A small adversarial Boolean model can be harder than a large independent threshold policy.

## 9. Reference acceptance cases

The bundle contains a language schema, valid and invalid model fixtures, scenario examples and expected findings. They are contract examples, not evidence that an engine already exists.

1. Partial constraints with no rules compile; coverage is `behavior_unspecified`.
2. Grant-on-approval without an expiry guard violates the expiry requirement.
3. Adding the expiry guard removes that violation, but leaves uncovered inputs.
4. A proposed deny rule for every other case makes coverage complete and outcomes unique within the declared domain.
5. A simultaneously applicable Grant and Deny produces a conflict.
6. Two identical Grant values from overlapping rules do not conflict.
7. Two Grant values with different expiry payloads conflict.
8. Selecting an approval assumption narrows scope and is reported; it is never selected implicitly.
9. An impossible domain causes dependent checks to be skipped instead of reporting success.
10. Unknown approval is not silently treated as approved.
11. Arbitrary-precision arithmetic survives JSON, evaluator and solver round trips.
12. Unknown references, unit mismatches, invalid dates and derived cycles fail before solving.
13. Solver witnesses replay in the evaluator; mismatches are engine errors.
14. Timeouts, job limits and restart behavior never yield success for unfinished obligations.

The deny rules in the complete-policy fixture are proposed policies for testing engine behavior, not requirements derived from the two source sentences. An open item asks whether granting conditions are sufficient. Even mathematically complete fixture behavior is not a confirmed business policy.

## 10. Extension points

Add textual syntax only as another frontend to the typed model. Add lifecycle actions as explicit state relations and property templates; preserve the snapshot semantics of v0. Add reviewed library operations for calculations only with total semantics and supported verification encodings. Add a Lean certificate backend only when proof assurance justifies its engineering cost. Add a NIF adapter only after measuring the process boundary and defining long-running job scheduling; keep the Rust core reusable.

These extensions do not authorize silently accepting an unsupported v0 model. The engine always reports what it analyzed and what remains outside its scope.

## Technical references

- SMT-LIB QF_LIA definition: https://smt-lib.org/logics-all.shtml
- Rust Z3 API: https://docs.rs/z3/latest/z3/
- Z3 scopes: https://microsoft.github.io/z3guide/docs/logic/basiccommands/
- Z3 budgets and parameters: https://microsoft.github.io/z3guide/programming/Parameters/
- BEAM native integration tradeoffs: https://www.erlang.org/doc/system/nif.html

