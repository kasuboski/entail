# Benchmarks

`scripts/benchmark.py` uses only Python's standard library. It deterministically regenerates fixtures in `benchmarks/` and checks canonical decimal-string integer literals/top-level model shape. Those checks are not schema or semantic validation: measurement runs compile the fixtures with the actual engine.

## Run

```sh
python3 scripts/benchmark.py --binary /path/to/entail --runs 5 --profile release
# Optional CLI comparisons include process start, parse/compile and analysis:
python3 scripts/benchmark.py --binary /path/to/entail --runs 5 --profile release --cli
```

Default runs cover independent thresholds at 10/100/1000, overlapping guards, same-constructor payloads, coupled Booleans, a shared derived DAG and near model limits. p50/p95 use five samples by default (nearest-rank p95); `--full` doubles requested sample count. The persistent worker uses serial four-byte big-endian length-prefixed JSON. Each family launches one worker, compiles five times, and analyzes five times. Wall compile and engine-reported `compile_elapsed_ms` are kept distinct. Analysis wall includes request transport; report elapsed and detached native stats are retained. The worker process's lifetime `VmHWM` is sampled from `/proc/<pid>/status` every 2 ms while it remains alive, covering all repetitions for that family. Missing `/proc` values remain missing, never invented. Use `--budget-ms`, `--per-obligation-ms`, `--max-obligations` for bounds (smoke defaults 1500 ms, 200 ms, 24 obligations). CLI comparisons receive only the total timeout.

Cache probes cover repeated identical analyses, semantic one-rule edits, metadata plus open-item changes, sequential edit/analyze loops, and assumption selection. Cache-hit results correctly have absent detached solver stats; the harness does not reuse stale native statistics.

## Current final worker evidence

[`benchmarks/smoke-release-native-2026-10-22.json`](../benchmarks/smoke-release-native-2026-10-22.json) records five-run release worker measurements against integration HEAD `3b37190693479e472ebe2342b6ea8ea960f2c384`. Build and measurement commands, hardware, Rust/Cargo/Z3 versions, budgets, compile timings, per-obligation detached native statistics, and sampled worker RSS are included. Environment: Linux 6.18.23 x86_64/glibc 2.44, AMD Ryzen 5 7600X (6 cores/12 threads); Rust/Cargo 1.98.0, Z3 5.1.0.0; release profile.

The native statistics are the backend-provided detached values per obligation, preserved as string-valued `statistics.values` maps. Z3 `max memory` (when present) remains under its native key/value without unit conversion. `assertion_count` is a **formula-size proxy**, not exact formula AST node count. The evidence includes it alongside the complete native stats, not reinterpreted as AST size. Engine `compile_elapsed_ms` is reported separately from Python-measured compile request wall time; it has millisecond integer precision, so small compiles legitimately report 0 ms.

Peak RSS is sampled process-wide Linux `VmHWM` in KiB while each worker remains alive; each family value spans that worker's five compile/analyze samples. The sampler polls every 2 ms, and reports only observed `/proc` data. This is distinct from Z3's own memory statistics. Same-analysis cache probes show first-call stats, then six cache hits and no detached stats on each repeat; semantic edits and assumption changes issue fresh queries, metadata changes preserve semantic hashes while refreshing content provenance.

The original [`smoke-release-2026-10-22.json`](../benchmarks/smoke-release-2026-10-22.json) and the prior CLI comparison remain preserved as historical evidence for the earlier integration revision. CLI measurements were not repeated for this follow-up; consult that earlier report for CLI results, with its source revision clearly distinguished.

## Budget interpretation and remaining gaps

Keep the engine's provisional 5000 ms total / 1000 ms per obligation / 10000 obligations defaults as starting values; the bounded smoke settings are designed for repeatable tests, not a latency SLO. Near-limit cases may correctly return partial reports; skipped work is inconclusive, never satisfied. The historical CLI 1000-rule run completed around 1.15 s p95, while its near-limit case exceeded the 1500 ms budget.

Exact formula AST node count is still not exposed; assertion count is only a proxy. Native stats and process RSS are now measured. CLI comparison was not rerun at the new integration revision. Results are smoke evidence on one host and build, not performance guarantees; expand platforms and model shapes before setting an SLO.
