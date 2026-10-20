# UltraFast evaluation — 2026-10-20

**The predeclared performance gate failed against both Normal and UltraCode.** All 16 executions passed the unchanged external graders and completion checks, but UltraFast did not achieve the required 30% reduction in median paired completion time. This result does not support a claim that UltraFast is substantially faster. The PR remains a draft.

[Protocol](ultrafast.md) · [Machine-readable results](../tools/task_cost_eval/results/ultrafast-20261020.json)

| Comparison | Median paired time saved | Total completion time, baseline → UltraFast | Accepted, baseline / UltraFast | Gate |
|---|---:|---:|---:|---|
| Normal → UltraFast | 1.18% | 305.19s → 311.87s | 4/4 / 4/4 | FAIL |
| UltraCode → UltraFast | 6.77% | 305.89s → 291.51s | 4/4 / 4/4 | FAIL |

Total completion time increased 2.19% against Normal and decreased 4.70% against UltraCode. Median paired savings are the median of each case’s proportional reduction, rather than a ratio of aggregate medians. There were zero additional failed pairs.

## Every measured case

Times below include startup, inference, tools, child agents, cleanup and external grading. Every cell represents a separate fresh execution; all passed.

| Case | Normal | UltraFast paired with Normal | UltraCode | UltraFast paired with UltraCode |
|---|---:|---:|---:|---:|
| H1 — search, divisible | 86.59s | 87.59s | 83.86s | 81.06s |
| H2 — layered configuration, divisible | 86.28s | 100.45s | 97.50s | 87.57s |
| H3 — ledger repair, sequential | 96.30s | 92.92s | 88.29s | 91.20s |
| H4 — query utilities, small control | 36.01s | 30.91s | 36.24s | 31.69s |

## Calls, cost and parallelism

| Comparison arm | All model calls | Modeled credits | Peak observed live children | Child attempts |
|---|---:|---:|---:|---:|
| Normal comparison: Normal | 85 | 10.33621 | 1 | 3 |
| Normal comparison: UltraFast | 78 | 11.98116 | 1 | 3 |
| UltraCode comparison: UltraCode | 78 | 10.71567 | 1 | 2 |
| UltraCode comparison: UltraFast | 76 | 10.74542 | 1 | 3 |

Main and worker inference used `gpt-6.1-sol` at low effort in all recorded calls. Utility calls used `gpt-6-luna`; unspecified utility effort remains `unknown` in the JSON. Credits use repository rates and are estimates, not invoice charges. All-call accounting passed, including main, worker and utility usage. Observed live children include waiting and do not prove simultaneous inference. This cohort did not demonstrate increased implementation parallelism.

Read-only inspection of the first comparison’s H1/H2 traces found correct activation and available implementation workers. Both modes instead chose one reviewer after implementation. H2’s UltraFast reviewer found a real CLI defect; the parent corrected and retested it. UltraFast skipped tool selection and reached first inference roughly 1.3 seconds earlier, while sending 23 tools instead of 19 and adding 1,681 initial prompt tokens. The four extra tools were unused. This confirms a context-cost tradeoff; it does not establish the cause of provider latency differences. No runtime defect was identified that would justify changing the implementation based on these fixtures.

## Reproduction and limits

- Source and protocol were committed at `7e5d9581c0babc38cf13f3a1fa1fb30f944d166b`; PR #39 was opened before provider execution. Both comparisons used the same release binary, SHA-256 `5100d97a82b8455c2b578b970b1d9337c1f55390bf6b23c66dc8dd9ac083a6ba`.
- Main and worker model: `chatgpt/gpt-6.1-sol`; utility: `chatgpt/gpt-6-luna`; automatic effort; capacity 8; no depth override; 900-second deadline; seed `20261020`. Pairs were adjacent with alternating arm order, and comparisons ran sequentially. Each arm used a fresh fixture repository and isolated profile.
- One repetition of all four pre-existing held-out cases per comparison, on a shared Apple M5 Max (18 CPUs, 128 GiB), macOS 27.0.1. Local build time was excluded. Other host activity and provider variability were not controlled. One repetition cannot establish statistical quality equivalence or general superiority.
- Runtime, prompts, fixtures, graders and evaluator sources were unchanged throughout the study. Frozen hashes, per-run raw and completion times, median and slowest times, tokens, call counts and actual efforts are retained in the JSON. No failed run was omitted or replaced. All mode acknowledgements, accounting audits and owned-process cleanup checks passed; temporary OAuth copies were removed.
- Runtime validation: 54 distinct focused Rust tests and 52 evaluator tests passed; the release build passed. The compiled CLI rejects simultaneous `--ultrafast --ultracode`. Live TUI inspection was blocked by automation target validation (`stale_state_token`); mode switching, footer rendering and Models controls passed automated tests. GitHub reports no PR checks; local tests are not external CI.
- Explicit model/effort choices, required review, permissions and acceptance requirements were preserved. No merge, release or installed-binary replacement was performed.

Complete local traces and strict accounting reports are retained under `/tmp/distill-ultrafast-evaluation-20261020/`. The published JSON omits profile paths, session/request identifiers, credentials and model transcripts, and records the SHA-256 of each full local report.
