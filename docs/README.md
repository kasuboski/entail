# Author decision engine documentation

This documentation describes Entail's first Rust implementation of an explicitly authored business-decision model. The engine validates a model and analyzes its proposed behavior; it does not discover requirements or execute business effects.

## Start here

- [Specification](SPEC.md): normative v0.1 language, semantics, checks, results, Rust components, worker/CLI contract, resource behavior, acceptance cases, and extension boundaries.
- [JSON-to-Z3 worked example](JSON-TO-Z3.md): follow the contractor-access fixture through compilation, check planning, symbolic queries, witness replay, and report fields; no SMT background required.
- [Worker protocol](WORKER.md): framed request/response and process lifecycle.
- [Benchmark report and methodology](BENCHMARKS.md): workloads, regeneration, and interpretation of historical measurements.
- [JSON Schema](schema/model-v0.schema.json): structural JSON contract. Schema validity does not imply successful semantic compilation.
- [Text grammar and formatting](SPEC.md#text-grammar-and-formatting): readable author syntax, canonical formatting, and CLI behavior.
- [Rust crate API](../crates/author_model/src/lib.rs): typed model/compiler/evaluator API; also see [`author_analysis`](../crates/author_analysis/src/lib.rs), [`author_z3`](../crates/author_z3/src/lib.rs), and [`author_worker`](../crates/author_worker/src/lib.rs).

## Examples and fixtures

- [`examples/models/`](examples/models/): valid JSON model examples, including incomplete drafts and models with findings; corresponding `.author` files are canonical text examples.
- [`fixtures/invalid/`](fixtures/invalid/): five JSON schema or semantic compilation failures. Text syntax, name-resolution, and semantic failures are exercised by the text frontend tests.
- [`fixtures/expectations.json`](fixtures/expectations.json): expected engine findings and diagnostic codes, not recorded solver output.
- [`examples/scenarios/`](examples/scenarios/): complete typed input assignments for concrete evaluation.
- [`examples/worker/`](examples/worker/): worker request examples. Replace the template handle with a real compile result, then encode each object with the four-byte length frame.
- [`examples/contractor-access.author`](examples/contractor-access.author): complete readable contractor-access example that is accepted by the text frontend.

The contractor source is a synthetic reference example. Complete-policy fixtures deliberately introduce proposed grant and denial behavior; they do not establish business correctness. `PolicyDenied` is a fixture-only proposed reason value, not a discovered business policy. Expiry enforcement remains a lifecycle open item. Fixtures are contract examples, not endorsements of the policies they encode.

## Deferred extensions

The text frontend is a syntax layer over the same typed model as JSON. Consider lifecycle modeling when a real method requires properties snapshots cannot establish, and aggregate operations when representative methods need them. Lean certificates require a concrete proof-assurance need. A NIF is warranted only if profiling shows transport overhead matters and scheduling/cancellation are designed; add concurrency when queue evidence warrants a bounded pool. Improve explanation minimization when counterexamples or contradiction groups impede analyst review. These are decision triggers, not commitments or changes to v0 semantics.

The schema validates structure; a model that satisfies it can still fail semantic compilation or contain unsafe business rules. Read the specification before treating any fixture or result as a business conclusion.
