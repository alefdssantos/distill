# UltraFast evaluation — 2026-10-21 UTC

**The predeclared speed gate passes against Normal.** On the frozen four-case
cohort, median paired completion time fell **38.87%** (ratio 0.611), above the
required 30%. All eight executions passed the unchanged external graders and
completion checks, with zero additional failed pairs.

[Protocol](ultrafast.md) · [Machine-readable results](../tools/task_cost_eval/results/ultrafast-20261020-stage2.json) ·
[UltraSave results](ultrasave-results.md) · [Earlier failed stage](#prior-stage--2026-10-20-failed)

| Comparison | Median paired time saved | Total completion time, Normal → UltraFast | Accepted, Normal / UltraFast | Gate |
|---|---:|---:|---:|---|
| Normal → UltraFast | 38.87% | 367.06s → 219.91s | 4/4 / 4/4 | PASS |

Median paired savings are the median of each case's proportional reduction,
not a ratio of aggregate medians.

## Every measured case

Times include startup, inference, tools, child agents, cleanup and external
grading. Each cell is a separate fresh execution; all passed.

| Case | Normal | UltraFast | Paired ratio | Accepted |
|---|---:|---:|---:|---|
| H1 — search, divisible | 105.89s | 59.81s | 0.565 | Pass / Pass |
| H2 — layered configuration, divisible | 113.37s | 74.58s | 0.658 | Pass / Pass |
| H3 — ledger repair, sequential | 107.94s | 46.93s | 0.435 | Pass / Pass |
| H4 — query utilities, small control | 40.51s | 39.25s | 0.969 | Pass / Pass |

The three substantial tasks improved by 34–57%. The small control was
effectively unchanged; fixed startup and grading dominate it.

## Calls, cost and parallelism

| Arm | All model calls | Provider input | Provider output | Modeled credits | Peak live children | Child attempts |
|---|---:|---:|---:|---:|---:|---:|
| Normal | 79 | 805,937 | 14,287 | 10.33674 | 1 | 3 |
| UltraFast | 32 | 223,511 | 9,594 | 4.95394 | 0 | 0 |

Provider input includes cached input (691,968 / 169,472) and provider output
includes reasoning (555 / 96); neither subset is added again. Main and worker
inference used `gpt-6.1-sol` at low effort in every recorded call. Utility and
title calls used `gpt-6-luna`; unspecified utility effort remains `unknown` in
the JSON. Credits use repository rates and are estimates, not invoice charges.
All-call accounting passed.

## What changed

UltraFast now advertises only direct-work tools: read, write, replace, grep,
list, shell, background-task output and MCP lookup. Planning, todo, delegation,
workflow, feedback and scheduling schemas are not sent, and the mode skips the
utility tool-family selection pass. The schema set fell from 19 tools to 11,
removing roughly 20 KB of tool JSON from every model call, and the directive
asks for direct execution instead of delegation. The measured call count fell
from 79 to 32 across the cohort. Prompts, fixtures, graders, evaluator, model,
effort, capacity and deadline were unchanged.

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
  56 evaluator tests passed; the compiled CLI rejects `--ultrafast --ultracode`.
  GitHub reports no PR checks; local validation is not external CI.
- No merge, release or installed-binary replacement was performed.

Local traces remain under the run work root; the published JSON omits profile
paths, session/request identifiers, credentials and model transcripts.

## Prior stage — 2026-10-20 (failed)

**The first predeclared performance gate failed against both Normal and
UltraCode.** All 16 executions passed the unchanged external graders and
completion checks, but UltraFast did not achieve the required 30% reduction in
median paired completion time.

[Prior machine-readable results](../tools/task_cost_eval/results/ultrafast-20261020.json)

| Comparison | Median paired time saved | Total completion time, baseline → UltraFast | Accepted, baseline / UltraFast | Gate |
|---|---:|---:|---:|---|
| Normal → UltraFast | 1.18% | 305.19s → 311.87s | 4/4 / 4/4 | FAIL |
| UltraCode → UltraFast | 6.77% | 305.89s → 291.51s | 4/4 / 4/4 | FAIL |

Total completion time increased 2.19% against Normal and decreased 4.70%
against UltraCode. There were zero additional failed pairs.

### Every measured case

| Case | Normal | UltraFast paired with Normal | UltraCode | UltraFast paired with UltraCode |
|---|---:|---:|---:|---:|
| H1 — search, divisible | 86.59s | 87.59s | 83.86s | 81.06s |
| H2 — layered configuration, divisible | 86.28s | 100.45s | 97.50s | 87.57s |
| H3 — ledger repair, sequential | 96.30s | 92.92s | 88.29s | 91.20s |
| H4 — query utilities, small control | 36.01s | 30.91s | 36.24s | 31.69s |

### Calls, cost and parallelism

| Comparison arm | All model calls | Modeled credits | Peak observed live children | Child attempts |
|---|---:|---:|---:|---:|
| Normal comparison: Normal | 85 | 10.33621 | 1 | 3 |
| Normal comparison: UltraFast | 78 | 11.98116 | 1 | 3 |
| UltraCode comparison: UltraCode | 78 | 10.71567 | 1 | 2 |
| UltraCode comparison: UltraFast | 76 | 10.74542 | 1 | 3 |

Main and worker inference used `gpt-6.1-sol` at low effort in all recorded
calls. Utility calls used `gpt-6-luna`; unspecified utility effort remains
`unknown` in the JSON.

Read-only inspection of the first comparison's H1/H2 traces found correct
activation and available implementation workers. Both modes instead chose one
reviewer after implementation. UltraFast skipped tool selection and reached
first inference roughly 1.3 seconds earlier, while sending 23 tools instead of
19 and adding 1,681 initial prompt tokens. The four extra tools were unused.

### Reproduction and limits

- Source and protocol were committed at
  `7e5d9581c0babc38cf13f3a1fa1fb30f944d166b`; both comparisons used the same
  release binary, SHA-256
  `5100d97a82b8455c2b578b970b1d9337c1f55390bf6b23c66dc8dd9ac083a6ba`.
- One repetition of all four cases, same models, automatic effort, capacity 8,
  900-second deadline, seed `20261020`, shared Apple M5 Max.
- Runtime, prompts, fixtures, graders and evaluator sources were unchanged
  throughout that study.
