// Modified for Distill by Samuel Fajreldines, 2026.
//! B8 (Auto execution mode): with the session in Auto, one Jev decision picks
//! the effective mode (Normal, UltraFast, UltraCode, UltraSave) when a request
//! starts, and round boundaries with a concrete signal may move it up or down.
//! Every pick reaches clients as `x.ai/session/execution_mode/changed`.

use distill_tool_types::ExecutionMode;
use distill_workspace::jev::flags::JevLever;

use super::*;

pub(super) const MODE_QUESTION: &str = "execution_mode";
pub(super) const MODE_CHANGED_METHOD: &str = "x.ai/session/execution_mode/changed";

/// Mid-request switches allowed per request; each one changes the tool list
/// and directive, which costs a cache miss.
const MAX_SWITCHES_PER_REQUEST: u8 = 2;
/// Completed tool rounds between reviews when nothing failed.
const REVIEW_EVERY_ROUNDS: usize = 8;
/// New failed tool calls since the last decision that prompt an early review.
const FAILURES_FOR_REVIEW: u64 = 2;
/// A mid-request switch needs this much of Jev's probability on the new mode.
const SWITCH_PROBABILITY: f64 = 0.6;
const REQUEST_CHARS: usize = 4_000;

/// Auto's per-request bookkeeping.
#[derive(Debug, Default, Clone)]
pub(crate) struct AutoModeTurn {
    switches: u8,
    failures_at_decision: u64,
    round_at_decision: usize,
}

const MODES: [ExecutionMode; 4] = [
    ExecutionMode::Ultrasave,
    ExecutionMode::Ultrafast,
    ExecutionMode::Normal,
    ExecutionMode::Ultracode,
];

fn mode_id(mode: ExecutionMode) -> &'static str {
    match mode {
        ExecutionMode::Normal => "normal",
        ExecutionMode::Ultrafast => "ultrafast",
        ExecutionMode::Ultracode => "ultracode",
        ExecutionMode::Ultrasave => "ultrasave",
    }
}

fn mode_criterion(mode: ExecutionMode) -> &'static str {
    match mode {
        ExecutionMode::Ultrasave => "UltraSave: the main model works directly with few tokens; tier routing may move rounds to the cheaper worker/utility model. Best for clear, bounded or mechanical work (one-file fixes, known edits, questions answerable from a few reads) where a cheaper model still gets it right.",
        ExecutionMode::Ultrafast => "UltraFast: minimum wall time; one round of parallel subagents on the worker model, tier routing toward the fastest capable model. Best for clear work with several independent parts, or when speed matters more than a small extra cost.",
        ExecutionMode::Normal => "Normal: default policy on the main model with full tools. Best for ambiguous, risky or judgment-heavy work (unexplained failures, design decisions, security, subtle correctness) where a cheaper or faster mode would likely lose quality or need rework.",
        ExecutionMode::Ultracode => "UltraCode: orchestrates large work across parallel subagents with deeper delegation. Best only for large multi-part work (many files or components, broad refactors, multi-step features) whose independent parts can be verified separately; overhead is wasted on small tasks.",
    }
}

fn mode_question() -> Option<std::collections::BTreeMap<String, distill_workspace::jev::Question>> {
    let criteria = MODES
        .iter()
        .map(|mode| {
            (
                mode_id(*mode).to_owned(),
                serde_json::Value::String(mode_criterion(*mode).to_owned()),
            )
        })
        .collect();
    let question = distill_workspace::jev::Question::choice(
        "Which execution mode is most efficient for the request in `request` given `progress`? \
         Efficiency means the best quality at the lowest money cost in the least time: never \
         trade a correct, verified result for savings, and count retries and rework as cost \
         and time. Prefer the cheapest and fastest mode that will still deliver the result \
         right the first time. When `current_mode` is set, switching changes the directive \
         and tool list mid-request; choose a different mode only when the evidence in \
         `progress` clearly shows the current one is the wrong fit.",
        criteria,
    )
    .ok()?;
    Some([(MODE_QUESTION.to_owned(), question)].into())
}

fn picked_mode(answers: &distill_workspace::jev::JevAnswerSet) -> Option<(ExecutionMode, f64)> {
    let choice = answers.choice(MODE_QUESTION)?;
    let mode = MODES.into_iter().find(|mode| mode_id(*mode) == choice)?;
    let probability = answers
        .probability(MODE_QUESTION, choice)
        .or_else(|| answers.confidence(MODE_QUESTION))
        .unwrap_or(0.0);
    Some((mode, probability))
}

fn bounded(text: &str) -> String {
    if text.chars().count() <= REQUEST_CHARS {
        return text.to_owned();
    }
    let mut out: String = text.chars().take(REQUEST_CHARS).collect();
    out.push_str(" […]");
    out
}

impl SessionActor {
    fn auto_mode_active(&self) -> bool {
        !self.startup_hints.is_subagent
            && self.tool_context.subagent_depth == 0
            && self.execution_mode.is_auto()
    }

    fn auto_mode_state(&self, request: &str, current: Option<ExecutionMode>, rounds: usize) -> serde_json::Value {
        let ledger = self.jev_ledger.borrow();
        let failed_checks = ledger
            .facts
            .recorded_checks()
            .iter()
            .filter(|check| check.failed)
            .count();
        serde_json::json!({
            "request": bounded(request),
            "current_mode": current.map(mode_id),
            "progress": {
                "completed_tool_rounds": rounds,
                "failed_tool_calls": ledger.facts.failed_tool_count(),
                "checks_run": ledger.facts.recorded_checks().len(),
                "failed_checks": failed_checks,
            },
            "worker_model_configured": self.tier_worker_slug().is_some(),
        })
    }

    /// Applies Auto's pick and tells every attached client.
    fn publish_auto_mode(&self, mode: ExecutionMode, reason: &str) {
        self.execution_mode.store_auto(mode);
        if let Err(error) = crate::extensions::session_ultracode::persist_mode(
            &self.session_info,
            distill_tool_types::ExecutionModeChoice::Auto,
            mode,
        ) {
            tracing::warn!(%error, "auto execution mode could not be persisted");
        }
        let payload = serde_json::json!({
            "sessionId": self.session_info.id.0.as_ref(),
            "mode": mode,
            "choice": distill_tool_types::ExecutionModeChoice::Auto,
            "reason": reason,
        });
        if let Ok(params) = serde_json::value::to_raw_value(&payload) {
            self.notifications
                .gateway
                .forward_fire_and_forget(acp::ExtNotification::new(MODE_CHANGED_METHOD, params.into()));
        }
    }

    /// Request start: Jev picks the mode for this request. Without an answer
    /// the mode Auto last used stays.
    pub(super) async fn jev_auto_pick_mode(&self, request: &str) {
        if !self.auto_mode_active() {
            return;
        }
        self.jev_ledger.borrow_mut().auto_mode = AutoModeTurn::default();
        let Some(questions) = mode_question() else {
            return;
        };
        let state = self.auto_mode_state(request, None, 0);
        let Some(answers) = crate::jev::ask_item(JevLever::B8ExecutionMode, state, questions).await else {
            crate::jev::record_item(JevLever::B8ExecutionMode, "defer", "Jev gave no answer", None, None);
            return;
        };
        let confidence = answers.confidence(MODE_QUESTION);
        let Some((mode, _)) = picked_mode(&answers) else {
            crate::jev::record_item(JevLever::B8ExecutionMode, "defer", "no usable mode answer", confidence, Some(&answers));
            return;
        };
        // The user may have fixed a mode while Jev was answering.
        if !self.auto_mode_active() {
            return;
        }
        crate::jev::record_item(JevLever::B8ExecutionMode, mode_id(mode), "request start", confidence, Some(&answers));
        self.publish_auto_mode(mode, "picked for this request");
    }

    /// Round boundary: with new failures or a long run, Jev may move the
    /// request to another mode. Pushes the new directive when it does; `true`
    /// means the caller must rebuild the mode's tool list.
    pub(super) async fn jev_auto_review_mode(&self, request: &str, rounds: usize) -> bool {
        if !self.auto_mode_active() {
            return false;
        }
        let (due, failures) = {
            let ledger = self.jev_ledger.borrow();
            let turn = &ledger.auto_mode;
            let failures = ledger.facts.failed_tool_count();
            let due = turn.switches < MAX_SWITCHES_PER_REQUEST
                && (failures.saturating_sub(turn.failures_at_decision) >= FAILURES_FOR_REVIEW
                    || rounds.saturating_sub(turn.round_at_decision) >= REVIEW_EVERY_ROUNDS);
            (due, failures)
        };
        if !due {
            return false;
        }
        {
            let mut ledger = self.jev_ledger.borrow_mut();
            ledger.auto_mode.failures_at_decision = failures;
            ledger.auto_mode.round_at_decision = rounds;
        }
        let Some(questions) = mode_question() else {
            return false;
        };
        let current = self.execution_mode.load();
        let state = self.auto_mode_state(request, Some(current), rounds);
        let Some(answers) = crate::jev::ask_item(JevLever::B8ExecutionMode, state, questions).await else {
            return false;
        };
        let confidence = answers.confidence(MODE_QUESTION);
        let Some((mode, probability)) = picked_mode(&answers) else {
            return false;
        };
        if !self.auto_mode_active() {
            return false;
        }
        if mode == current || probability < SWITCH_PROBABILITY {
            crate::jev::record_item(JevLever::B8ExecutionMode, "keep", mode_id(current), confidence, Some(&answers));
            return false;
        }
        crate::jev::record_item(JevLever::B8ExecutionMode, mode_id(mode), &format!("switch from {}", mode_id(current)), confidence, Some(&answers));
        self.jev_ledger.borrow_mut().auto_mode.switches += 1;
        let reason = format!("switched from {} after {rounds} tool rounds", current.label());
        self.publish_auto_mode(mode, &reason);
        self.inject_execution_mode_reminder();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn answer(choice: &str, probability: f64) -> distill_workspace::jev::JevAnswerSet {
        distill_workspace::jev::JevAnswerSet {
            model: "test-jev".to_owned(),
            answers: [(
                MODE_QUESTION.to_owned(),
                distill_workspace::jev::Answer::Choice {
                    choice: choice.to_owned(),
                    probabilities: [(choice.to_owned(), probability)].into(),
                    confidence: Some(probability),
                },
            )]
            .into(),
            usage: Default::default(),
            request_id: None,
            latency_ms: 0,
        }
    }

    #[test]
    fn the_question_offers_every_mode_and_unknown_answers_pick_nothing() {
        let questions = mode_question().expect("question");
        assert_eq!(questions.len(), 1);
        for mode in MODES {
            assert_eq!(picked_mode(&answer(mode_id(mode), 0.9)).map(|(m, _)| m), Some(mode));
        }
        assert!(picked_mode(&answer("turbo", 0.9)).is_none());
    }

    /// Auto picks at request start, may switch later on evidence, and a fixed
    /// user choice is never overridden.
    #[tokio::test(flavor = "current_thread")]
    async fn auto_picks_switches_on_evidence_and_never_overrides_a_fixed_mode() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let actor = super::super::support::plain_actor().await;
                actor.execution_mode.store_auto(ExecutionMode::Normal);
                crate::jev::set_test_decision_answers([Some(answer("ultrasave", 0.8))]);
                actor.jev_auto_pick_mode("rename a variable").await;
                assert!(actor.execution_mode.is_auto());
                assert_eq!(actor.execution_mode.load(), ExecutionMode::Ultrasave);

                // No signal yet: nothing is asked.
                crate::jev::set_test_decision_answers([Some(answer("normal", 0.9))]);
                actor.jev_auto_review_mode("rename a variable", 1).await;
                assert_eq!(crate::jev::test_decision_answers_remaining(), 1);

                // A long run is a signal; a weak answer keeps the mode.
                crate::jev::set_test_decision_answers([Some(answer("normal", 0.4))]);
                actor.jev_auto_review_mode("rename a variable", REVIEW_EVERY_ROUNDS).await;
                assert_eq!(actor.execution_mode.load(), ExecutionMode::Ultrasave);

                // A confident answer switches.
                crate::jev::set_test_decision_answers([Some(answer("normal", 0.9))]);
                actor.jev_auto_review_mode("rename a variable", 2 * REVIEW_EVERY_ROUNDS).await;
                assert_eq!(actor.execution_mode.load(), ExecutionMode::Normal);
                assert!(actor.execution_mode.is_auto());

                // A fixed choice belongs to the user.
                actor.execution_mode.store(ExecutionMode::Ultracode);
                crate::jev::set_test_decision_answers([Some(answer("ultrasave", 0.99))]);
                actor.jev_auto_pick_mode("rename a variable").await;
                assert_eq!(actor.execution_mode.load(), ExecutionMode::Ultracode);
                assert_eq!(crate::jev::test_decision_answers_remaining(), 1);
                crate::jev::clear_test_decision_answers();
            })
            .await;
    }
}
