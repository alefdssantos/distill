#![cfg_attr(rustfmt, rustfmt::skip)]
    use super::*;

    fn execution_mode_changed_ext(session_id: &str, mode: &str, reason: &str) -> acp::ExtNotification {
        let payload = serde_json::json!({
            "sessionId": session_id,
            "mode": mode,
            "choice": "auto",
            "reason": reason,
        });
        let raw = serde_json::value::to_raw_value(&payload).unwrap();
        acp::ExtNotification::new("x.ai/session/execution_mode/changed", std::sync::Arc::from(raw))
    }

    #[test]
    fn auto_pick_updates_active_footer_and_scrollback_once() {
        use distill_tool_types::{ExecutionMode, ExecutionModeChoice};
        let mut app = make_app_with_agent("sess-auto");
        assert!(handle_ext_notification(
            &execution_mode_changed_ext("sess-auto", "ultrasave", "long refactor"),
            &mut app,
        ));
        let agent = app.agents.get(&AgentId(0)).unwrap();
        assert_eq!(agent.session.models.execution_mode, ExecutionMode::Ultrasave);
        assert_eq!(agent.session.models.execution_mode_choice, ExecutionModeChoice::Auto);
        assert_eq!(app.models.execution_mode, ExecutionMode::Ultrasave);
        assert_eq!(app.models.execution_mode_choice, ExecutionModeChoice::Auto);
        let lines: Vec<String> = (0..agent.scrollback.len())
            .filter_map(|i| match agent.scrollback.get(i).map(|e| &e.block) {
                Some(RenderBlock::System(b)) if b.text.starts_with("Auto mode:") => Some(b.text.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(lines, ["Auto mode: UltraSave — long refactor"]);
    }
