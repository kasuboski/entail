# Benchmarks

`scripts/benchmark.py` uses only Python's standard library. It deterministically regenerates fixtures in `benchmarks/` and checks canonical decimal-string integer literals/top-level model shape. Those checks are not schema or semantic validation: end-to-end runs below compile every fixture with the actual engine.

## Run

```sh
python3 scripts/benchmark.py --binary /path/to/entail --runs 5 --profile release
# Add cold-process CLI comparisons (startup + parse/compile + analysis):
python3 scripts/benchmark.py --binary /path/to/entail --runs 5 --profile release --cli
```

Default runs cover independent thresholds at 10/100/1000, overlapping guards, same-constructor payloads, coupled Booleans, a shared derived DAG and near model limits. Each reported p50/p95 uses five samples by default (p95 is nearest-rank). `--full` runs every generated family with twice the requested sample count; repeated one-rule edits and the cache/configuration probes always run. The framed worker uses a serial four-byte big-endian length-prefixed JSON protocol, starts one process per family, compiles each measured model, then analyzes it. Compile and analyze timings are separated. CLI timings include process startup, parse/compile and analysis. `--budget-ms`, `--per-obligation-ms`, and `--max-obligations` control the explicit request budget and worker cap; default smoke settings are 1500 ms, 200 ms, and 24 obligations to bound adversarial-case runs. CLI checks receive the same total timeout (the worker's per-obligation and max-obligations controls are not expressible in the current CLI).

The harness also probes repeated identical analyses, a semantic one-rule edit, metadata plus open-item changes, one-rule edit/analyze iterations, and assumption selection. It reports semantic/content hashes and per-report cache/query/obligation/status statistics, so metadata-only reuse and assumption-key separation are checked through actual protocol responses rather than inferred from elapsed time.

## Initial release smoke results

The observed five-run release results are captured in [`benchmarks/smoke-release-2026-10-22.json`](../benchmarks/smoke-release-2026-10-22.json). They were collected against integrated source commit `57a0f158a4636c958c7f9742a3d1cf2236c655a1`, using the release `entail` executable and the build command recorded in that artifact. Environment: Linux x86_64 / glibc 2.44, AMD Ryzen 5 7600X (6 cores/12 threads), Rust 1.98.0, Cargo 1.98.0, Z3 5.1.0.0. Build profile: release. Each family has five worker and five CLI analysis samples, plus five repeated worker analyses in the cache probe.

Selected CLI p50/p95 wall times (milliseconds; process startup included):

| Family | p50 | p95 | Queries (one report) | Notes |
| --- | ---: | ---: | ---: | --- |
| Threshold rules 10 | 17.4 | 18.8 | 12 | completed |
| Threshold rules 100 | 83.4 | 83.8 | 102 | completed |
| Threshold rules 1000 | 1146.8 | 1155.4 | 1002 | completed |
| Overlapping guards | 37.9 | 41.7 | 42 | completed |
| Same-constructor payloads | 770.4 | 776.4 | 822 | 781 conflict findings; varied payload expressions |
| Coupled Booleans | 11.9 | 12.2 | 3 | completed |
| Shared derived DAG | 15.3 | 16.2 | 3 | completed |
| Near model limits (256 inputs/2048 rules) | 1774.8 | 1778.0 | 628 | 1500 ms budget; 1 obligation skipped at time limit |

Persistent-worker analysis p50/p95 was respectively 0.14/16.27, 1.30/23.27, 10.24/31.99, 0.60/23.64, 1.01/22.87, 0.11/7.33, 0.34/10.44 and 34.03/67.68 ms in the same family order. Some low-cost worker samples have large p95 outliers, so p50 alone understates observed variance. Compilation was measured separately; for example the 1000-rule model compiled at 19.35/23.64 ms p50/p95 and near-limits at 53.68/64.41 ms. These figures are smoke evidence, not guarantees. The CLI and persistent worker have different execution paths and should not be compared as equivalent latencies.

### Cache, edits and partial results

On the cache probe, the first analysis executed 6 solver queries; each of four identical repeats reported 6 cache hits and zero solver queries. A semantic one-rule edit changed the semantic hash and executed 6 fresh queries. The metadata/open-item edit preserved semantic hash, changed content hash, rebuilt the current report's content hash, and got 6 semantic cache hits. Selecting assumption `A1` executed 6 fresh queries; returning to no assumptions got 6 hits, consistent with assumptions being part of the exact cache configuration. Five successive one-rule edits each executed 6 fresh queries. See the JSON artifact for exact hashes and report metrics.

The worker smoke uses a 24-obligation cap: 100+ rule/overlap/payload and near-limit cases intentionally report a skipped obligation in this bounded mode. A skipped obligation is counted as inconclusive, never satisfied. The CLI comparison does not impose that cap. Its near-limit run reached the 1500 ms total budget after 642 completed queries and left one planned obligation skipped. Completed results remained present. Thus the bounded worker smoke validates safe partial reporting, not full analysis of those families.

### Initial budget guidance

Retain the engine's explicit provisional defaults (5000 ms total, 1000 ms per obligation, 10000 obligations) for initial interactive use rather than lowering them based on this small sample. The 1000-rule threshold case completed in about 1.15 s CLI p95 under a 1500 ms total budget, while the near-limit case exceeded that budget and produced a correct partial report. This supports keeping the defaults provisional and distinguishing input-size limits from analysis budgets; it does **not** establish a latency guarantee. Revisit with broader/adversarial models, longer runs, platform variation, and backend statistics before claiming a budget SLO.

## Gaps and honest reporting

Peak process memory was not measured. Formula size and per-obligation native solver statistics were not exposed by the backend during this run; report-level elapsed time, query count, cache hits, planned/executed obligations and statuses were available. No unavailable metric is estimated or fabricated. A later memory run can wrap worker execution in `/usr/bin/time -v`; capture its maximum RSS and exact command separately. When formula-size/backend statistics become available, the harness should include them without changing the meaning of existing measurements.
