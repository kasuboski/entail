# Entail Author Decision Engine

Entail v0 is a Rust engine for validating and analyzing an explicitly authored model of one business decision. Its intended input is the versioned JSON AST in [`author-engine-v0/schema/model-v0.schema.json`](author-engine-v0/schema/model-v0.schema.json); the language and result semantics are specified in [`author-engine-v0/author-engine-v0-spec.md`](author-engine-v0/author-engine-v0-spec.md).

**This integration is in progress.** The model compiler/evaluator, analysis planner, native Z3 backend, and framed transport are integrated. The `entail` binary is still a foundation stub; the CLI and persistent worker implementation are not yet committed to this integration branch. Do not use the current binary to evaluate policy or treat a successful library build as a decision result. Current component and end-to-end evidence is tracked in [`docs/DELIVERY.md`](docs/DELIVERY.md).

## Build and test

The reproducible development environment is pinned in [`BUILD.md`](BUILD.md). On the supported x86_64-linux Nix environment:

```sh
mise install
mise run check
```

This runs formatting, strict Clippy, Rust 1.85 MSRV checking, and the workspace test suite using the pinned native Z3 5.1.0 and Rust `z3` crate 0.19.4. `mise run check` is the required integration gate; while components are being developed, its current result and any blockers are recorded in the delivery evidence.

## Correctness boundary

The engine checks a formalization; it does not discover or establish business truth. In particular:

- Source references record provenance, not whether a source entails the interpretation.
- Rules are proposed behavior and participate in analysis whether marked proposed or confirmed. Review metadata is not approval.
- Findings are relative to the declared inputs, domain, rules, constraints, and explicitly selected assumptions. Conditional conclusions must retain that scope.
- An analysis without findings does not establish that the model is complete, fair, legally sufficient, or safe for every lifecycle state. Partial models and unresolved items must remain visible.
- V0 is snapshot analysis, not a business-effect executor or proof of lifecycle behavior. It cannot silently infer omitted requirements or external facts.

Use the specification and the actual results—not fixture labels or a clean solver run—to decide whether a formalization is appropriate. Acceptance scripts and benchmark harnesses are evidence tools, not proofs of correctness; see [`docs/ACCEPTANCE.md`](docs/ACCEPTANCE.md) and [`docs/BENCHMARKS.md`](docs/BENCHMARKS.md).
