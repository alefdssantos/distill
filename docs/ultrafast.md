# UltraFast

A session has one execution mode: Normal, UltraFast, UltraCode, or UltraSave.
Selecting a mode replaces the previous mode.
Use `/ultrafast on`, `/ultrafast off`, or `/ultrafast` in the interactive UI;
headless prompts accept `--ultrafast`. The CLI rejects simultaneous
execution-mode flags.

UltraFast prioritizes direct work. It advertises only direct-work tools: file
inspection and editing, search, shell and background-task output, and MCP
lookup. Planning, todo, delegation, workflow, feedback and scheduling schemas
are not sent, so they cannot spend a model round. It retains the requested model, explicit
reasoning-effort choices, permissions, task requirements, and verification
obligations. More agents alone do not establish a speed or quality improvement.

## Reproducible paired evaluation

The existing comparison runner accepts explicit `--baseline-mode` and
`--candidate-mode` values: `normal`, `ultrafast`, `ultracode`, or `ultrasave`. The historical
`--baseline-ultracode` and `--candidate-ultracode` flags remain available for
comparisons with older binaries; they cannot be combined with an explicit mode
for the same arm.

For ACP, explicit modes use `_x.ai/session/execution_mode/set` with
`{ "sessionId": "...", "mode": "ultrafast" }`. The runner requires the returned
`mode` to match before sending the task. The same acknowledgement is required
for explicit Normal and UltraCode arms. A passing fixture cannot compensate for
a missing or incorrect activation acknowledgement.

The initial evaluation is fixed before provider execution:

- Use all four cases in `cohort-ultracode-heldout-v1.json`: three substantial
  tasks and one small control, covering divisible and sequential work.
- Compare the same frozen candidate binary in Normal versus UltraFast, then
  UltraCode versus UltraFast. Run each case once in each arm: 16 model runs in
  total. Do not tune runtime behavior or graders on these results.
- Use `chatgpt/gpt-6.1-sol` for both main and task workers, the same existing
  utility tier, automatic effort in both arms, capacity eight, and a 900-second
  deadline. Effort is the same requested policy; the saved per-call ledger shows
  the actual effort applied. Leave depth overrides unset so each mode uses its
  own runtime policy.
- Keep pairs adjacent, shuffle cases with seed `20261020`, and alternate arm
  order across cases. Execute one comparison at a time to avoid competing study
  runs. Each run starts with a fresh fixture repository and isolated profile.
- Freeze and hash the binary, fixture, prompt, grader, grader helpers, and
  evaluator source. Existing graders are unchanged and run outside the model's
  workspace after its process tree has stopped.

The predeclared target is **at least 30% lower median paired completion time,
with no additional failed pairs**, separately against both Normal and
UltraCode. A quality regression means a baseline-passing pair fails in
UltraFast. Passing means satisfying the unchanged external grader and the
transport/completion checks; this bounded test cannot establish general quality
equivalence.

Completion time includes model startup, inference, tools, children, cleanup,
and external grading. A failed run is charged at least its full 900-second
deadline so an early failure cannot manufacture a speed gain. The report also
retains raw wall time for every run, including failures, median and slowest raw
time, all-call usage, actual model/effort counts, and observed active subagent
counts. Observed live children include waiting time and are not proof of
simultaneous provider inference. Main, worker, utility, title and retry usage
must pass the existing strict ledger audit; missing usage is unknown, not zero.
Modeled credits use the repository's rates and are not invoice charges.

Build and copy the candidate binary to an immutable absolute path, open the PR,
then run the commands below. The default prints the plan without provider calls;
append `--execute` for measured execution. Each comparison needs a new empty
work root. Use an existing signed-in ChatGPT OAuth profile; auth copies are
removed after every run.

```sh
python3 -B -m tools.task_cost_eval.compare_parallel \
  --baseline-binary /absolute/path/to/frozen-distill \
  --candidate-binary /absolute/path/to/frozen-distill \
  --baseline-mode normal --candidate-mode ultrafast \
  --transport acp --verdict ultrafast --max-concurrent 8 \
  --model chatgpt/gpt-6.1-sol --worker-model chatgpt/gpt-6.1-sol \
  --cohort tools/task_cost_eval/cohort-ultracode-heldout-v1.json \
  --order-seed 20261020 --repetitions 1 --timeout 900 \
  --distill-profile ~/.distill \
  --work-root /absolute/path/to/normal-vs-ultrafast-runs \
  --output /absolute/path/to/normal-vs-ultrafast.json

python3 -B -m tools.task_cost_eval.compare_parallel \
  --baseline-binary /absolute/path/to/frozen-distill \
  --candidate-binary /absolute/path/to/frozen-distill \
  --baseline-mode ultracode --candidate-mode ultrafast \
  --transport acp --verdict ultrafast --max-concurrent 8 \
  --model chatgpt/gpt-6.1-sol --worker-model chatgpt/gpt-6.1-sol \
  --cohort tools/task_cost_eval/cohort-ultracode-heldout-v1.json \
  --order-seed 20261020 --repetitions 1 --timeout 900 \
  --distill-profile ~/.distill \
  --work-root /absolute/path/to/ultracode-vs-ultrafast-runs \
  --output /absolute/path/to/ultracode-vs-ultrafast.json
```

Report both comparisons even if one misses its target. Do not omit failed runs,
replace them with retries, change the threshold after observing results, or
select only favorable cases. Any follow-up repetitions must repeat the full
cohort with the same settings and retain the initial results as a distinct
stage. One repetition is a bounded observation, not a reliable estimate of
provider variability or statistical evidence of equivalence. Binary build,
fixture staging, study orchestration, and manual review time are excluded from
the task timing and model-usage totals.

Runner regression checks use local fake peers and no model calls:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -B -m unittest \
  tools.task_cost_eval.test_acp_driver \
  tools.task_cost_eval.test_compare_parallel \
  tools.task_cost_eval.test_ultrafast_verdict
```

The [initial measured evaluation](ultrafast-results.md) passed all 16 task runs but
failed the 30% speed target against both Normal and UltraCode.
