use super::*;
use crate::agent::account_routing::{self, AccountRoute, tests::NamedAccounts};

#[test]
#[serial_test::serial]
fn account_routing_named_only_session_admission_auto_and_cold_route_restore() {
    crate::agent::account_routing::tests::isolated(
        "agent::mvp_agent::tests::account_routing_tests::account_routing_named_only_session_admission_auto_and_cold_route_restore",
        || {
            run_local_for_bridge_test(|| async {
                let fixture = NamedAccounts::new("http://127.0.0.1:1/v1");
                fixture.routing(crate::accounts::AutoStrategy::HighestRemaining, true);
                fixture.quota("one", Some(10.0));
                fixture.quota("two", Some(30.0));
                fixture.quota("three", Some(80.0));
                assert!(!crate::codex_auth::is_logged_in(), "no legacy credentials");
                let agent = build_minimal_agent_for_tests();
                for (id, model) in &fixture.models {
                    agent.models_manager.insert_test_entry(id, model.clone());
                }
                agent
                    .initialize_request
                    .set(
                        acp::InitializeRequest::new(acp::ProtocolVersion::V1).client_capabilities(
                            acp::ClientCapabilities::new()
                                .fs(acp::FileSystemCapabilities::new())
                                .terminal(false),
                        ),
                    )
                    .unwrap();
                let cwd = tempfile::tempdir().unwrap();
                let meta = serde_json::json!({"modelId":"auto","workerModelId":"gpt-6-luna"})
                    .as_object()
                    .cloned();
                let sid = agent
                    .new_session_inner(
                        acp::NewSessionRequest::new(cwd.path().to_path_buf()).meta(meta),
                    )
                    .await
                    .expect("named-only Auto session admission")
                    .session_id;
                let handle = agent.resident_handle(&sid).unwrap();
                assert_eq!(handle.model_id.0.as_ref(), "gpt-6-luna");
                let config = handle
                    .chat_state_handle
                    .get_sampling_config()
                    .await
                    .unwrap();
                let route = AccountRoute::read(&config.extra_headers).unwrap().unwrap();
                assert_eq!(route.account_id, "three");
                assert_eq!(
                    route.worker_model, "gpt-6.1-sol",
                    "Auto bundle wins over previous worker pin"
                );
                assert!(!route.pinned);
                assert!(!config.extra_headers.contains_key("x-distill-worker-pin"));
                assert!(
                    agent
                        .model_state(Some(&sid))
                        .available_models
                        .iter()
                        .any(|model| model.model_id.0.as_ref() == "auto")
                );
                let restored = account_routing::load_session_config(&handle.info, &fixture.models)
                    .await
                    .unwrap()
                    .unwrap();
                assert_eq!(
                    restored
                        .bearer_resolver
                        .unwrap()
                        .current_bearer()
                        .as_deref(),
                    Some("three-token")
                );
                drain_persistence(&agent, &sid).await;
                agent.remove_session(&sid);
                Box::pin(
                    agent.load_session_inner(
                        acp::LoadSessionRequest::new(sid.clone(), cwd.path().to_path_buf()).meta(
                            serde_json::json!({"workerModelId":"gpt-6-luna"})
                                .as_object()
                                .cloned(),
                        ),
                    ),
                )
                .await
                .expect("cold load with an explicit worker");
                let loaded = agent.resident_handle(&sid).unwrap();
                assert_eq!(
                    crate::extensions::session_worker_model::effective_worker(
                        &loaded.worker_override
                    )
                    .model_id
                    .as_deref(),
                    Some("gpt-6-luna"),
                    "load chooses the supplied worker instead of the saved Auto worker"
                );
                let sampling = loaded
                    .chat_state_handle
                    .get_sampling_config()
                    .await
                    .unwrap();
                assert!(sampling.extra_headers.contains_key("x-distill-worker-pin"));
                let loaded_route = AccountRoute::read(&sampling.extra_headers)
                    .unwrap()
                    .unwrap();
                assert_eq!(loaded_route.account_id, "three");
                assert_eq!(loaded_route.option_id, route.option_id);
                assert!(!loaded_route.pinned, "worker selection keeps Main on Auto");
                let restored = account_routing::load_session_config(&loaded.info, &fixture.models)
                    .await
                    .unwrap()
                    .unwrap();
                assert!(restored.extra_headers.contains_key("x-distill-worker-pin"));

                Box::pin(
                    agent.resume_session_inner(
                        acp::ResumeSessionRequest::new(sid.clone(), cwd.path().to_path_buf())
                            .meta(serde_json::json!({"workerModelId":""}).as_object().cloned()),
                    ),
                )
                .await
                .expect("resident resume disabling the worker");
                let resumed = agent.resident_handle(&sid).unwrap();
                assert_eq!(
                    crate::extensions::session_worker_model::effective_worker(
                        &resumed.worker_override
                    )
                    .model_id,
                    None,
                    "an empty worker selection disables delegation's worker"
                );
                let sampling = resumed
                    .chat_state_handle
                    .get_sampling_config()
                    .await
                    .unwrap();
                assert!(sampling.extra_headers.contains_key("x-distill-worker-pin"));
                let resumed_route = AccountRoute::read(&sampling.extra_headers)
                    .unwrap()
                    .unwrap();
                assert_eq!(resumed_route.option_id, route.option_id);
                assert!(!resumed_route.pinned);
                let restored = account_routing::load_session_config(&resumed.info, &fixture.models)
                    .await
                    .unwrap()
                    .unwrap();
                assert!(restored.extra_headers.contains_key("x-distill-worker-pin"));

                // Both attach entry points must replace a saved concrete choice with Auto.
                for cold_load in [true, false] {
                    Box::pin(crate::agent::handlers::model_switch::apply(
                        &agent,
                        acp::SetSessionModelRequest::new(
                            sid.clone(),
                            acp::ModelId::new("gpt-6.1-sol"),
                        ),
                        crate::agent::handlers::model_switch::SwitchEffort::Preserve,
                        crate::agent::handlers::model_switch::ConfigNotice::Skip,
                    ))
                    .await
                    .expect("save a concrete model different from Auto's Main");
                    drain_persistence(&agent, &sid).await;
                    let saved: crate::session::persistence::Summary = serde_json::from_slice(
                        &std::fs::read(
                            crate::session::persistence::session_dir(&handle.info)
                                .join("summary.json"),
                        )
                        .unwrap(),
                    )
                    .unwrap();
                    assert_eq!(saved.current_model_id.0.as_ref(), "gpt-6.1-sol");

                    if cold_load {
                        agent.remove_session(&sid);
                        Box::pin(
                            agent.load_session_inner(
                                acp::LoadSessionRequest::new(sid.clone(), cwd.path().to_path_buf())
                                    .meta(
                                        serde_json::json!({
                                            "modelId": "auto",
                                            "workerModelId": "gpt-6-luna",
                                        })
                                        .as_object()
                                        .cloned(),
                                    ),
                            ),
                        )
                        .await
                        .expect("cold load with explicit ACP Auto");
                    } else {
                        agent.cfg.borrow_mut().default_model_override = Some("auto".into());
                        Box::pin(
                            agent.resume_session_inner(
                                acp::ResumeSessionRequest::new(
                                    sid.clone(),
                                    cwd.path().to_path_buf(),
                                )
                                .meta(
                                    serde_json::json!({"workerModelId": ""})
                                        .as_object()
                                        .cloned(),
                                ),
                            ),
                        )
                        .await
                        .expect("resident resume with explicit CLI Auto");
                        agent.cfg.borrow_mut().default_model_override = None;
                    }
                    let auto = agent.resident_handle(&sid).unwrap();
                    assert_eq!(auto.model_id.0.as_ref(), "gpt-6-luna");
                    let sampling = auto.chat_state_handle.get_sampling_config().await.unwrap();
                    let selected = AccountRoute::read(&sampling.extra_headers)
                        .unwrap()
                        .unwrap();
                    assert_eq!(selected.account_id, "three");
                    assert_eq!(selected.option_id.as_deref(), Some("three"));
                    assert_eq!(selected.main_model, "gpt-6-luna");
                    assert_eq!(selected.worker_model, "gpt-6.1-sol");
                    assert!(!selected.pinned);
                    assert!(
                        !sampling
                            .extra_headers
                            .contains_key(account_routing::PIN_HEADER)
                    );
                    assert!(!sampling.extra_headers.contains_key("x-distill-worker-pin"));
                    assert_eq!(
                        crate::extensions::session_worker_model::effective_worker(
                            &auto.worker_override
                        )
                        .model_id
                        .as_deref(),
                        Some("gpt-6.1-sol"),
                        "explicit Auto replaces stale native worker metadata (cold_load={cold_load})"
                    );
                    let persisted =
                        account_routing::load_session_config(&auto.info, &fixture.models)
                            .await
                            .unwrap()
                            .unwrap();
                    let persisted_route = AccountRoute::read(&persisted.extra_headers)
                        .unwrap()
                        .unwrap();
                    assert_eq!(persisted_route.option_id.as_deref(), Some("three"));
                    assert!(!persisted_route.pinned);
                    assert!(!persisted.extra_headers.contains_key("x-distill-worker-pin"));
                }

                let mut effort_model = fixture.models["gpt-6-luna"].clone();
                effort_model.info.supports_reasoning_effort = true;
                effort_model.info.reasoning_effort = Some(ReasoningEffort::High);
                agent
                    .models_manager
                    .insert_test_entry("gpt-6-luna", effort_model);
                Box::pin(
                    agent.set_model_gated(
                        acp::SetSessionModelRequest::new(
                            sid.clone(),
                            acp::ModelId::new("gpt-6-luna"),
                        )
                        .meta(
                            serde_json::json!({"reasoningEffort":"high"})
                                .as_object()
                                .cloned(),
                        ),
                    ),
                )
                .await
                .expect("start from an explicit High effort");
                assert_eq!(
                    agent
                        .resident_handle(&sid)
                        .unwrap()
                        .chat_state_handle
                        .get_sampling_config()
                        .await
                        .unwrap()
                        .reasoning_effort,
                    Some(ReasoningEffort::High)
                );
                agent
                    .session_registry
                    .set_unavailable_model(&sid, acp::ModelId::new("gone-model"));
                let acknowledged = Box::pin(
                    agent.set_model_gated(
                        acp::SetSessionModelRequest::new(sid.clone(), acp::ModelId::new("auto"))
                            .meta(
                                serde_json::json!({"reasoningEffort":"low"})
                                    .as_object()
                                    .cloned(),
                            ),
                    ),
                )
                .await
                .expect("Auto applies the explicit effort and recovers the session");
                let meta = acknowledged.meta.unwrap();
                assert_eq!(meta["canonicalModelId"], serde_json::json!("auto"));
                assert_eq!(meta["model"], serde_json::json!({"Ok": "gpt-6-luna"}));
                assert_eq!(meta["reasoningEffort"], serde_json::json!("low"));
                assert!(agent.session_registry.unavailable_model(&sid).is_none());
                assert_eq!(
                    agent
                        .resident_handle(&sid)
                        .unwrap()
                        .chat_state_handle
                        .get_sampling_config()
                        .await
                        .unwrap()
                        .reasoning_effort,
                    Some(ReasoningEffort::Low)
                );
                assert_eq!(
                    agent.model_state(Some(&sid)).current_model_id.0.as_ref(),
                    "gpt-6-luna",
                    "model_state continues to report the active concrete model"
                );
                assert_eq!(
                    agent
                        .resident_handle(&sid)
                        .unwrap()
                        .chat_state_handle
                        .get_sampling_config()
                        .await
                        .unwrap()
                        .model,
                    "gpt-6-luna",
                    "the logical acknowledgement never becomes a wire model"
                );

                // A zero-turn harness change must not commit before its route is durable.
                {
                    let current = agent.resident_handle(&sid).unwrap();
                    assert_eq!(
                        current.signals_handle.snapshot().await.unwrap().turn_count,
                        0
                    );
                    assert!(
                        current
                            .gateway_enabled
                            .load(std::sync::atomic::Ordering::Relaxed)
                    );
                    let (tx, rx) = tokio::sync::oneshot::channel();
                    current
                        .cmd_tx
                        .send(crate::session::SessionCommand::GetActiveAgent { responds_to: tx })
                        .unwrap();
                    let previous_harness = rx.await.unwrap();
                    assert_ne!(previous_harness.as_deref(), Some("codex"));
                    let previous_worker = current.worker_override.read().clone();
                    let previous_sampling = current
                        .chat_state_handle
                        .get_sampling_config()
                        .await
                        .unwrap();
                    let previous_conversation = current.chat_state_handle.get_conversation().await;
                    let previous_effort_auto = current
                        .jev_effort_auto
                        .load(std::sync::atomic::Ordering::Relaxed);
                    let previous_default_effort_auto = agent.models_manager.current_effort_auto();
                    assert!(!previous_effort_auto);
                    drain_persistence(&agent, &sid).await;
                    let dir = crate::session::persistence::session_dir(&current.info);
                    let previous_prompt_files = [
                        "prompt_context.json",
                        "system_prompt.txt",
                        "chat_history.jsonl",
                    ]
                    .map(|name| {
                        let path = dir.join(name);
                        let bytes = std::fs::read(&path).unwrap();
                        (path, bytes)
                    });
                    let route_path = dir.join("account-route.json");
                    let previous_route = std::fs::read(&route_path).unwrap();
                    fixture.quota("one", Some(90.0));
                    let mut strict_model = fixture.models["gpt-6.1-sol"].clone();
                    strict_model.info.agent_type = "codex".into();
                    agent
                        .models_manager
                        .insert_test_entry("gpt-6.1-sol", strict_model);
                    let backup = route_path.with_extension("backup");
                    std::fs::rename(&route_path, &backup).unwrap();
                    std::fs::create_dir(&route_path).unwrap();
                    let rejected = Box::pin(
                        agent.set_model_gated(
                            acp::SetSessionModelRequest::new(
                                sid.clone(),
                                acp::ModelId::new("auto"),
                            )
                            .meta(
                                serde_json::json!({"reasoningEffortAuto": true})
                                    .as_object()
                                    .cloned(),
                            ),
                        ),
                    )
                    .await;
                    std::fs::remove_dir(&route_path).unwrap();
                    std::fs::rename(&backup, &route_path).unwrap();
                    let rejected =
                        rejected.expect_err("a blocked route rejects the zero-turn harness switch");
                    assert_eq!(
                        rejected.data,
                        Some(serde_json::json!(
                            "Could not persist selected account route"
                        ))
                    );
                    let (tx, rx) = tokio::sync::oneshot::channel();
                    current
                        .cmd_tx
                        .send(crate::session::SessionCommand::GetActiveAgent { responds_to: tx })
                        .unwrap();
                    assert_eq!(
                        rx.await.unwrap(),
                        previous_harness,
                        "a rejected switch keeps the active harness"
                    );
                    let unchanged = agent.resident_handle(&sid).unwrap();
                    assert_eq!(unchanged.model_id, current.model_id);
                    assert_eq!(unchanged.agent_name, current.agent_name);
                    assert_eq!(*unchanged.worker_override.read(), previous_worker);
                    let sampling = unchanged
                        .chat_state_handle
                        .get_sampling_config()
                        .await
                        .unwrap();
                    assert_eq!(sampling.model, previous_sampling.model);
                    assert_eq!(sampling.extra_headers, previous_sampling.extra_headers);
                    assert_eq!(
                        sampling.reasoning_effort,
                        previous_sampling.reasoning_effort
                    );
                    assert_eq!(
                        unchanged
                            .jev_effort_auto
                            .load(std::sync::atomic::Ordering::Relaxed),
                        previous_effort_auto
                    );
                    assert_eq!(
                        agent.models_manager.current_effort_auto(),
                        previous_default_effort_auto
                    );
                    assert_eq!(
                        serde_json::to_value(unchanged.chat_state_handle.get_conversation().await)
                            .unwrap(),
                        serde_json::to_value(&previous_conversation).unwrap()
                    );
                    drain_persistence(&agent, &sid).await;
                    assert_eq!(std::fs::read(&route_path).unwrap(), previous_route);
                    for (path, bytes) in previous_prompt_files {
                        assert_eq!(
                            std::fs::read(&path).unwrap(),
                            bytes,
                            "{} changed after rejection",
                            path.display()
                        );
                    }

                    let switched = Box::pin(
                        agent.set_model_gated(
                            acp::SetSessionModelRequest::new(
                                sid.clone(),
                                acp::ModelId::new("auto"),
                            )
                            .meta(
                                serde_json::json!({"reasoningEffortAuto": true})
                                    .as_object()
                                    .cloned(),
                            ),
                        ),
                    )
                    .await
                    .expect("the same zero-turn switch succeeds after removing the blocked route");
                    assert_eq!(
                        switched.meta.unwrap()["model"],
                        serde_json::json!({"Ok": "gpt-6.1-sol"})
                    );
                    let (tx, rx) = tokio::sync::oneshot::channel();
                    current
                        .cmd_tx
                        .send(crate::session::SessionCommand::GetActiveAgent { responds_to: tx })
                        .unwrap();
                    assert_eq!(rx.await.unwrap().as_deref(), Some("codex"));
                    let switched = agent.resident_handle(&sid).unwrap();
                    assert_eq!(switched.model_id.0.as_ref(), "gpt-6.1-sol");
                    assert_eq!(switched.agent_name, "codex");
                    assert_eq!(
                        switched
                            .worker_override
                            .read()
                            .as_ref()
                            .unwrap()
                            .model_id
                            .as_deref(),
                        Some("gpt-6-luna")
                    );
                    assert!(
                        switched
                            .jev_effort_auto
                            .load(std::sync::atomic::Ordering::Relaxed)
                    );
                    let sampling = switched
                        .chat_state_handle
                        .get_sampling_config()
                        .await
                        .unwrap();
                    let selected = AccountRoute::read(&sampling.extra_headers)
                        .unwrap()
                        .unwrap();
                    assert_eq!(selected.account_id, "one");
                    assert_eq!(selected.main_model, "gpt-6.1-sol");
                    assert_eq!(selected.worker_model, "gpt-6-luna");
                    let conversation = switched.chat_state_handle.get_conversation().await;
                    assert_ne!(
                        distill_sampling_types::current_system_prompt(&conversation),
                        distill_sampling_types::current_system_prompt(&previous_conversation)
                    );
                    let persisted = account_routing::load_session_config(
                        &switched.info,
                        &agent.models_manager.models(),
                    )
                    .await
                    .unwrap()
                    .unwrap();
                    assert_eq!(
                        AccountRoute::read(&persisted.extra_headers)
                            .unwrap()
                            .unwrap()
                            .account_id,
                        "one"
                    );

                    fixture.quota("one", Some(10.0));
                    Box::pin(
                        agent.set_model_gated(
                            acp::SetSessionModelRequest::new(
                                sid.clone(),
                                acp::ModelId::new("auto"),
                            )
                            .meta(
                                serde_json::json!({"reasoningEffort": "low"})
                                    .as_object()
                                    .cloned(),
                            ),
                        ),
                    )
                    .await
                    .expect("restore the original Auto pair for the remaining rejection checks");
                }

                let current = agent.resident_handle(&sid).unwrap();
                let previous_worker = current.worker_override.read().clone();
                let previous_sampling = current
                    .chat_state_handle
                    .get_sampling_config()
                    .await
                    .unwrap();
                let route_path = crate::session::persistence::session_dir(&current.info)
                    .join("account-route.json");
                let previous_route = std::fs::read(&route_path).unwrap();
                fixture.quota("one", Some(90.0));
                let mut incompatible_model = fixture.models["gpt-6.1-sol"].clone();
                incompatible_model.info.agent_type = "codex".into();
                agent
                    .models_manager
                    .insert_test_entry("gpt-6.1-sol", incompatible_model);
                current.signals_handle.increment_turn();
                assert!(current.signals_handle.snapshot().await.unwrap().turn_count > 0);
                let rejected = Box::pin(agent.set_model_gated(acp::SetSessionModelRequest::new(
                    sid.clone(),
                    acp::ModelId::new("auto"),
                )))
                .await
                .expect_err("an incompatible Auto Main must reject the whole pair");
                let rejection =
                    config::ModelSwitchIncompatibleAgentError::from_acp_error(&rejected)
                        .expect("Main harness incompatibility");
                assert_eq!(rejection.model_id, "gpt-6.1-sol");
                assert_eq!(rejection.required_agent_type, "codex");
                let unchanged = agent.resident_handle(&sid).unwrap();
                assert_eq!(unchanged.model_id, current.model_id);
                assert_eq!(*unchanged.worker_override.read(), previous_worker);
                let sampling = unchanged
                    .chat_state_handle
                    .get_sampling_config()
                    .await
                    .unwrap();
                assert_eq!(sampling.model, previous_sampling.model);
                assert_eq!(sampling.extra_headers, previous_sampling.extra_headers);
                assert_eq!(std::fs::read(&route_path).unwrap(), previous_route);

                // Exercise the actor's inner error after the compatibility gate succeeds.
                agent
                    .models_manager
                    .insert_test_entry("gpt-6.1-sol", fixture.models["gpt-6.1-sol"].clone());
                let backup = route_path.with_extension("backup");
                std::fs::rename(&route_path, &backup).unwrap();
                std::fs::create_dir(&route_path).unwrap();
                let rejected = Box::pin(agent.set_model_gated(acp::SetSessionModelRequest::new(
                    sid.clone(),
                    acp::ModelId::new("auto"),
                )))
                .await;
                std::fs::remove_dir(&route_path).unwrap();
                std::fs::rename(&backup, &route_path).unwrap();
                let rejected =
                    rejected.expect_err("an actor failure must not acknowledge a switch");
                assert_eq!(
                    rejected.data,
                    Some(serde_json::json!(
                        "Could not persist selected account route"
                    ))
                );
                let unchanged = agent.resident_handle(&sid).unwrap();
                assert_eq!(unchanged.model_id, current.model_id);
                assert_eq!(*unchanged.worker_override.read(), previous_worker);
                let sampling = unchanged
                    .chat_state_handle
                    .get_sampling_config()
                    .await
                    .unwrap();
                assert_eq!(sampling.model, previous_sampling.model);
                assert_eq!(sampling.extra_headers, previous_sampling.extra_headers);
                assert_eq!(std::fs::read(&route_path).unwrap(), previous_route);
                let path = fixture.store.credential_path("three").unwrap();
                std::fs::remove_file(path).unwrap();
                assert!(
                    account_routing::load_session_config(&handle.info, &fixture.models)
                        .await
                        .is_err(),
                    "cold restore never adopts another account after logout"
                );
                agent.remove_session(&sid);
            });
        },
    );
}
