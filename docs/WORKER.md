# Entail worker integration contract

`entail worker` is a persistent, serial JSON process intended to be supervised by a Gleam owner. Rust owns model compilation, concrete evaluation and solver-backed analysis. The client owns durable model JSON, request ordering, cancellation policy and restart recovery. Worker handles are process-local capabilities, not durable identifiers.

## Transport-independent contract

Each request is one JSON object with exactly these fields:

```json
{"protocol_version":"author-worker/0.1","request_id":"r1","op":"hello","params":{}}
```

`protocol_version` must equal `author-worker/0.1`; `request_id` is a nonempty string; `params` is an object. Success is `{"protocol_version":"author-worker/0.1","request_id":"r1","result":{...}}`. Failure is `{"protocol_version":"author-worker/0.1","request_id":"r1","error":{"code":"...","message":"..."}}` instead of `result`. Requests are processed in order, one at a time. A client must correlate replies by request ID and must not pipeline assumptions about concurrent execution.

Operations:

- `hello` accepts `{}` and returns language/protocol/engine/solver versions, supported checks and configured limits.
- `compile` accepts `{"model": <author-model/0.1 JSON>}`. A valid model returns an opaque handle, content/semantic hashes and readiness (`ready_for_analysis` or `incomplete_model`). Invalid models are a successful protocol response containing diagnostics and no handle; callers should repair and compile again.
- `analyze` accepts `handle`, optional `checks` (an array of supported names), optional `assumption_ids`, and optional `budget` (`total_timeout_ms`, `per_obligation_timeout_ms`, `max_obligations`, optional nonzero `solver_resource_limit`). Feasibility is mandatory; an empty check list requests no additional checks. Options and assumptions are deterministic, duplicate check names are removed, and requested budgets are clamped to worker limits. Findings and incomplete/inconclusive results are normal responses.
- `evaluate` accepts `handle`, a complete `inputs` object, optional `assumption_ids`, and optional `expected_outcome`. No solver is used.
- `release` accepts `handle` and reports whether it existed. Capacity is never recovered by silently evicting another live model.

Stable protocol failures include `invalid_envelope`, `version_mismatch`, `unknown_operation`, `unknown_handle`, `capacity_exceeded`, `invalid_params`, and `internal_error`. Unknown/expired handles are client state errors. Malformed JSON frames return `malformed_json`; if a `request_id` can be recovered it is echoed, otherwise it is `null`. The worker continues after a complete malformed JSON payload. An invalid or oversized frame length is fatal because the stream cannot safely be resynchronized.

## Framing and process IO

Standard input/output use 4-byte unsigned big-endian payload lengths followed by exactly that many UTF-8 JSON bytes. The default payload cap is 8 MiB. Reads and writes must tolerate partial OS pipe operations. A clean EOF between frames ends normally; EOF inside a header or payload is a protocol termination. Standard output is reserved exclusively for frames. Diagnostics and operational logs belong on standard error. Configurable frame, handle, budget and model limits are exposed through worker flags/environment: `--max-frame-bytes` / `ENTAIL_MAX_FRAME_BYTES`, `--max-handles` / `ENTAIL_MAX_HANDLES`, `--max-budget-ms` / `ENTAIL_MAX_BUDGET_MS`, `--max-obligations` / `ENTAIL_MAX_OBLIGATIONS`, and `--max-inputs`, `--max-rules`, `--max-constraints`, `--max-expression-nodes`, `--max-expression-depth` (matching `ENTAIL_MAX_*` names). `hello` reports effective values.

The production client can implement this contract in any typed language without depending on Rust types, solver objects, pointer values or internal crate APIs. A JSON decoder should reject malformed response envelopes and surface the stable `error.code`; it should preserve the request ID and avoid interpreting findings as transport failures.

## Supervision, deadlines and restart

There is no in-band cancellation operation in v0. A deadline expiry means the owner must terminate the OS process, wait for its termination, and only then start the replacement. Closing a port/pipe alone is not evidence that the process or native solver has stopped. In particular, always perform the equivalent of `Port.close` **and** an OS process wait/reap; do not launch replacement workers while the old process may still be solving.

After a worker exits, discard every handle. Recompile retained model JSON in the new process before analysis/evaluation; do not persist handles or attempt to reconstruct them. Each process uses a fresh session nonce in handles so stale IDs are not accidentally accepted after restart. If termination or reaping fails, keep the worker slot unavailable and surface a supervision error rather than creating an unbounded orphan process.

One worker is serial. Parallel analyses require separate supervised processes and independent handles. A process kill is the hard-cancellation mechanism and may discard the in-flight response; completed responses before the killed request remain the caller's durable responsibility.
