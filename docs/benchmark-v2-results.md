# UltraFast and UltraSave on 20 held-out cases — 2026-10-21 UTC

**Both execution modes pass the predeclared gates on the 20-case cohort.**
UltraFast reduced median paired completion time by **34.92%** and UltraSave
reduced median paired provider tokens by **70.45%**, with every run accepted,
zero quality regressions and complete all-call accounting.

[UltraFast results](ultrafast-results.md) · [UltraSave results](ultrasave-results.md) ·
[UltraFast JSON](../tools/task_cost_eval/results/ultrafast-20261021-v2.json) ·
[UltraSave JSON](../tools/task_cost_eval/results/ultrasave-20261021-v2.json) ·
[Cohort](../tools/task_cost_eval/cohort-ultrafast-v2.json)

## Cohort

`cohort-ultrafast-v2.json` holds 20 independent, single-module Python tasks
that were not used in any earlier comparison. Each case ships a seeded fixture
with `NotImplementedError` stubs, a reference solution, a hidden grader and a
public smoke test. Subjects: SemVer parsing, ISO-8601 durations, interval
merging, largest-remainder money allocation, byte-size parsing, RFC 4180 CSV,
POSIX path normalization, word wrapping, stable deduplication, pagination
cursors, deep config merge, topological sort, token buckets, retry backoff,
tree rendering, sparse matrices, clock parsing, streaming statistics, TTL LRU
caches and mustache-like templates.

A case is accepted only when the external grader exits 0. The grader checks
documented edge behavior, validation, ordering, duplicate handling and input
immutability; it is never copied into the task workspace. Seeded fixtures fail
and reference solutions pass in `tools/task_cost_eval/test_benchmark_cases_v2.py`,
which also asserts 20 unique cases and runs the public tests on every reference.

## UltraFast

| Metric | Result |
|---|---:|
| Pairs | 20 of 20 |
| Median paired completion ratio | 0.651 |
| Median paired time saved | 34.92% |
| Quality regressions | 0 |
| All-call accounting complete | 20 of 20 runs |
| Verdict | **PASS** |

Total measured time was 1506.70s (Normal) versus 1057.20s (UltraFast), including
grader time; provider credits were 39.88 versus 21.90 and physical model calls
331 versus 151.

| Case | Normal | UltraFast | Ratio |
|---|---:|---:|---:|
| backoff | 80.16s | 43.71s | 0.545 |
| bytes | 79.16s | 52.69s | 0.666 |
| clock | 46.14s | 41.86s | 0.907 |
| configmerge | 47.92s | 48.70s | 1.016 |
| csv | 85.30s | 51.97s | 0.609 |
| cursor | 96.00s | 52.88s | 0.551 |
| dedupe | 42.79s | 41.50s | 0.970 |
| duration | 116.62s | 59.76s | 0.512 |
| intervals | 103.49s | 55.89s | 0.540 |
| lru | 94.18s | 59.85s | 0.636 |
| matrix | 96.75s | 55.76s | 0.576 |
| money | 46.11s | 42.65s | 0.925 |
| paths | 45.77s | 37.36s | 0.816 |
| semver | 71.37s | 45.39s | 0.636 |
| stats | 47.33s | 42.41s | 0.896 |
| template | 71.76s | 57.89s | 0.807 |
| tokenbucket | 127.35s | 50.05s | 0.393 |
| toposort | 79.76s | 58.88s | 0.738 |
| tree | 82.51s | 50.15s | 0.608 |
| wrap | 46.24s | 107.87s | 2.333 |

`wrap` regressed in this sample: the UltraFast run took 2.33x as long as its
Normal pair, which is why the median, not the mean, carries the gate. The other
19 cases are at or below 1.02.

## UltraSave

| Metric | Result |
|---|---:|
| Pairs | 20 of 20 |
| Median paired token ratio | 0.296 |
| Median paired tokens saved | 70.45% |
| Quality regressions | 0 |
| All-call accounting complete | 20 of 20 runs |
| Verdict | **PASS** |

Total tokens were 3,243,789 (Normal) versus 992,669 (UltraSave); provider
credits were 40.01 versus 19.54 and physical model calls 343 versus 151.
Per-case ratios ran from 0.174 (template) to 0.711 (backoff).

| Case | Normal | UltraSave | Ratio |
|---|---:|---:|---:|
| backoff | 87,046 | 61,857 | 0.711 |
| bytes | 237,438 | 46,187 | 0.195 |
| clock | 176,664 | 41,781 | 0.236 |
| configmerge | 83,987 | 43,301 | 0.516 |
| csv | 188,881 | 44,846 | 0.237 |
| cursor | 214,457 | 59,469 | 0.277 |
| dedupe | 99,056 | 43,146 | 0.436 |
| duration | 300,149 | 73,010 | 0.243 |
| intervals | 249,582 | 53,559 | 0.215 |
| lru | 181,035 | 45,411 | 0.251 |
| matrix | 82,277 | 47,903 | 0.582 |
| money | 133,641 | 44,185 | 0.331 |
| paths | 86,660 | 41,265 | 0.476 |
| semver | 188,465 | 54,098 | 0.287 |
| stats | 72,838 | 48,396 | 0.664 |
| template | 251,252 | 43,665 | 0.174 |
| tokenbucket | 83,086 | 53,906 | 0.649 |
| toposort | 200,771 | 61,042 | 0.304 |
| tree | 195,972 | 42,701 | 0.218 |
| wrap | 130,532 | 42,941 | 0.329 |

## Accounting reconciliation

Provider overload occasionally makes the sampler retry a request. A retry that
fails before any channel emits output cannot have produced billable output
tokens, but its usage is unknown to the ledger, which conservatively marks the
session incomplete.

The runtime now records `output_started` on the
`shell.turn.inference_retry` log event, taken from the per-request stream
ownership. The benchmark reconciliation reads that evidence and treats such an
attempt as a known zero-usage call only when all of the following hold:

- the attempt is a `main_retry` with no provider usage in the ledger;
- an `inference_retry` log line carries the same `sampler_request_id`, kind and
  attempt number with `output_started: false`;
- every other call in the run is complete with a finite credit value;
- the ledger reports no pending attempts.

A retry that had already started streaming keeps its unknown usage and the run
stays incomplete. The 0.70 and 0.80 gate thresholds and the zero-regression
rule are unchanged; no run was dropped or replaced. Both new result files carry
`accounting_complete: true` and `accounting_evidence.complete: true` on all 80
runs, and every `call_usage` entry is complete with token fields that sum to
the run totals.

## The `bench-csv` contract

The earlier sample failed a shared grader check because the prompt said
`rows` is an iterable and that invalid row entries raise `ValueError`, but never
stated what a non-iterable `rows` value does. The prompt now says explicitly
that a non-iterable `rows` value raises `ValueError`, matching the grader and
the reference solution. The assertion itself is unchanged, the seeded fixture
still fails and both modes now pass the case.

## Protocol

- Binary: `distill 2.0.48 (39626e621550)`, SHA-256
  `311aa045e4035c5145b907bb6fd813b45dac9d383469283901b91a34d4b3c7db`,
  built from commit `39626e62155098c84516c0b50f4a13e5b4e0fd78` with
  `CARGO_PROFILE_RELEASE_DEBUG=0 GROK_VERSION=2.0.48 cargo build --locked
  --release -p distill-pager-bin --bin distill`. Both arms used this binary.
- Model `chatgpt/gpt-6.1-sol` for main and worker, automatic effort, ACP
  transport, capacity 8, 900-second deadline, order seed `20261021`, one
  repetition, adjacent pairs with alternating arm order, fresh fixture
  repository and isolated profile per run.
- Commands, run from the repository root, with a new empty work root each and
  the frozen binary copied to `<work>/bin/distill`:

```sh
python3 -B -m tools.task_cost_eval.compare_parallel \
  --baseline-binary <work>/bin/distill \
  --candidate-binary <work>/bin/distill \
  --baseline-mode normal --candidate-mode ultrafast \
  --transport acp --verdict ultrafast --max-concurrent 8 \
  --model chatgpt/gpt-6.1-sol --worker-model chatgpt/gpt-6.1-sol \
  --cohort tools/task_cost_eval/cohort-ultrafast-v2.json \
  --order-seed 20261021 --repetitions 1 --timeout 900 \
  --distill-profile ~/.distill \
  --work-root <work>/runs/fast \
  --output <work>/fast.json --execute

python3 -B -m tools.task_cost_eval.compare_parallel \
  --baseline-binary <work>/bin/distill \
  --candidate-binary <work>/bin/distill \
  --baseline-mode normal --candidate-mode ultrasave \
  --transport acp --verdict ultrasave --max-concurrent 8 \
  --model chatgpt/gpt-6.1-sol --worker-model chatgpt/gpt-6.1-sol \
  --cohort tools/task_cost_eval/cohort-ultrafast-v2.json \
  --order-seed 20261021 --repetitions 1 --timeout 900 \
  --distill-profile ~/.distill \
  --work-root <work>/runs/save \
  --output <work>/save.json --execute
```

Timing includes startup, inference, tools, children, cleanup and external
grading. One repetition per case, shared Apple M5 Max (18 CPUs, 128 GiB),
macOS 27.0.1; other host activity and provider variability were not controlled.
This sample does not establish statistical equivalence or general superiority.
All mode acknowledgements, binary and frozen-input checks, cleanup checks and
all-call accounting passed; nothing failed and no run was removed or replaced.
