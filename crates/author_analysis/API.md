# `author_analysis` public API (v0)

This is the solver-independent seam shared with `author_z3` and `author_worker`. The analysis crate depends on `author_model`, never on a solver implementation.

## Entry point

```rust
pub fn analyze<B: Backend>(
    model: &CompiledModel,
    options: &AnalysisOptions,
    backend: &mut B,
) -> Result<AnalysisReport, Diagnostic>
```

`AnalysisOptions` selects checks, assumption declaration IDs and budgets. Empty `checks` requests mandatory domain feasibility only. Domain feasibility is always run first regardless of requested checks. Default checks and budgets are explicit and serializable; worker-facing JSON uses check names `domain_feasibility`, `rule_reachability`, `coverage`, `outcome_conflict`, and `constraint_compliance` and budget fields `total_timeout_ms`, `per_obligation_timeout_ms`, `max_obligations`, and optional `solver_resource_limit`.

## Backend contract

```rust
pub trait Backend {
    fn solve(
        &mut self,
        model: &CompiledModel,
        obligation: &Obligation,
        assumption_ids: &[String],
        budget: &SolveBudget,
    ) -> BackendAnswer;

    fn solver_version(&self) -> String;
    fn solver_configuration(&self) -> String { String::new() }
}
```

The stable solver-independent `Obligation` variants are:

```rust
DomainFeasibility,
RuleReachability { rule_id: String },
Coverage,
OutcomeConflict { left_rule_id: String, right_rule_id: String },
ConstraintCompliance { rule_id: String, constraint_id: String },
```

`BackendAnswer` is the enum `Sat { inputs: InputValues }`, `Unsat`, `Unknown { reason }`, `Timeout`, `Cancelled`, or `Error { code, message }`. Analysis records per-obligation and report elapsed time, cache hits, solver query count, and planned/executed obligation counts. Backends may return `SolverStatistics` through the defaulted `Backend::statistics` hook. Every SAT input assignment is independently replayed through `author_model` before it can be reported or cached.

`SolveBudget` contains the effective per-query timeout and optional solver resource limit. `AnalysisBudget` contains total timeout, per-obligation timeout, maximum obligations and optional solver resource limit. Resolved defaults and effective values are reported.

## Result contract

All public options, obligations, statuses, findings, results, context and reports derive serde serialization/deserialization with stable snake_case tags. An `AnalysisReport` includes model/language/engine/solver versions, current content and semantic hashes, selected assumptions, domain predicate IDs, selected checks, proposed participating declaration IDs, current open items, ordered obligation results, effective budget, and `complete`.

Compliance options also schedule rule-reachability obligations so each compliance result can include its rule's reachability status and the `vacuous_unreachable_rule` label. Each result has a deterministic obligation ID, obligation/check kind, referenced declaration IDs, status (`satisfied`, `finding`, `unknown`, `timeout`, `cancelled`, `skipped`, `error`), optional finding category and witness, source IDs, conditional and vacuity labels, elapsed time/statistics, and an optional skip reason. Domain feasibility categories are `feasible` and `empty_domain`; reachability `reachable` and `unreachable`; coverage `uncovered` and `covered` (or `behavior_unspecified` when there are no rules); conflict `conflict` and `compatible`; compliance `violation` and `compliant`, with `no_requirements_for_outcome` and `vacuous_unreachable_rule` represented distinctly.

Completed SAT/UNSAT answers alone may be cached. A bounded process-local cache keys semantic hash, exact obligation, assumption IDs, engine/encoding version, backend solver configuration/version, and effective solve budget. UNKNOWN, timeout, cancellation and errors are not cached. Current report metadata is rebuilt on every analysis.

Pair and compliance obligations are generated lazily; no complete quadratic pair list is materialized. Identical rule conclusions and literal-false guards are soundly pruned from pair queries. Work is ordered canonically by declaration IDs, independent of source-array order. Exhausted total time or obligation count preserves completed results, emits a skipped-work marker, sets `remaining_work=true`, and sets `complete=false`. A SAT witness that does not replay as satisfying the exact obligation is reported as `error` with stable code `witness_mismatch` and is not cached.
