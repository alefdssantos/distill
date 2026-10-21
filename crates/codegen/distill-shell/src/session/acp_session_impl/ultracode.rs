// Modified for Distill by Samuel Fajreldines, 2026.
//! Authoritative per-turn execution mode for `SessionActor`.
use super::*;

pub(super) const ULTRACODE_DIRECTIVE: &str = "Ultracode mode is ON. For this objective choose the smallest useful next action: execute directly when the work is understood; investigate a specific uncertainty; or decompose only when bounded smaller tasks can be verified independently. Delegation is optional in this mode, overriding default delegation preferences, with no agent quota. Keep targeted inspection, implementation and checks for a bounded known change with one execution owner; separate exploration or planning only to resolve a concrete uncertainty. Preserve useful independent parallel work and quality checks. Before delegating, state the objective, restrictions, owned paths, acceptance criteria and existing evidence references. Preserve explicit model/effort pins and capability and permission ceilings. Delegate only tasks smaller than the parent objective, within the shared depth, concurrency and budget limits. If admission is refused, execute locally or wait for independent work to finish; never recursively retry or multiply budgets. Workflow-owned, harness-internal, output-budgeted and isolated-worktree agents are leaves. Integrate child results by inspecting their evidence and actual changes, then verify the parent objective; a child report alone is not completion. Report unresolved uncertainty and concrete runtime limits.";

pub(super) const ULTRAFAST_DIRECTIVE: &str = "UltraFast mode is active. Minimize elapsed time to a verified result. For a bounded understood change, inspect, implement and verify directly; this overrides worker-first delegation preferences. Start only independent substantial tasks whose parallel execution saves more time than setup and integration. Give each task a complete goal, owned paths, constraints, acceptance criteria and checks. Launch independent tasks in the background in the same round, then immediately execute the critical remaining work yourself. Each executor owns its bounded task from inspection through implementation and checks; do not split a small task into research, planning and execution agents. Batch independent reads and tool calls. Avoid overlapping writes. Integrate completed parts while other independent tasks continue; rerun any checks invalidated by later edits. Children execute locally and never delegate. Preserve model and effort pins, permission and capability ceilings, required project checks, risk-appropriate independent review and final integrated verification. Inspect actual changes and check evidence; an agent report alone is not proof. After failure investigate its cause and change the approach, never skip the failing check or repeat blindly. Stop when acceptance criteria are verified. Keep progress and final reports concise, including unverified requirements.";

pub(super) const ULTRASAVE_DIRECTIVE: &str = "UltraSave mode is active. Minimize total input and output tokens across all calls needed for a verified result, including helpers and rework. Execute bounded understood work directly; this overrides worker-first delegation preferences. Search narrowly before reading, read relevant ranges, reuse unchanged evidence, and batch independent reads or edits where the available tools support it. Avoid repeated plans, file dumps, commentary and redundant checks. Do not repeat source text in a report when a precise path, finding and check result suffice. Delegate only when a bounded assignment is expected to save total context and output, or independent review is required by the task or its risk; include only the context the child needs. Children execute locally and never delegate. Preserve explicit model and effort choices, repository instructions, permissions, exact code and error evidence, required checks and risk-appropriate review. Never hide failures, truncate required work, or omit verification to save tokens. Use recovery tools when narrowed context is insufficient. Stop when acceptance criteria are verified and report the result and remaining uncertainty concisely.";

impl SessionActor {
    /// Every turn states the authority, including Normal after switching off.
    pub(super) fn inject_execution_mode_reminder(&self) {
        use distill_tool_types::ExecutionMode;
        let directive = match self.execution_mode.load() {
            ExecutionMode::Normal => "Normal execution mode is active. Follow the configured default execution policy.",
            ExecutionMode::Ultrafast => ULTRAFAST_DIRECTIVE,
            ExecutionMode::Ultracode => ULTRACODE_DIRECTIVE,
            ExecutionMode::Ultrasave => ULTRASAVE_DIRECTIVE,
        };
        self.push_system_reminder_with_tag(
            &format!("Current execution mode supersedes all earlier execution mode reminders. {directive}"),
            self.reminder_wrapper_tag(),
        );
    }
}
