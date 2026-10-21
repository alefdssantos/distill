# UltraSave evaluation — 2026-10-21 UTC

**The predeclared token-economy gate passes.** On the frozen four-case cohort,
median paired total token use fell **74.12%** (ratio 0.259), well below the
required 0.80 ratio. All eight executions passed the unchanged external graders
and completion checks, with zero additional failed pairs.

[Protocol](ultrasave.md) · [Machine-readable results](../tools/task_cost_eval/results/ultrasave-20261020-stage2.json) ·
[UltraFast results](ultrafast-results.md) · [Earlier failed stage](#prior-stage--2026-10-21-failed)

Across all four tasks, total tokens fell **74.86%**: 939,497 → 236,166. The
median paired ratio and the aggregate ratio differ because each task's
proportional change has equal weight in the paired metric.

## Every measured case

Tokens are provider input plus output over every physical main, worker, utility,
title and retry call. Input already includes cached input; output already
includes reasoning. Neither subset is added again. Times include startup,
inference, tools, children, cleanup and external grading. Each cell comes from a
separate fresh execution.

| Case | Normal tokens | UltraSave tokens | Token change | Normal completion | UltraSave completion | Accepted |
|---|---:|---:|---:|---:|---:|---|
| H1 — search, divisible | 274,913 | 62,648 | -77.21% | 120.76s | 64.59s | Pass / Pass |
| H2 — layered configuration, divisible | 202,709 | 58,747 | -71.02% | 100.90s | 68.93s | Pass / Pass |
| H3 — ledger repair, sequential | 396,817 | 70,873 | -82.14% | 119.72s | 54.82s | Pass / Pass |
| H4 — query utilities, small control | 65,058 | 43,898 | -32.52% | 40.91s | 42.09s | Pass / Pass |

## Usage, cost and time

| Metric | Normal | UltraSave |
|---|---:|---:|
| Provider input | 925,046 | 226,616 |
| Cached input, included above | 775,040 | 147,968 |
| Uncached input | 150,006 | 78,648 |
| Provider output | 14,451 | 9,550 |
| Reasoning, included in output | 482 | 178 |
| Total input + output | 939,497 | 236,166 |
| All model calls | 86 | 32 |
| Modeled credits | 12.52839 | 6.26128 |
| Total completion time | 382.28s | 230.43s |
| Accepted tasks | 4/4 | 4/4 |

Provider input and output include cached input and reasoning respectively.
Modeled credits use repository rates and are estimates, not invoice charges.
All-call accounting passed.

## What changed

UltraSave now advertises only direct-work tools: read, write, replace, grep,
list, shell, background-task output and MCP lookup. Planning, todo, delegation,
workflow, feedback and scheduling schemas are not sent, and the mode skips the
utility tool-family selection pass. The schema set fell from 19 tools to 11,
removing roughly 20 KB of tool JSON from every model call. The measured call
count fell from 86 to 32 across the cohort; the directive also asks for direct
execution and narrow reads. Prompts, fixtures, graders, evaluator, model,
effort, capacity and deadline were unchanged. The earlier skill-descriptor limit
and effort-reuse policy remain in place; the live cohort has an empty skill
catalog, so this study did not exercise that limit.

## Reproduction and limits

- Runtime change committed at `89c910c26997c9fa740ca84a2c42d34e53f09a40`;
  both arms used the same release binary `distill 2.0.48 (89c910c26997)`,
  SHA-256 `0d37d30d224a57f6f799122dd9fb11aca4b5e1e22aea79240b1676fbf3b1db49`.
- Main and worker model: `chatgpt/gpt-6.1-sol`; utility: `chatgpt/gpt-6-luna`;
  automatic effort; capacity 8; no depth override; 900-second deadline; seed
  `20261020`. Pairs were adjacent with alternating arm order; each arm used a
  fresh fixture repository and isolated profile.
- One repetition of all four pre-existing held-out cases on a shared Apple
  M5 Max (18 CPUs, 128 GiB), macOS 27.0.1. Other host activity and provider
  variability were not controlled. One repetition does not establish
  statistical quality equivalence or general superiority.
- All eight mode acknowledgements, complete all-call accounting audits,
  frozen-input and binary checks, setup confirmation and owned-process cleanup
  passed. No failed run was omitted or replaced.
- Runtime validation: the locked release build passed; focused Rust filters
  `ultracode_tests` (8), `tool_subset` (2) and `execution_mode` (3) passed;
  56 evaluator tests passed; the compiled CLI rejects simultaneous non-default
  mode flags. GitHub reports no PR checks; local validation is not external CI.
- No merge, release or installed-binary replacement was performed.

Local traces remain under the run work root; the published JSON omits profile
paths, session/request identifiers, credentials and model transcripts.

## Prior stage — 2026-10-21 (failed)

**The first predeclared token-economy gate failed.** Median paired total token
use increased **2.11%**, while all eight executions passed the unchanged
external graders and completion checks.

[Prior machine-readable results](../tools/task_cost_eval/results/ultrasave-20261020.json)

Across all four tasks, total tokens decreased **1.14%**: 882,666 → 872,596.

### Every measured case

| Case | Normal tokens | UltraSave tokens | Token change | Normal completion | UltraSave completion | Accepted |
|---|---:|---:|---:|---:|---:|---|
| H1 — search, divisible | 293,141 | 230,326 | -21.43% | 93.17s | 85.11s | Pass / Pass |
| H2 — layered configuration, divisible | 279,401 | 281,135 | +0.62% | 95.92s | 97.18s | Pass / Pass |
| H3 — ledger repair, sequential | 246,705 | 295,437 | +19.75% | 91.56s | 108.70s | Pass / Pass |
| H4 — query utilities, small control | 63,419 | 65,698 | +3.59% | 31.61s | 35.09s | Pass / Pass |

### Usage, cost and time

| Metric | Normal | UltraSave |
|---|---:|---:|
| Provider input | 867,883 | 858,031 |
| Cached input, included above | 711,424 | 741,760 |
| Uncached input | 156,459 | 116,271 |
| Provider output | 14,783 | 14,565 |
| Reasoning, included in output | 502 | 687 |
| Total input + output | 882,666 | 872,596 |
| All model calls | 83 | 82 |
| Modeled credits | 12.45941 | 10.28296 |
| Total completion time | 312.26s | 326.08s |
| Accepted tasks | 4/4 | 4/4 |

Modeled credits decreased 17.47% alongside 25.69% less uncached input and
greater reported cache use. The comparison does not prove that UltraSave caused
the cache difference. Total completion time increased 4.43%; speed was not the
UltraSave gate. Main and worker inference used `gpt-6.1-sol` at low effort:
72 calls in Normal and 69 in UltraSave. Utility/title calls used `gpt-6-luna`:
11 and 13 calls respectively. The eight runtime traces all recorded
`jev.skill_projection.skipped` with `empty_catalog`.

### Reproduction and limits

- Implementation and protocol were published at
  `964fcbe02015ee001673d54f7346ef119f220442`. Both arms used release binary
  `distill 2.0.9 (964fcbe02015)`, SHA-256
  `820284e915688a7c946af3fcb5b75e7643d641a9f79744e685351fb9c1eb02c1`.
- Same models, automatic effort, capacity 8, 900-second deadline, seed
  `20261020`, shared Apple M5 Max, one repetition per case.
- Runtime, prompts, fixtures, graders and evaluator sources remained unchanged
  throughout that study; no failed run was omitted or replaced.
