# Entail Author Decision Engine

Entail v0 validates and analyzes an explicitly authored model of one business decision. Input is the versioned JSON AST in [`author-engine-v0/schema/model-v0.schema.json`](author-engine-v0/schema/model-v0.schema.json); semantics and protocol are specified in [`author-engine-v0/author-engine-v0-spec.md`](author-engine-v0/author-engine-v0-spec.md).

## Build and run

The reproducible development environment is pinned in [`BUILD.md`](BUILD.md). The supported/tested route is x86_64 Linux with Nix and mise; stay inside `nix develop` when running the dynamically linked Z3 executable.

```sh
mise install
mise run check
nix develop --command cargo build --workspace --bin entail --locked
```

Example commands using the included partial-policy fixture:

```sh
nix develop --command target/debug/entail validate author-engine-v0/fixtures/valid/partial.json
nix develop --command target/debug/entail analyze author-engine-v0/fixtures/valid/partial.json --checks coverage
nix develop --command target/debug/entail evaluate author-engine-v0/fixtures/valid/unsafe-grant.json author-engine-v0/scenarios/unsafe-expiry.json
```

They emit JSON. Validation checks the model; analysis returns typed obligation results, witnesses and unresolved metadata; evaluation applies one concrete typed input scenario. A usable persistent process is available as `entail worker`; its 4-byte big-endian length-prefixed JSON protocol and lifecycle contract are documented in [`docs/WORKER.md`](docs/WORKER.md).

Black-box checks against the built executable:

```sh
nix develop --command python3 scripts/acceptance.py target/debug/entail
nix develop --command python3 scripts/worker_acceptance.py target/debug/entail
```

Final integration commands and acceptance coverage are tracked in [`docs/DELIVERY.md`](docs/DELIVERY.md). The archived release worker smoke includes analysis/cache/query/native-statistics and sampled RSS measurements from an earlier source snapshot; it is single-host historical smoke evidence, not a final-revision performance claim or guarantee.

## Correctness boundary

The engine checks a formalization; it does not discover or establish business truth:

- Source references record provenance, not whether a source entails the interpretation.
- Rules participate in analysis whether marked proposed or confirmed. Review metadata is not business approval.
- Findings are relative to the declared inputs, domain, rules, constraints, and explicitly selected assumptions. Conditional conclusions must retain that scope.
- No findings do not establish model completeness, fairness, legal sufficiency, or safety. Partial models and unresolved items remain material.
- V0 analyzes a single snapshot; it does not prove lifecycle behavior or execute business effects, and it cannot silently infer omitted requirements or external facts.

Use results as evidence for human review, not as business authorization. Fixtures and benchmarks test the engine, not the truth or appropriateness of a policy.
