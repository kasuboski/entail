# Author decision engine v0 implementation bundle

This bundle defines a first Rust implementation. It contains no engine implementation.

- `author-engine-v0-spec.md`: language semantics, checks, Rust components, protocol and acceptance criteria.
- `schema/model-v0.schema.json`: JSON Schema 2020-12 structural contract; semantic checks are additionally required.
- `fixtures/valid/`: structurally and semantically intended-valid models, including incomplete drafts and policies with analysis findings.
- `fixtures/invalid/`: schema or semantic compilation failures.
- `fixtures/expectations.json`: expected engine findings, not recorded solver output.
- `scenarios/`: complete typed input assignments for concrete evaluation.
- `protocol/`: request examples. Replace the template handle with a real compile result, then encode each object with the 4-byte length frame.
- `examples/contractor-access.author`: the stable readable reference excerpt. It is documentation, not accepted v0 input; v0 accepts JSON only. The full JSON fixtures supply enum/source declarations and additional metadata.

## Deferred extensions

Consider a text frontend only as a syntax layer over the same typed model; lifecycle modeling when a real method requires properties snapshots cannot establish; and aggregate operations when representative methods need them. Lean certificates require a concrete proof-assurance need. A NIF is warranted only if profiling shows transport overhead matters and scheduling/cancellation are designed; add concurrency when queue evidence warrants a bounded pool. Improve explanation minimization when counterexamples or contradiction groups impede analyst review.

The contractor source is a synthetic reference example. Complete-policy fixtures deliberately introduce proposed grant and denial behavior; they do not establish business correctness. `PolicyDenied` is a fixture-only proposed reason value, not a discovered business policy. Expiry enforcement remains a lifecycle open item.

Start with the specification. The schema validates structure; a model that satisfies it can still fail semantic compilation or contain unsafe business rules.
