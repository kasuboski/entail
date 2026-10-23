# Entail Author Decision Engine

Entail v0 validates and analyzes an explicitly authored model of one business decision. Models can use the versioned JSON AST in [`docs/schema/model-v0.schema.json`](docs/schema/model-v0.schema.json) or the readable text syntax in [`docs/SPEC.md`](docs/SPEC.md); both compile to the same typed model.

## Build and run

The supported/tested development route is x86_64 Linux with mise and Nix. `mise.toml` pins Rust 1.98.0 and cargo-nextest; `flake.nix` supplies GCC, pkg-config, Z3, libclang and Python, with the Z3 library path configured for the dynamically linked executable. Stay inside `nix develop` when building or running it.

```sh
mise install
mise run check
mise exec -- nix develop --command cargo build --workspace --bin entail --locked
```

Example commands using the included partial-policy fixture:

```sh
nix develop --command target/debug/entail validate docs/examples/models/partial.json
nix develop --command target/debug/entail analyze docs/examples/models/partial.json --checks coverage
nix develop --command target/debug/entail evaluate docs/examples/models/unsafe-grant.json docs/examples/scenarios/unsafe-expiry.json
nix develop --command target/debug/entail validate docs/examples/contractor-access.author
nix develop --command target/debug/entail fmt --check docs/examples/models/*.author docs/examples/contractor-access.author
```

Validation, analysis, and evaluation emit JSON. They accept either JSON or text models; scenario assignments remain JSON. `entail fmt` canonicalizes text from stdin to stdout, rewrites named text files, or checks them with `--check`. Analysis returns typed obligation results, witnesses and unresolved metadata; evaluation applies one concrete typed input scenario. A usable persistent process is available as `entail worker`; its 4-byte big-endian length-prefixed JSON protocol and lifecycle contract are documented in [`docs/WORKER.md`](docs/WORKER.md).

Black-box checks against the built executable:

```sh
nix develop --command python3 scripts/acceptance.py target/debug/entail
nix develop --command python3 scripts/worker_acceptance.py target/debug/entail
```

The CLI returns 0 for findings and 2 for invalid model/file/JSON input. Backend internal errors map to exit 3, but black-box acceptance cannot induce that path because the public CLI has no deterministic backend-fault injection seam; mapping is tested at the analysis boundary. `scripts/acceptance.py` and `scripts/worker_acceptance.py` provide black-box checks against the built executable. Python helpers can be tested with `mise exec -- nix develop --command python3 -m unittest discover -s scripts -p 'test_*.py' -v`; see [`docs/BENCHMARKS.md`](docs/BENCHMARKS.md) for workload regeneration and historical measurement interpretation.

## Correctness boundary

The engine checks a formalization; it does not discover or establish business truth:

- Source references record provenance, not whether a source entails the interpretation.
- Rules participate in analysis whether marked proposed or confirmed. Review metadata is not business approval.
- Findings are relative to the declared inputs, domain, rules, constraints, and explicitly selected assumptions. Conditional conclusions must retain that scope.
- No findings do not establish model completeness, fairness, legal sufficiency, or safety. Partial models and unresolved items remain material.
- V0 accepts explicitly authored JSON snapshots; it has no text DSL, production Gleam integration, effects execution, lifecycle modeling, or Lean proof layer. It does not prove lifecycle behavior or execute business effects, and it cannot silently infer omitted requirements or external facts.

Use results as evidence for human review, not as business authorization. Fixtures and benchmarks test the engine, not the truth or appropriateness of a policy.
