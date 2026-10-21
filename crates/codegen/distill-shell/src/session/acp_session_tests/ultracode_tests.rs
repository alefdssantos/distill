// Modified for Distill by Samuel Fajreldines, 2026.
//! Ultracode: the per-turn reminder is pushed only while the flag is on, and effort is untouched.
use super::support::*;
use super::*;

#[tokio::test]
async fn execution_mode_reminder_supersedes_previous_modes_without_changing_effort() {
    use distill_tool_types::ExecutionMode;
    tokio::task::LocalSet::new().run_until(async {
        let (actor, _gateway_rx) = build_actor().await;
        let before = actor.chat_state_handle.get_sampling_config().await.map(|cfg| cfg.reasoning_effort);
        for (mode, expected) in [
            (ExecutionMode::Normal, "Normal execution mode is active"),
            (ExecutionMode::Ultracode, "Ultracode mode is ON"),
            (ExecutionMode::Ultrafast, "UltraFast mode is active"),
            (ExecutionMode::Ultrasave, "UltraSave mode is active"),
            (ExecutionMode::Normal, "Normal execution mode is active"),
        ] {
            actor.execution_mode.store(mode);
            actor.inject_execution_mode_reminder();
            let conversation = actor.chat_state_handle.get_conversation().await;
            let text = conversation.last().unwrap().text_content();
            assert!(text.contains(expected), "{text}");
            assert!(text.contains("supersedes all earlier"), "{text}");
        }
        assert_eq!(before, actor.chat_state_handle.get_sampling_config().await.map(|cfg| cfg.reasoning_effort));
    }).await;
}

#[tokio::test]
async fn execution_policy_survives_agent_rebuild_and_shared_off() {
    use distill_tools::implementations::distill::task::types::ExecutionPolicy;
    tokio::task::LocalSet::new()
        .run_until(async {
            let mut spec = crate::session::agent_rebuild::test_rebuild_spec_default();
            let enabled = std::sync::Arc::new(distill_tool_types::AtomicExecutionMode::new(distill_tool_types::ExecutionMode::Ultracode));
            let mutable = std::sync::Arc::get_mut(&mut spec).unwrap();
            mutable.execution_policy = Some(ExecutionPolicy {
                mode: enabled.clone(),
                max_depth: 2,
                off_max_depth: 1,
                capability_ceiling: None,
                allowed_subagent_types: None,
            });
            mutable.subagent_event_tx = Some(tokio::sync::mpsc::unbounded_channel().0);
            let first = spec
                .build_agent(
                    distill_agent::config::AgentDefinition::default_distill(),
                    distill_agent::DEFAULT_SYSTEM_PROMPT_LABEL,
                    crate::test_support::TEST_MODEL,
                )
                .await
                .unwrap();
            let rebuilt = spec
                .build_agent(
                    distill_agent::config::AgentDefinition::default_distill(),
                    distill_agent::DEFAULT_SYSTEM_PROMPT_LABEL,
                    crate::test_support::TEST_MODEL,
                )
                .await
                .unwrap();
            let before = first
                .tool_bridge()
                .toolset()
                .get_resource_cloned::<ExecutionPolicy>()
                .await
                .unwrap();
            let after = rebuilt
                .tool_bridge()
                .toolset()
                .get_resource_cloned::<ExecutionPolicy>()
                .await
                .unwrap();
            assert!(std::sync::Arc::ptr_eq(&before.mode, &after.mode));
            assert_eq!(after.max_depth, 2);
            enabled.store(distill_tool_types::ExecutionMode::Normal);
            assert!(!before.is_enabled() && !after.is_enabled());
        })
        .await;
}
