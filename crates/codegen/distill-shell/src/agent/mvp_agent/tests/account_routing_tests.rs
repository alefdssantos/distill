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

                // Same-model account handoffs must normalize both live and durable history.
                {
                    use crate::session::storage::StorageAdapter as _;
                    use base64::Engine as _;
                    use distill_sampling_types::{ContentPart, ConversationItem, ToolCall, rs};
                    let handle = agent.resident_handle(&sid).unwrap();
                    fixture.quota("one", Some(10.0));
                    let mut png = Vec::new();
                    image::ImageBuffer::from_pixel(32, 32, image::Rgba([10u8, 20, 30, 255]))
                        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
                        .unwrap();
                    let image_url = format!(
                        "data:image/png;base64,{}",
                        base64::engine::general_purpose::STANDARD.encode(png)
                    );
                    // Wait behind Initialize before snapshotting or replacing startup history.
                    let (tx, rx) = tokio::sync::oneshot::channel();
                    handle
                        .cmd_tx
                        .send(crate::session::SessionCommand::GetCurrentModel { responds_to: tx })
                        .unwrap();
                    assert_eq!(rx.await.unwrap().id, "gpt-6-luna");
                    let mut history = handle.chat_state_handle.get_conversation().await;
                    let ordinary = vec![
                        ConversationItem::user("account handoff user"),
                        ConversationItem::assistant_tool_calls(vec![ToolCall {
                            id: "account_handoff_call".into(),
                            name: "read_file".into(),
                            arguments: "{}".into(),
                        }]),
                        ConversationItem::tool_result_with_images(
                            "account_handoff_call",
                            "account handoff result",
                            vec![ContentPart::Image {
                                url: image_url.into(),
                            }],
                        ),
                        ConversationItem::agent_message("account handoff task"),
                    ];
                    history.push(ConversationItem::Reasoning(rs::ReasoningItem {
                        id: "old-account-reason".into(),
                        summary: vec![],
                        content: None,
                        encrypted_content: Some("old-account-secret".into()),
                        status: None,
                    }));
                    history.push(ConversationItem::BackendToolCall(
                        distill_sampling_types::BackendToolCallItem {
                            kind: distill_sampling_types::BackendToolKind::WebSearch(
                                rs::WebSearchToolCall {
                                    id: "old-account-backend".into(),
                                    status: rs::WebSearchToolCallStatus::Completed,
                                    action: rs::WebSearchToolCallAction::Search(
                                        rs::WebSearchActionSearch {
                                            query: "handoff search context".into(),
                                            sources: None,
                                        },
                                    ),
                                },
                            ),
                        },
                    ));
                    history.extend(ordinary.clone());
                    let assert_portable = |items: &[ConversationItem]| {
                        assert!(!items.iter().any(|item| matches!(
                            item,
                            ConversationItem::Reasoning(_) | ConversationItem::BackendToolCall(_)
                        )));
                        assert!(
                            items
                                .iter()
                                .any(|item| item.text_content().contains("handoff search context"))
                        );
                        assert!(
                            items
                                .windows(ordinary.len())
                                .any(|items| serde_json::to_value(items).unwrap()
                                    == serde_json::to_value(&ordinary).unwrap()),
                            "human, tool pair, image and raw agent provenance are unchanged"
                        );
                    };
                    handle
                        .chat_state_handle
                        .replace_conversation(history.clone());
                    // The chat actor must enqueue its rewrite before the storage flush.
                    assert_eq!(
                        handle.chat_state_handle.get_conversation_len().await,
                        history.len()
                    );
                    drain_persistence(&agent, &sid).await;
                    let dir = crate::session::persistence::session_dir(&handle.info);
                    let history_path = dir.join("chat_history.jsonl");
                    let route_path = dir.join("account-route.json");
                    let before_history = std::fs::read(&history_path).unwrap();
                    let backup = route_path.with_extension("handoff-backup");
                    fixture.quota("two", Some(90.0)); // two and three both use Luna.
                    std::fs::rename(&route_path, &backup).unwrap();
                    std::fs::create_dir(&route_path).unwrap();
                    let rejected = Box::pin(agent.set_model_gated(
                        acp::SetSessionModelRequest::new(sid.clone(), acp::ModelId::new("auto")),
                    ))
                    .await;
                    std::fs::remove_dir(&route_path).unwrap();
                    std::fs::rename(&backup, &route_path).unwrap();
                    assert!(rejected.is_err());
                    assert_eq!(
                        serde_json::to_value(handle.chat_state_handle.get_conversation().await)
                            .unwrap(),
                        serde_json::to_value(&history).unwrap()
                    );
                    drain_persistence(&agent, &sid).await;
                    assert_eq!(std::fs::read(&history_path).unwrap(), before_history);
                    Box::pin(agent.set_model_gated(acp::SetSessionModelRequest::new(
                        sid.clone(),
                        acp::ModelId::new("auto"),
                    )))
                    .await
                    .expect("explicit Auto changes account on the same model");
                    let switched = agent.resident_handle(&sid).unwrap();
                    let sampling = switched
                        .chat_state_handle
                        .get_sampling_config()
                        .await
                        .unwrap();
                    assert_eq!(sampling.model, "gpt-6-luna");
                    assert_eq!(
                        AccountRoute::read(&sampling.extra_headers)
                            .unwrap()
                            .unwrap()
                            .account_id,
                        "two"
                    );
                    assert_portable(&switched.chat_state_handle.get_conversation().await);
                    drain_persistence(&agent, &sid).await;
                    let storage = crate::session::storage::JsonlStorageAdapter::with_root(
                        crate::util::distill_home::distill_home(),
                    );
                    assert_portable(&storage.load_chat_history_from_dir(&dir).unwrap());

                    // Cold load sees A's transcript while Auto now selects B, with no model change.
                    switched
                        .chat_state_handle
                        .replace_conversation(history.clone());
                    assert_eq!(
                        switched.chat_state_handle.get_conversation_len().await,
                        history.len()
                    );
                    drain_persistence(&agent, &sid).await;
                    fixture.quota("three", Some(95.0));
                    agent.remove_session(&sid);
                    Box::pin(
                        agent.load_session_inner(
                            acp::LoadSessionRequest::new(sid.clone(), cwd.path().to_path_buf())
                                .meta(serde_json::json!({"modelId":"auto"}).as_object().cloned()),
                        ),
                    )
                    .await
                    .expect("cold Auto normalizes the prior account transcript");
                    let loaded = agent.resident_handle(&sid).unwrap();
                    let sampling = loaded
                        .chat_state_handle
                        .get_sampling_config()
                        .await
                        .unwrap();
                    assert_eq!(sampling.model, "gpt-6-luna");
                    assert_eq!(
                        AccountRoute::read(&sampling.extra_headers)
                            .unwrap()
                            .unwrap()
                            .account_id,
                        "three"
                    );
                    assert_portable(&loaded.chat_state_handle.get_conversation().await);
                    drain_persistence(&agent, &sid).await;
                    assert_portable(&storage.load_chat_history_from_dir(&dir).unwrap());

                    // Simulate route B already written while A's old history survived a restart.
                    agent.remove_session(&sid);
                    storage
                        .replace_chat_history(&handle.info, &history)
                        .await
                        .unwrap();
                    Box::pin(agent.load_session_inner(acp::LoadSessionRequest::new(
                        sid.clone(),
                        cwd.path().to_path_buf(),
                    )))
                    .await
                    .expect("matching saved account cannot authenticate opaque disk history");
                    let loaded = agent.resident_handle(&sid).unwrap();
                    assert_portable(&loaded.chat_state_handle.get_conversation().await);
                    let request = loaded
                        .chat_state_handle
                        .build_request(
                            vec![],
                            None,
                            false,
                            None,
                            sid.0.to_string(),
                            "handoff-proof".into(),
                        )
                        .await
                        .unwrap();
                    assert_eq!(
                        serde_json::to_string(&request.items)
                            .unwrap()
                            .matches(
                                distill_chat_state::compaction_utils::AGENT_MESSAGE_MODEL_LABEL
                            )
                            .count(),
                        1
                    );
                    drain_persistence(&agent, &sid).await;
                    assert_portable(&storage.load_chat_history_from_dir(&dir).unwrap());
                    fixture.quota("two", Some(30.0));
                    fixture.quota("three", Some(80.0));
                }
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

#[test]
#[serial_test::serial]
fn account_routing_active_turn_rejects_account_handoff_until_response_is_recorded() {
    crate::agent::account_routing::tests::isolated(
        "agent::mvp_agent::tests::account_routing_tests::account_routing_active_turn_rejects_account_handoff_until_response_is_recorded",
        || {
            run_local_for_bridge_test(|| async {
                tokio::time::timeout(std::time::Duration::from_secs(45), async {
                    use agent_client_protocol::Agent as _;
                    use distill_sampling_types::ConversationItem;
                    use distill_test_support::{
                        InferenceEndpoint, InferenceRequestMatcher, MockInferenceServer,
                        MockModelEntry, ScriptedResponse,
                        sse::responses_api_script_exact,
                    };
                    use serde_json::json;
                    use std::sync::atomic::Ordering;

                    let server = MockInferenceServer::start_with_models(vec![
                        MockModelEntry::new("gpt-6-luna").with_api_backend("responses"),
                    ])
                    .await
                    .unwrap();
                    let fixture = NamedAccounts::new(&server.url());
                    fixture.routing(crate::accounts::AutoStrategy::HighestRemaining, true);
                    fixture.quota("one", Some(10.0));
                    fixture.quota("two", Some(30.0));
                    fixture.quota("three", Some(80.0));
                    let mut routing = fixture.store.load_routing().unwrap();
                    routing.options.iter_mut().find(|option| option.id == "two").unwrap()
                        .worker_model = Some("gpt-6-luna".into());
                    fixture.store.save_routing(&routing).unwrap();

                    let auth_home = tempfile::tempdir().unwrap();
                    let auth = std::sync::Arc::new(distill_login::AuthManager::new(
                        auth_home.path(), distill_login::GrokComConfig::default(),
                    ));
                    let (gateway_tx, mut gateway_rx) = tokio::sync::mpsc::unbounded_channel();
                    tokio::task::spawn_local(async move {
                        while let Some(message) = gateway_rx.recv().await {
                            if let distill_acp_lib::AcpClientMessage::SessionNotification(args) = message {
                                let _ = args.response_tx.send(Ok(()));
                            }
                        }
                    });
                    let agent = std::rc::Rc::new(MvpAgent::new(
                        GatewaySender::new(gateway_tx), &config::Config::default(), auth, None, None,
                    ).unwrap());
                    // initialize_request is set below without the public initialize lifecycle.
                    // A real prompt still needs the coordinator to answer its completion drain.
                    agent.start_subagent_coordinator();
                    for (id, model) in &fixture.models {
                        let mut model = model.clone();
                        model.info.supports_reasoning_effort = true;
                        model.info.reasoning_effort = Some(ReasoningEffort::High);
                        agent.models_manager.insert_test_entry(id, model);
                    }
                    agent.initialize_request.set(
                        acp::InitializeRequest::new(acp::ProtocolVersion::V1).client_capabilities(
                            acp::ClientCapabilities::new()
                                .fs(acp::FileSystemCapabilities::new()).terminal(false),
                        ),
                    ).unwrap();
                    let cwd = tempfile::tempdir().unwrap();
                    let sid = agent.new_session_inner(
                        acp::NewSessionRequest::new(cwd.path().to_path_buf())
                            .meta(json!({"modelId":"auto","workerModelId":"gpt-6.1-sol"}).as_object().cloned()),
                    ).await.unwrap().session_id;
                    let handle = agent.resident_handle(&sid).unwrap();

                    let mut events = responses_api_script_exact("retained account A answer", "gpt-6-luna");
                    assert_eq!(events.pop().unwrap().data, "[DONE]");
                    let mut terminal = events.pop().unwrap();
                    let mut completed: serde_json::Value = serde_json::from_str(&terminal.data).unwrap();
                    let output = completed["response"]["output"].as_array_mut().unwrap();
                    output.insert(0, json!({
                        "type":"web_search_call", "id":"active-a-backend", "status":"completed",
                        "action":{"type":"search", "query":"retained account A search"},
                    }));
                    output.insert(0, json!({
                        "type":"reasoning", "id":"active-a-reason", "summary":[],
                        "encrypted_content":"active-a-secret", "status":"completed",
                    }));
                    terminal.data = completed.to_string();
                    // Block response.completed itself; no response items arrive before release.
                    let matcher = InferenceRequestMatcher::foreground(InferenceEndpoint::Responses);
                    let mut held = server.expect_response_blocked(
                        "account A response", matcher,
                        ScriptedResponse::sse(vec![events.remove(0), terminal]),
                    );
                    let first_prompt = tokio::task::spawn_local({
                        let agent = agent.clone();
                        let sid = sid.clone();
                        async move {
                            agent.prompt(acp::PromptRequest::new(sid, vec![
                                acp::ContentBlock::Text(acp::TextContent::new("retained human question")),
                            ]).meta(json!({"verbatim":true}).as_object().cloned())).await
                        }
                    });
                    held.wait_blocked().await;
                    assert!(!first_prompt.is_finished());
                    let first_request = server.requests().into_iter()
                        .find(|request| request.path == "/v1/responses" && request.conversation.is_some())
                        .unwrap();
                    assert_eq!(first_request.authorization.as_deref(), Some("Bearer three-token"));

                    // An effort-only change keeps the same authority and remains allowed.
                    Box::pin(agent.set_model_gated(
                        acp::SetSessionModelRequest::new(sid.clone(), acp::ModelId::new("gpt-6-luna"))
                            .meta(json!({"reasoningEffort":"low"}).as_object().cloned()),
                    )).await.expect("same-account effort changes remain available during a turn");
                    let current = agent.resident_handle(&sid).unwrap();
                    let before_config = current.chat_state_handle.get_sampling_config().await.unwrap();
                    assert_eq!(before_config.reasoning_effort, Some(ReasoningEffort::Low));
                    assert_eq!(AccountRoute::read(&before_config.extra_headers).unwrap().unwrap().account_id, "three");
                    let before_worker = current.worker_override.read().clone();
                    assert_eq!(before_worker.as_ref().unwrap().model_id.as_deref(), Some("gpt-6.1-sol"));
                    assert!(!current.jev_effort_auto.load(Ordering::Relaxed));
                    let before_default_effort_auto = agent.models_manager.current_effort_auto();
                    let (tx, rx) = tokio::sync::oneshot::channel();
                    current.cmd_tx.send(crate::session::SessionCommand::GetActiveAgent { responds_to: tx }).unwrap();
                    let before_harness = rx.await.unwrap();
                    let before_history = current.chat_state_handle.get_conversation().await;
                    assert!(!before_history.iter().any(|item| matches!(
                        item, ConversationItem::Reasoning(_) | ConversationItem::BackendToolCall(_)
                    )));
                    drain_persistence(&agent, &sid).await;
                    let dir = crate::session::persistence::session_dir(&current.info);
                    let before_files = ["account-route.json", "chat_history.jsonl", "system_prompt.txt", "prompt_context.json"]
                        .map(|name| (name, std::fs::read(dir.join(name)).unwrap()));
                    fixture.quota("two", Some(90.0));
                    let auto_request = || acp::SetSessionModelRequest::new(sid.clone(), acp::ModelId::new("auto"))
                        .meta(json!({"reasoningEffortAuto":true}).as_object().cloned());
                    let rejected = Box::pin(agent.set_model_gated(auto_request())).await
                        .expect_err("account handoff must wait for the active response");
                    assert_eq!(rejected.data, Some(json!("Cannot change the account or model while a turn is running")));
                    let unchanged = agent.resident_handle(&sid).unwrap();
                    assert_eq!(unchanged.model_id, current.model_id);
                    assert_eq!(unchanged.agent_name, current.agent_name);
                    assert_eq!(*unchanged.worker_override.read(), before_worker);
                    assert!(!unchanged.jev_effort_auto.load(Ordering::Relaxed));
                    assert_eq!(agent.models_manager.current_effort_auto(), before_default_effort_auto);
                    assert_eq!(
                        serde_json::to_value(unchanged.chat_state_handle.get_sampling_config().await.unwrap()).unwrap(),
                        serde_json::to_value(&before_config).unwrap(),
                    );
                    let (tx, rx) = tokio::sync::oneshot::channel();
                    unchanged.cmd_tx.send(crate::session::SessionCommand::GetActiveAgent { responds_to: tx }).unwrap();
                    assert_eq!(rx.await.unwrap(), before_harness);
                    assert!(
                        serde_json::to_value(unchanged.chat_state_handle.get_conversation().await).unwrap()
                            == serde_json::to_value(&before_history).unwrap(),
                        "rejected handoff changed live history",
                    );
                    drain_persistence(&agent, &sid).await;
                    for (name, bytes) in before_files {
                        assert!(std::fs::read(dir.join(name)).unwrap() == bytes, "{name} changed after rejection");
                    }
                    assert!(!first_prompt.is_finished());
                    held.release();
                    assert_eq!(first_prompt.await.unwrap().unwrap().stop_reason, acp::StopReason::EndTurn);
                    held.assert_satisfied();
                    let after_response = handle.chat_state_handle.get_conversation().await;
                    assert!(after_response.iter().any(|item| matches!(
                        item, ConversationItem::Reasoning(reason) if reason.encrypted_content.as_deref() == Some("active-a-secret")
                    )), "the late response must actually append account A's opaque reasoning");
                    assert!(after_response.iter().any(|item| matches!(
                        item, ConversationItem::BackendToolCall(call) if call.id() == "active-a-backend"
                    )), "the late response must actually append account A's backend handle");

                    Box::pin(agent.set_model_gated(auto_request())).await
                        .expect("the same account handoff succeeds after the turn finishes");
                    let switched = agent.resident_handle(&sid).unwrap();
                    let config = switched.chat_state_handle.get_sampling_config().await.unwrap();
                    assert_eq!(config.model, "gpt-6-luna");
                    assert_eq!(AccountRoute::read(&config.extra_headers).unwrap().unwrap().account_id, "two");
                    assert!(switched.jev_effort_auto.load(Ordering::Relaxed));
                    assert_eq!(switched.worker_override.read().as_ref().unwrap().model_id.as_deref(), Some("gpt-6-luna"));
                    assert!(!switched.chat_state_handle.get_conversation().await.iter().any(|item| matches!(
                        item, ConversationItem::Reasoning(_) | ConversationItem::BackendToolCall(_)
                    )));
                    let second = server.expect_response(
                        "account B response", matcher,
                        ScriptedResponse::sse(responses_api_script_exact("account B done", "gpt-6-luna")),
                    );
                    let result = agent.prompt(acp::PromptRequest::new(sid.clone(), vec![
                        acp::ContentBlock::Text(acp::TextContent::new("continue with retained context")),
                    ]).meta(json!({"verbatim":true}).as_object().cloned())).await.unwrap();
                    assert_eq!(result.stop_reason, acp::StopReason::EndTurn);
                    second.assert_satisfied();
                    let requests: Vec<_> = server.requests().into_iter()
                        .filter(|request| request.path == "/v1/responses" && request.conversation.is_some())
                        .collect();
                    assert_eq!(requests.len(), 2);
                    assert_eq!(requests[1].authorization.as_deref(), Some("Bearer two-token"));
                    assert_eq!(requests[1].header("chatgpt-account-id"), Some("wire-two"));
                    let wire = requests[1].body.as_ref().unwrap().to_string();
                    for opaque in ["active-a-secret", "active-a-reason", "active-a-backend"] {
                        assert!(!wire.contains(opaque), "account A opaque state replayed: {opaque}");
                    }
                    for ordinary in ["retained human question", "retained account A answer", "retained account A search"] {
                        assert!(wire.contains(ordinary), "lost ordinary context: {ordinary}");
                    }
                    agent.remove_session(&sid);
                }).await.expect("delayed-response account handoff finishes within the fixture deadline");
            });
        },
    );
}

/// RemoteCode follows every `session/new` and `session/load` with `session/set_model` for the
/// SAME model, carrying only the effort. An explicit model admitted without an account route
/// (here: its own API key) is pinned by `PIN_HEADER` alone, so losing the header on that bind
/// would let global Auto take the session over from the next turn: another model, another account.
#[test]
#[serial_test::serial]
fn account_routing_effort_bind_keeps_routeless_explicit_model_pinned() {
    crate::agent::account_routing::tests::isolated(
        "agent::mvp_agent::tests::account_routing_tests::account_routing_effort_bind_keeps_routeless_explicit_model_pinned",
        || {
            run_local_for_bridge_test(|| async {
                let fixture = NamedAccounts::new("http://127.0.0.1:1/v1");
                fixture.routing(crate::accounts::AutoStrategy::OrderedFirst, true);
                let agent = build_minimal_agent_for_tests();
                for (id, model) in &fixture.models {
                    agent.models_manager.insert_test_entry(id, model.clone());
                }
                let mut own_key = fixture.models["gpt-6-luna"].clone();
                own_key.info.model = "own-key-model".into();
                own_key.api_key = Some("own-key".into());
                own_key.info.supports_reasoning_effort = true;
                own_key.info.reasoning_effort = Some(ReasoningEffort::High);
                agent
                    .models_manager
                    .insert_test_entry("own-key-model", own_key);
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
                let sid = agent
                    .new_session_inner(
                        acp::NewSessionRequest::new(cwd.path().to_path_buf()).meta(
                            serde_json::json!({"modelId":"own-key-model"})
                                .as_object()
                                .cloned(),
                        ),
                    )
                    .await
                    .expect("an own-key model needs no account route")
                    .session_id;
                let models = agent.models_manager.models();
                // The session's stored config, and the routes its next turn would take:
                // a turn rebuilds its sampler config from exactly these two stores.
                let next_turn = async || {
                    let handle = agent.resident_handle(&sid).unwrap();
                    let stored = handle
                        .chat_state_handle
                        .get_sampling_config()
                        .await
                        .unwrap();
                    let turn = distill_sampler::SamplerConfig {
                        api_key: handle.chat_state_handle.get_credentials().await.api_key,
                        base_url: stored.base_url.clone(),
                        model: stored.model.clone(),
                        api_backend: stored.api_backend.clone(),
                        extra_headers: stored.extra_headers.clone(),
                        ..Default::default()
                    };
                    let routes =
                        account_routing::routes_for_turn(&fixture.store, &models, &turn, true)
                            .await
                            .unwrap();
                    (stored, turn.api_key, routes)
                };
                let handle = agent.resident_handle(&sid).unwrap();
                assert_eq!(handle.model_id.0.as_ref(), "own-key-model");
                let (admitted, api_key, routes) = next_turn().await;
                assert_eq!(api_key.as_deref(), Some("own-key"));
                assert!(
                    admitted
                        .extra_headers
                        .contains_key(account_routing::PIN_HEADER)
                );
                assert!(
                    AccountRoute::read(&admitted.extra_headers)
                        .unwrap()
                        .is_none()
                        && account_routing::load_session_config(&handle.info, &models)
                            .await
                            .unwrap()
                            .is_none(),
                    "the header is this session's only pin"
                );
                assert!(routes.is_empty());

                for (meta, effort) in [
                    (
                        serde_json::json!({"reasoningEffort":"low","reasoningEffortAuto":false}),
                        ReasoningEffort::Low,
                    ),
                    (
                        serde_json::json!({"reasoningEffortAuto":false}),
                        ReasoningEffort::Low,
                    ),
                    (
                        serde_json::json!({"reasoningEffortAuto":true}),
                        ReasoningEffort::Low,
                    ),
                    (
                        serde_json::json!({"reasoningEffort":"high"}),
                        ReasoningEffort::High,
                    ),
                ] {
                    let acknowledged = Box::pin(
                        agent.set_model_gated(
                            acp::SetSessionModelRequest::new(
                                sid.clone(),
                                acp::ModelId::new("own-key-model"),
                            )
                            .meta(meta.as_object().cloned()),
                        ),
                    )
                    .await
                    .expect("an effort bind for the session's own model");
                    assert_eq!(
                        acknowledged.meta.unwrap()["canonicalModelId"],
                        serde_json::json!("own-key-model")
                    );
                    let (bound, api_key, routes) = next_turn().await;
                    assert_eq!(bound.model, "own-key-model");
                    assert_eq!(bound.reasoning_effort, Some(effort), "{meta} was applied");
                    assert_eq!(api_key.as_deref(), Some("own-key"));
                    assert!(
                        bound
                            .extra_headers
                            .contains_key(account_routing::PIN_HEADER),
                        "{meta} dropped the explicit model's pin"
                    );
                    assert!(
                        routes.is_empty(),
                        "{meta} handed the explicit model to Auto: {routes:?}"
                    );
                }

                // Choosing Auto is a model choice: it must still release the pin.
                Box::pin(agent.set_model_gated(acp::SetSessionModelRequest::new(
                    sid.clone(),
                    acp::ModelId::new("auto"),
                )))
                .await
                .expect("explicit Auto replaces the pinned own-key model");
                let (auto, api_key, routes) = next_turn().await;
                assert!(!auto.extra_headers.contains_key(account_routing::PIN_HEADER));
                let selected = AccountRoute::read(&auto.extra_headers).unwrap().unwrap();
                assert_eq!(selected.option_id.as_deref(), Some("one"));
                assert!(!selected.pinned);
                assert!(api_key.is_none(), "Auto never sends the own key");
                assert_eq!(
                    routes[0].option_id.as_deref(),
                    Some("one"),
                    "the next turn stays on Auto"
                );
                agent.remove_session(&sid);
            });
        },
    );
}
