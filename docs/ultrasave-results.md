# UltraSave evaluation — 2026-10-21 UTC

**The predeclared token-economy gate failed.** Median paired total token use increased **2.11%**, while all eight executions passed the unchanged external graders and completion checks. The required reduction was at least 20%, with no additional failed pairs. These results do not establish a useful token reduction for UltraSave. The PR remains a draft.

[Protocol](ultrasave.md) · [Machine-readable results](../tools/task_cost_eval/results/ultrasave-20261020.json) · [Separate UltraFast results](ultrafast-results.md)

Across all four tasks, total tokens decreased **1.14%**: 882,666 → 872,596. The median paired result and aggregate result differ because each task's proportional change has equal weight in the paired metric. There were zero additional failed pairs; one repetition does not establish statistical quality equivalence.

## Every measured case

Tokens are provider input plus output over every physical main, worker, utility, title and retry call. Input already includes cached input; output already includes reasoning. Neither subset is added again. Times include startup, inference, tools, children, cleanup and external grading. Each cell comes from a separate fresh execution.

| Case | Normal tokens | UltraSave tokens | Token change | Normal completion | UltraSave completion | Accepted |
|---|---:|---:|---:|---:|---:|---|
| H1 — search, divisible | 293,141 | 230,326 | -21.43% | 93.17s | 85.11s | Pass / Pass |
| H2 — layered configuration, divisible | 279,401 | 281,135 | +0.62% | 95.92s | 97.18s | Pass / Pass |
| H3 — ledger repair, sequential | 246,705 | 295,437 | +19.75% | 91.56s | 108.70s | Pass / Pass |
| H4 — query utilities, small control | 63,419 | 65,698 | +3.59% | 31.61s | 35.09s | Pass / Pass |

## Usage, cost and time

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

Modeled credits decreased **17.47%**, alongside **25.69% less uncached input** and greater reported cache use. This is distinct from total-token savings. Credits use repository rates and are estimates, not invoice charges. The comparison does not prove that UltraSave caused the cache difference. Total completion time increased **4.43%**; speed was not the UltraSave gate.

Main and worker inference used `gpt-6.1-sol` at low effort in all recorded calls: 72 calls in Normal and 69 in UltraSave. Utility/title calls used `gpt-6-luna`: 11 and 13 calls respectively. Unspecified utility effort is retained as `unknown` in the JSON. Both arms made three child attempts across the cohort, with peak observed live children of one; live children include waiting and are not proof of simultaneous provider inference.

The overall call count barely changed: 83 → 82. The smaller Sol call count was partly offset by two additional Luna calls. The eight runtime traces all recorded `jev.skill_projection.skipped` with `empty_catalog`, so this study did **not exercise the four-versus-eight skill-descriptor limit**. Its preservation of explicit/active skills and complete recovery is covered by focused runtime tests, not by these live measurements. Independent inspection found no concrete mode-activation, routing or accounting defect that would justify changing the runtime based on these fixtures.

## Reproduction and limits

- Implementation and protocol were published in PR #39 before provider execution, at `964fcbe02015ee001673d54f7346ef119f220442`. Both arms used release binary `distill 2.0.9 (964fcbe02015)`, SHA-256 `820284e915688a7c946af3fcb5b75e7643d641a9f79744e685351fb9c1eb02c1`. This is a new binary with a fresh Normal baseline, separate from the earlier UltraFast study.
- Main and worker model: `chatgpt/gpt-6.1-sol`; utility: `chatgpt/gpt-6-luna`; automatic effort; capacity 8; no depth override; 900-second deadline; seed `20261020`. Pairs were adjacent with alternating arm order. Each arm used a fresh fixture repository and isolated profile on the same account.
- One repetition of all four pre-existing Python cases, on a shared Apple M5 Max (18 CPUs, 128 GiB), macOS 27.0.1. This cohort was already used for UltraFast; it is not fresh unseen confirmation for UltraSave. Other host activity and provider variability were not controlled. Build, fixture staging and study orchestration time were excluded.
- Runtime, prompts, fixtures, graders and evaluator sources remained unchanged throughout the study. All eight mode acknowledgements, complete call-accounting audits, frozen-hash checks and owned-process cleanup checks passed. Temporary OAuth copies were removed. No failed run was omitted or replaced, and no runtime tuning followed these results.
- Runtime validation: **55 distinct focused Rust tests and 56 evaluator tests passed**, as did the locked release build. The compiled CLI rejects every pair of simultaneous non-default mode flags. Automated tests cover all four modes, lifecycle persistence, footer, Models controls, activation ordering, explicit model/effort preservation, child limits and full skill-catalog recovery.
- Live TUI inspection remains unverified because automation target validation rejected the window action. GitHub reports no PR checks; local validation is not external CI. No merge, release or installed-binary replacement was performed.

Complete local traces and accounting reports are retained under `/tmp/distill-ultrasave-evaluation-20261020/`. The published JSON omits profile paths, session/request identifiers, credentials and model transcripts, and includes the SHA-256 of the full local report.
