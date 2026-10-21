# UltraFast and UltraSave on 20 new held-out cases — 2026-10-21 UTC

**The new cohort confirms the speed and token effects but exposes one shared
quality failure.** Across 20 newly authored cases, UltraFast reduced median
paired completion time by **36.68%** on the 18 pairs with complete accounting,
and UltraSave reduced median paired tokens by **65.31%**. Both modes failed the
same input-validation check in `bench-csv`, so neither comparison earns a
passing verdict in this sample.

[UltraFast results](ultrafast-results.md) · [UltraSave results](ultrasave-results.md) ·
[UltraFast JSON](../tools/task_cost_eval/results/ultrafast-20261021-v2.json) ·
[UltraSave JSON](../tools/task_cost_eval/results/ultrasave-20261021-v2.json) ·
[Cohort](../tools/task_cost_eval/cohort-ultrafast-v2.json)

## Cohort

`cohort-ultrafast-v2.json` adds 20 independent, single-module Python tasks that
were not used in any earlier comparison. Each case ships a seeded fixture with
`NotImplementedError` stubs, a reference solution, a hidden grader and a public
smoke test. Subjects: SemVer parsing, ISO-8601 durations, interval merging,
largest-remainder money allocation, byte-size parsing, RFC 4180 CSV, POSIX path
normalization, word wrapping, stable deduplication, pagination cursors, deep
config merge, topological sort, token buckets, retry backoff, tree rendering,
sparse matrices, clock parsing, streaming statistics, TTL LRU caches and
mustache-like templates.

A case is accepted only when the external grader exits 0. The grader checks
documented edge behavior, validation, ordering, duplicate handling and input
immutability; it is never copied into the task workspace. Seeded fixtures fail
and reference solutions pass in `tools/task_cost_eval/test_benchmark_cases_v2.py`
(also asserts 20 unique cases and public tests on every reference).

## UltraFast

| Metric | Result |
|---|---:|
| Complete pairs | 18 of 20 |
| Median paired completion ratio | 0.633 |
| Median paired time saved | 36.68% |
| Quality regressions | 1 (`bench-csv`) |
| Verdict | INCOMPLETE |

Verdict reasons: baseline runs for `bench-bytes` and `bench-clock` reported
incomplete all-call ledger reconciliation after provider retries, so the
verdict refuses to compute a passing result. Both measures and all 40 runs
remain in the JSON; nothing was retried or replaced.

Excluding the two incomplete pairs, total completion time was 1552.85s
(Normal) versus 957.60s (UltraFast). With all runs, 1800.25s versus 1061.38s.

| Case | Normal | UltraFast | Ratio | Accepted |
|---|---:|---:|---:|---|
| backoff | 59.70s | 58.49s | 0.980 | Pass / Pass |
| bytes | 166.07s | 56.19s | 0.338 | Pass / Pass* |
| clock | 81.34s | 47.59s | 0.585 | Pass / Pass* |
| configmerge | 61.50s | 54.67s | 0.889 | Pass / Pass |
| csv | 121.63s | 91.49s | 0.752 | Pass / Fail |
| cursor | 109.65s | 69.20s | 0.631 | Pass / Pass |
| dedupe | 66.51s | 36.51s | 0.549 | Pass / Pass |
| duration | 114.69s | 58.09s | 0.507 | Pass / Pass |
| intervals | 133.74s | 57.20s | 0.428 | Pass / Pass |
| lru | 92.81s | 52.15s | 0.562 | Pass / Pass |
| matrix | 88.16s | 36.14s | 0.410 | Pass / Pass |
| money | 57.37s | 47.34s | 0.825 | Pass / Pass |
| paths | 48.21s | 37.75s | 0.783 | Pass / Pass |
| semver | 102.16s | 50.07s | 0.490 | Pass / Pass |
| stats | 54.89s | 36.93s | 0.673 | Pass / Pass |
| template | 105.83s | 68.26s | 0.645 | Pass / Pass |
| tokenbucket | 99.64s | 62.36s | 0.626 | Pass / Pass |
| toposort | 94.93s | 50.57s | 0.533 | Pass / Pass |
| tree | 86.55s | 55.52s | 0.642 | Pass / Pass |
| wrap | 54.90s | 34.87s | 0.635 | Pass / Pass |

\* Baseline ledger reconciliation incomplete; excluded from the paired median.

## UltraSave

| Metric | Result |
|---|---:|
| Pairs | 20 of 20 |
| Median paired token ratio | 0.347 |
| Median paired tokens saved | 65.31% |
| Median on accepted pairs only | 0.352 |
| Quality regressions | 1 (`bench-csv`) |
| Verdict | FAIL |

Total tokens were 3,161,042 (Normal) versus 1,064,161 (UltraSave); model calls
were 340 versus 160. Per-case ratios ran from 0.161 (configmerge) to 0.705
(template). The single regression sets its gate ratio to 1.0, as the policy
requires, but the median across accepted pairs is unchanged at 0.352.

## The failing check

Both candidates failed the same grader assertion:

```
non-iterable rows: raised TypeError: 'int' object is not iterable
```

`select_rows(42, ['id'], {})` is expected to raise `ValueError`. The prompt
states that `rows` is an iterable, and separately that invalid row entries
raise `ValueError`, but never names a non-iterable `rows` value explicitly.
Both modes produced correct dictionaries for all iterable inputs and failed
only on this unspecified boundary. The failure is reported as measured; the
prompt was not rewritten after seeing it.

## Protocol

- Binary: `distill 2.0.48 (e6093a18dacd)`, SHA-256
  `4f771b7224e1968fb18da385ccffc1d1fa891d8dd6790d2f997cb447d87d9ac1`,
  built from the runtime commit under test. Both arms used the same binary.
- Model `chatgpt/gpt-6.1-sol` for main and worker, automatic effort, ACP
  transport, capacity 8, 900-second deadline, order seed `20261021`, one
  repetition, adjacent pairs with alternating arm order, fresh fixture
  repository and isolated profile per run.
- Commands, run from the repository root, with a new empty work root each:

```sh
python3 -B -m tools.task_cost_eval.compare_parallel \
  --baseline-binary /tmp/ultrafast-v2-bin/distill \
  --candidate-binary /tmp/ultrafast-v2-bin/distill \
  --baseline-mode normal --candidate-mode ultrafast \
  --transport acp --verdict ultrafast --max-concurrent 8 \
  --model chatgpt/gpt-6.1-sol --worker-model chatgpt/gpt-6.1-sol \
  --cohort tools/task_cost_eval/cohort-ultrafast-v2.json \
  --order-seed 20261021 --repetitions 1 --timeout 900 \
  --distill-profile ~/.distill \
  --work-root /tmp/ultrafast-v2-runs/fast \
  --output /tmp/ultrafast-v2-runs/fast.json --execute

python3 -B -m tools.task_cost_eval.compare_parallel \
  --baseline-binary /tmp/ultrafast-v2-bin/distill \
  --candidate-binary /tmp/ultrafast-v2-bin/distill \
  --baseline-mode normal --candidate-mode ultrasave \
  --transport acp --verdict ultrasave --max-concurrent 8 \
  --model chatgpt/gpt-6.1-sol --worker-model chatgpt/gpt-6.1-sol \
  --cohort tools/task_cost_eval/cohort-ultrafast-v2.json \
  --order-seed 20261021 --repetitions 1 --timeout 900 \
  --distill-profile ~/.distill \
  --work-root /tmp/ultrafast-v2-runs/save \
  --output /tmp/ultrafast-v2-runs/save.json --execute
```

Timing includes startup, inference, tools, children, cleanup and external
grading. One repetition per case, shared Apple M5 Max (18 CPUs, 128 GiB),
macOS 27.0.1; other host activity and provider variability were not
controlled. This sample does not establish statistical equivalence or general
superiority. All mode acknowledgements, binary and frozen-input checks and
cleanup passed; the two accounting gaps and the one quality regression are the
complete set of exceptions, and no failed run was removed or replaced.
