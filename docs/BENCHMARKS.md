# Benchmarks

`python3 scripts/benchmark.py` deterministically regenerates JSON fixtures in `benchmarks/` and checks required top-level shape and canonical string encoding of every integer expression literal. Fixtures cover independent thresholds (10/100/1000), overlapping guards, same-constructor payload variation, coupled Booleans, shared derived DAGs, near declared model limits (256 inputs, 2048 rules), plus repeated-edit and metadata-change seeds. Semantic validity must still be established by compiling with the engine; generator checks are not a substitute for schema/semantic validation.

## Running measurements

The harness has no third-party Python dependencies. Once an executable is integrated:

```sh
python3 scripts/benchmark.py --binary ./target/release/author --command '{binary} validate {model}'
```

`--command` accepts a command template with `{binary}` and `{model}` placeholders. The default smoke set is modest (five families, three process invocations each); `--full` adds larger threshold/near-limit models. Use `--runs N` and `--timeout SECONDS` as needed. Wall time includes process startup and the selected CLI operation. Adapt the command to the actual CLI contract; worker framing is not yet implemented by this harness, so persistent-worker measurements remain a gap. Repeated-edit seeds are generated, but automated stateful cache probes await a stable public operation contract.

The JSON report includes p50/p95 observed wall time and failures, plus platform/machine/Python and Rust version when available. It explicitly marks unsupported metrics unavailable. Do not interpret CLI validation latency as analysis latency. Query count, formula size, cache hit/inconclusive rates, peak memory, Z3 version, and build profile are only reportable when the executable exposes or supplies them; this harness does not infer or fabricate them. Record the exact release/debug build invocation and solver version alongside archived output when running analysis. No end-to-end results have been collected yet because the integrated executable/worker contract is not available in this worktree.

A future worker mode should use the protocol's four-byte length-prefixed JSON messages, compile once, issue analyze requests repeatedly, and distinguish cold/warm, semantic edits, and metadata-only changes. Keep compilation and solve timings separate when the engine reports them. Use repeated wall-time samples, avoid claiming cache behavior from timings alone, and report timeouts/inconclusive outcomes as such.
