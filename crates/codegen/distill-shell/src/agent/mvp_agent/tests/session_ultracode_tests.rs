// Modified for Distill by Samuel Fajreldines, 2026.
//! `x.ai/session/ultracode/set`: capability, toggle/explicit set, effort untouched.
use agent_client_protocol as acp;
use distill_tool_types::ExecutionMode;

use super::{build_minimal_agent_for_tests, make_test_handle, run_local_for_bridge_test};
use crate::agent::mvp_agent::MvpAgent;
use crate::extensions::session_ultracode::{SET_METHOD, handle};

const SESSION: &str = "ultracode-sess";

// Re-exec beats the cached Distill home and keeps synthetic state out of the
// user's store, using the existing isolated-process test pattern.
fn run_in_isolated_home(test_name: &str) -> bool {
    const CHILD: &str = "DISTILL_ULTRACODE_TEST_CHILD";
    if std::env::var(CHILD).as_deref() == Ok(test_name) {
        return false;
    }
    let home = tempfile::tempdir().unwrap();
    let filter = module_path!().split_once("::").unwrap().1;
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            &format!("{filter}::{test_name}"),
            "--nocapture",
            "--test-threads=1",
        ])
        .env(CHILD, test_name)
        .env("DISTILL_HOME", home.path())
        .env("GROK_HOME", home.path())
        .output()
        .expect("isolated UltraCode test process");
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("1 passed"),
        "child filter must run one test"
    );
    true
}

fn agent_with_session() -> MvpAgent {
    let agent = build_minimal_agent_for_tests();
    let sid = acp::SessionId::new(SESSION);
    let mut session = make_test_handle("test-model", false, None);
    session.info.id = sid.clone();
    session.reasoning_effort = Some(distill_sampling_types::ReasoningEffort::Low);
    agent.insert_resident(&sid, session);
    agent
}

async fn set(agent: &MvpAgent, params: serde_json::Value) -> Result<serde_json::Value, acp::Error> {
    let request =
        acp::ExtRequest::new(SET_METHOD, serde_json::value::to_raw_value(&params).unwrap().into());
    let response = handle(agent, &request).await?;
    Ok(serde_json::from_str(response.0.get()).unwrap())
}

fn flag(agent: &MvpAgent) -> bool {
    let sid = acp::SessionId::new(SESSION);
    agent.resident_handle(&sid).unwrap().execution_mode.load() == ExecutionMode::Ultracode
}

#[test]
fn initialize_advertises_session_ultracode() {
    let capabilities = crate::agent::mvp_agent::acp_agent::x_ai_capabilities();
    assert_eq!(capabilities["sessionUltracode"], serde_json::json!(true));
}

/// Omitted `enabled` toggles; explicit values set. The flag is the only thing that changes, never effort.
#[test]
fn ultracode_set_toggles_and_sets_without_touching_effort() {
    if run_in_isolated_home("ultracode_set_toggles_and_sets_without_touching_effort") {
        return;
    }
    run_local_for_bridge_test(|| async {
        let agent = agent_with_session();
        let sid = acp::SessionId::new(SESSION);
        assert!(!flag(&agent));
        let info = agent.resident_handle(&sid).unwrap().info;
        assert!(crate::extensions::session_ultracode::load_mode(&info).unwrap() == ExecutionMode::Normal);

        let out = set(&agent, serde_json::json!({ "sessionId": SESSION })).await.unwrap();
        assert_eq!(out, serde_json::json!({ "enabled": true }));
        assert!(flag(&agent));
        assert!(crate::extensions::session_ultracode::load_mode(&info).unwrap() == ExecutionMode::Ultracode);

        let out = set(&agent, serde_json::json!({ "sessionId": SESSION, "enabled": true }))
            .await
            .unwrap();
        assert_eq!(out, serde_json::json!({ "enabled": true }), "explicit on is idempotent");

        let out = set(&agent, serde_json::json!({ "sessionId": SESSION })).await.unwrap();
        assert_eq!(out, serde_json::json!({ "enabled": false }));
        assert!(!flag(&agent));

        let out = set(&agent, serde_json::json!({ "sessionId": SESSION, "enabled": false }))
            .await
            .unwrap();
        assert_eq!(out, serde_json::json!({ "enabled": false }));

        assert!(
            crate::extensions::session_ultracode::load_mode(&info).unwrap() == ExecutionMode::Normal,
            "explicit off must survive reload"
        );
        let handle = agent.resident_handle(&sid).unwrap();
        assert_eq!(
            handle.reasoning_effort,
            Some(distill_sampling_types::ReasoningEffort::Low),
            "toggling Ultracode must not change effort"
        );
    });
}

#[test]
fn ultracode_set_unknown_session_is_not_found() {
    run_local_for_bridge_test(|| async {
        let agent = agent_with_session();
        let err = set(&agent, serde_json::json!({ "sessionId": "nope", "enabled": true }))
            .await
            .unwrap_err();
        assert_eq!(err.code, acp::Error::resource_not_found(None).code);
    });
}

#[test]
fn ultracode_child_toggle_cannot_change_or_persist_the_root_switch() {
    if run_in_isolated_home("ultracode_child_toggle_cannot_change_or_persist_the_root_switch") {
        return;
    }
    run_local_for_bridge_test(|| async {
        let agent = agent_with_session();
        set(
            &agent,
            serde_json::json!({ "sessionId": SESSION, "enabled": true }),
        )
        .await
        .unwrap();
        let root = agent
            .resident_handle(&acp::SessionId::new(SESSION))
            .unwrap();
        let mut child = make_test_handle("test-model", false, None);
        child.info.id = acp::SessionId::new("ultracode-child");
        child.tool_context.subagent_depth = 1;
        child.execution_mode = root.execution_mode.clone();
        let child_info = child.info.clone();
        agent.insert_resident(&child.info.id.clone(), child);
        let error = set(
            &agent,
            serde_json::json!({ "sessionId": "ultracode-child", "enabled": false }),
        )
        .await
        .unwrap_err();
        assert_eq!(error.code, acp::Error::invalid_params().code);
        assert!(flag(&agent));
        assert!(crate::extensions::session_ultracode::load_mode(&root.info).unwrap() == ExecutionMode::Ultracode);
        assert!(
            !crate::session::persistence::session_dir(&child_info)
                .join("execution_mode.json")
                .exists()
        );
        set(
            &agent,
            serde_json::json!({ "sessionId": SESSION, "enabled": false }),
        )
        .await
        .unwrap();
        assert!(root.execution_mode.load() == ExecutionMode::Normal);
        assert!(crate::extensions::session_ultracode::load_mode(&root.info).unwrap() == ExecutionMode::Normal);
    });
}

#[test]
fn execution_modes_replace_each_other_and_persist_without_changing_pins() {
    if run_in_isolated_home("execution_modes_replace_each_other_and_persist_without_changing_pins") {
        return;
    }
    run_local_for_bridge_test(|| async {
        let agent = agent_with_session();
        let root = agent.resident_handle(&acp::SessionId::new(SESSION)).unwrap();
        // Existing saved UltraCode sessions still restore correctly.
        let dir = crate::session::persistence::ensure_owner_only_session_dir(&root.info).unwrap();
        std::fs::write(dir.join("ultracode.json"), r#"{"enabled":true}"#).unwrap();
        assert_eq!(crate::extensions::session_ultracode::load_mode(&root.info).unwrap(), ExecutionMode::Ultracode);
        for mode in [ExecutionMode::Ultrafast, ExecutionMode::Ultracode, ExecutionMode::Ultrasave, ExecutionMode::Normal, ExecutionMode::Ultrafast] {
            let params = serde_json::json!({"sessionId": SESSION, "mode": mode});
            let request = acp::ExtRequest::new(
                crate::extensions::session_ultracode::MODE_SET_METHOD,
                serde_json::value::to_raw_value(&params).unwrap().into(),
            );
            let response = handle(&agent, &request).await.unwrap();
            assert_eq!(serde_json::from_str::<serde_json::Value>(response.0.get()).unwrap(), serde_json::json!({"mode": mode}));
            assert_eq!(root.execution_mode.load(), mode);
            assert_eq!(crate::extensions::session_ultracode::load_mode(&root.info).unwrap(), mode);
        }
        // Disabling an inactive legacy mode must not disable UltraFast.
        set(&agent, serde_json::json!({"sessionId": SESSION, "enabled": false})).await.unwrap();
        assert_eq!(root.execution_mode.load(), ExecutionMode::Ultrafast);
        // Relative commands use server state even when a client has not yet
        // received the preceding acknowledgement.
        for (target, operation, expected) in [
            (ExecutionMode::Ultrafast, "toggle", ExecutionMode::Normal),
            (ExecutionMode::Ultrafast, "toggle", ExecutionMode::Ultrafast),
            (ExecutionMode::Ultracode, "disable", ExecutionMode::Ultrafast),
            (ExecutionMode::Ultrafast, "disable", ExecutionMode::Normal),
            (ExecutionMode::Ultrasave, "toggle", ExecutionMode::Ultrasave),
            (ExecutionMode::Ultrafast, "disable", ExecutionMode::Ultrasave),
            (ExecutionMode::Ultrasave, "toggle", ExecutionMode::Normal),
        ] {
            let params = serde_json::json!({"sessionId": SESSION, "mode": target, "operation": operation});
            let request = acp::ExtRequest::new(
                crate::extensions::session_ultracode::MODE_SET_METHOD,
                serde_json::value::to_raw_value(&params).unwrap().into(),
            );
            let response = handle(&agent, &request).await.unwrap();
            assert_eq!(serde_json::from_str::<serde_json::Value>(response.0.get()).unwrap(), serde_json::json!({"mode": expected}));
            assert_eq!(root.execution_mode.load(), expected);
        }
        assert_eq!(root.reasoning_effort, Some(distill_sampling_types::ReasoningEffort::Low));
    });
}
