//! Negotiated ACP readiness and operation evidence for MacCode's stdio client.
//! Status requests run on the same local executor as ACP, never on a timer thread.
use super::mvp_agent::MvpAgent;
use acp::Agent as _;
use agent_client_protocol as acp;
use distill_acp_lib::{AcpAgentGatewaySender, AcpClientMessage};
use futures::future::LocalBoxFuture;
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    rc::Rc,
    time::Instant,
};

pub(super) const CAPABILITY: &str = "remotecode/operationStatusV1";

struct Evidence {
    session_id: Option<String>,
    phase: &'static str,
    tools: HashSet<String>,
    permissions: HashSet<String>,
    sequence: u64,
    changed: Instant,
    progressed: Instant,
}

#[derive(Clone)]
pub(super) struct Operations {
    entries: Rc<RefCell<HashMap<String, Evidence>>>,
    gateway: AcpAgentGatewaySender,
}

impl Operations {
    pub(super) fn new(gateway: AcpAgentGatewaySender) -> Self {
        Self {
            entries: Default::default(),
            gateway,
        }
    }

    fn begin(
        &self,
        meta: Option<&acp::Meta>,
        session: Option<&acp::SessionId>,
        phase: &'static str,
    ) -> Progress {
        let id = meta
            .and_then(|m| m.get("remotecode/operationId"))
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty() && id.len() <= 128)
            .map(str::to_owned);
        if let Some(id) = &id {
            let now = Instant::now();
            self.entries.borrow_mut().insert(
                id.clone(),
                Evidence {
                    session_id: session.map(|s| s.0.to_string()),
                    phase,
                    tools: HashSet::new(),
                    permissions: HashSet::new(),
                    sequence: 1,
                    changed: now,
                    progressed: now,
                },
            );
            self.publish(id);
        }
        Progress {
            operations: self.clone(),
            id,
        }
    }

    fn snapshot(&self, id: &str) -> Value {
        let entries = self.entries.borrow();
        let Some(e) = entries.get(id) else {
            return json!({"operationId": id, "state": "finished"});
        };
        // A heartbeat reads these clocks; it never changes them.
        let limit = match e.phase {
            "executing_tool" => 600_000,
            "waiting_authorization" => 300_000,
            "waiting_model" => 180_000,
            _ => 60_000,
        };
        json!({"operationId": id, "sessionId": e.session_id, "state": "running",
            "phase": e.phase, "sequence": e.sequence,
            "phaseElapsedMs": e.changed.elapsed().as_millis() as u64,
            "lastProgressAgeMs": e.progressed.elapsed().as_millis() as u64,
            "stallAfterMs": limit})
    }

    fn publish(&self, id: &str) {
        if let Ok(raw) = serde_json::value::to_raw_value(&self.snapshot(id)) {
            self.gateway
                .forward_fire_and_forget(acp::ExtNotification::new(
                    "remotecode/operation",
                    raw.into(),
                ));
        }
    }

    fn advance(&self, id: &str, phase: &'static str) {
        let changed = {
            let mut entries = self.entries.borrow_mut();
            let Some(e) = entries.get_mut(id) else {
                return;
            };
            let changed = e.phase != phase;
            e.sequence += 1;
            e.progressed = Instant::now();
            if changed {
                e.phase = phase;
                e.changed = e.progressed;
            }
            changed
        };
        // Token output updates evidence but does not double the token stream.
        if changed {
            self.publish(id);
        }
    }

    pub(super) fn observe(&self, message: &AcpClientMessage) {
        match message {
            AcpClientMessage::SessionNotification(args) => {
                let update = &args.request.update;
                if !matches!(
                    update,
                    acp::SessionUpdate::ToolCall(_)
                        | acp::SessionUpdate::ToolCallUpdate(_)
                        | acp::SessionUpdate::AgentMessageChunk(_)
                        | acp::SessionUpdate::AgentThoughtChunk(_)
                ) {
                    return;
                }
                self.session_progress(&args.request.session_id, |e| match update {
                    acp::SessionUpdate::ToolCall(tool) => {
                        if matches!(
                            tool.status,
                            acp::ToolCallStatus::Completed | acp::ToolCallStatus::Failed
                        ) {
                            e.tools.remove(tool.tool_call_id.0.as_ref());
                        } else {
                            e.tools.insert(tool.tool_call_id.0.to_string());
                        }
                    }
                    acp::SessionUpdate::ToolCallUpdate(tool) => {
                        if matches!(
                            tool.fields.status,
                            Some(acp::ToolCallStatus::Completed | acp::ToolCallStatus::Failed)
                        ) {
                            e.tools.remove(tool.tool_call_id.0.as_ref());
                        } else {
                            e.tools.insert(tool.tool_call_id.0.to_string());
                        }
                    }
                    _ => {}
                });
            }
            AcpClientMessage::RequestPermission(args) => {
                self.session_progress(&args.request.session_id, |e| {
                    e.permissions
                        .insert(args.request.tool_call.tool_call_id.0.to_string());
                });
            }
            _ => {}
        }
    }

    pub(super) fn permission_resolved(&self, session: &acp::SessionId, tool: &acp::ToolCallId) {
        self.session_progress(session, |e| {
            e.permissions.remove(tool.0.as_ref());
        });
    }

    fn session_progress(&self, session: &acp::SessionId, update: impl Fn(&mut Evidence)) {
        let changes: Vec<_> = self
            .entries
            .borrow_mut()
            .iter_mut()
            .filter(|(_, e)| e.session_id.as_deref() == Some(session.0.as_ref()))
            .map(|(id, e)| {
                update(e);
                let phase = if !e.permissions.is_empty() {
                    "waiting_authorization"
                } else if !e.tools.is_empty() {
                    "executing_tool"
                } else {
                    "waiting_model"
                };
                (id.clone(), phase)
            })
            .collect();
        for (id, phase) in changes {
            self.advance(&id, phase);
        }
    }
}

pub(super) struct Progress {
    operations: Operations,
    id: Option<String>,
}
impl Progress {
    pub(super) fn phase(&self, phase: &'static str) {
        if let Some(id) = &self.id {
            self.operations.advance(id, phase);
        }
    }
}
impl Drop for Progress {
    fn drop(&mut self) {
        if let Some(id) = &self.id {
            self.operations.entries.borrow_mut().remove(id);
        }
    }
}

type Factory =
    Box<dyn FnOnce(Progress) -> LocalBoxFuture<'static, Result<(MvpAgent, Progress), acp::Error>>>;

pub(super) struct StdioAgent {
    factory: RefCell<Option<Factory>>,
    ready: tokio::sync::OnceCell<Result<(MvpAgent, acp::InitializeResponse), acp::Error>>,
    initialize: RefCell<Option<acp::InitializeRequest>>,
    negotiated: Cell<bool>,
    operations: Operations,
}
impl StdioAgent {
    pub(super) fn new(operations: Operations, factory: Factory) -> Self {
        Self {
            factory: RefCell::new(Some(factory)),
            ready: Default::default(),
            initialize: Default::default(),
            negotiated: Cell::new(false),
            operations,
        }
    }
    async fn ready(
        &self,
        progress: Progress,
    ) -> Result<&(MvpAgent, acp::InitializeResponse), acp::Error> {
        let args = self
            .initialize
            .borrow()
            .clone()
            .ok_or_else(acp::Error::invalid_request)?;
        self.ready
            .get_or_init(|| async {
                let factory = self
                    .factory
                    .borrow_mut()
                    .take()
                    .ok_or_else(acp::Error::internal_error)?;
                let (agent, _progress) = factory(progress).await?;
                let response = agent.initialize(args).await?;
                Ok((agent, response))
            })
            .await
            .as_ref()
            .map_err(Clone::clone)
    }
    async fn agent(&self) -> Result<&MvpAgent, acp::Error> {
        let progress = self.operations.begin(None, None, "loading_configuration");
        self.ready(progress).await.map(|r| &r.0)
    }
}

#[async_trait::async_trait(?Send)]
impl acp::Agent for StdioAgent {
    async fn initialize(
        &self,
        args: acp::InitializeRequest,
    ) -> Result<acp::InitializeResponse, acp::Error> {
        if self.initialize.borrow().is_some() {
            return Err(acp::Error::invalid_request());
        }
        let negotiated = args
            .client_capabilities
            .meta
            .as_ref()
            .and_then(|m| m.get(CAPABILITY))
            .and_then(Value::as_bool)
            == Some(true);
        self.negotiated.set(negotiated);
        *self.initialize.borrow_mut() = Some(args);
        if negotiated {
            Ok(acp::InitializeResponse::new(acp::ProtocolVersion::V1).meta(
                json!({CAPABILITY: true, "remotecode/readyRequired": true,
                    "agentVersion": distill_version::VERSION})
                .as_object()
                .cloned(),
            ))
        } else {
            Ok(self
                .ready(self.operations.begin(None, None, "loading_configuration"))
                .await?
                .1
                .clone())
        }
    }
    async fn ext_method(&self, args: acp::ExtRequest) -> Result<acp::ExtResponse, acp::Error> {
        if self.negotiated.get() {
            let params: Value = serde_json::from_str(args.params.get())
                .map_err(|_| acp::Error::invalid_params())?;
            match args.method.as_ref() {
                "remotecode/status" => {
                    let id = params
                        .get("operationId")
                        .and_then(Value::as_str)
                        .ok_or_else(acp::Error::invalid_params)?;
                    return Ok(acp::ExtResponse::new(
                        serde_json::value::to_raw_value(&self.operations.snapshot(id))
                            .unwrap()
                            .into(),
                    ));
                }
                "remotecode/ready" => {
                    let progress = self.operations.begin(
                        params.get("_meta").and_then(Value::as_object),
                        None,
                        "loading_configuration",
                    );
                    let response = &self.ready(progress).await?.1;
                    return Ok(acp::ExtResponse::new(
                        serde_json::value::to_raw_value(response).unwrap().into(),
                    ));
                }
                _ => {}
            }
        }
        self.agent().await?.ext_method(args).await
    }
    async fn authenticate(
        &self,
        args: acp::AuthenticateRequest,
    ) -> Result<acp::AuthenticateResponse, acp::Error> {
        self.agent().await?.authenticate(args).await
    }
    async fn new_session(
        &self,
        args: acp::NewSessionRequest,
    ) -> Result<acp::NewSessionResponse, acp::Error> {
        let _progress = self
            .operations
            .begin(args.meta.as_ref(), None, "preparing_session");
        self.agent().await?.new_session(args).await
    }
    async fn load_session(
        &self,
        args: acp::LoadSessionRequest,
    ) -> Result<acp::LoadSessionResponse, acp::Error> {
        let _progress = self.operations.begin(
            args.meta.as_ref(),
            Some(&args.session_id),
            "preparing_session",
        );
        self.agent().await?.load_session(args).await
    }
    async fn resume_session(
        &self,
        args: acp::ResumeSessionRequest,
    ) -> Result<acp::ResumeSessionResponse, acp::Error> {
        let _progress = self.operations.begin(
            args.meta.as_ref(),
            Some(&args.session_id),
            "preparing_session",
        );
        self.agent().await?.resume_session(args).await
    }
    async fn list_sessions(
        &self,
        args: acp::ListSessionsRequest,
    ) -> Result<acp::ListSessionsResponse, acp::Error> {
        self.agent().await?.list_sessions(args).await
    }
    async fn close_session(
        &self,
        args: acp::CloseSessionRequest,
    ) -> Result<acp::CloseSessionResponse, acp::Error> {
        self.agent().await?.close_session(args).await
    }
    async fn set_session_mode(
        &self,
        args: acp::SetSessionModeRequest,
    ) -> Result<acp::SetSessionModeResponse, acp::Error> {
        self.agent().await?.set_session_mode(args).await
    }
    async fn set_session_model(
        &self,
        args: acp::SetSessionModelRequest,
    ) -> Result<acp::SetSessionModelResponse, acp::Error> {
        self.agent().await?.set_session_model(args).await
    }
    async fn set_session_config_option(
        &self,
        args: acp::SetSessionConfigOptionRequest,
    ) -> Result<acp::SetSessionConfigOptionResponse, acp::Error> {
        self.agent().await?.set_session_config_option(args).await
    }
    async fn prompt(&self, args: acp::PromptRequest) -> Result<acp::PromptResponse, acp::Error> {
        let _progress =
            self.operations
                .begin(args.meta.as_ref(), Some(&args.session_id), "waiting_model");
        self.agent().await?.prompt(args).await
    }
    async fn cancel(&self, args: acp::CancelNotification) -> Result<(), acp::Error> {
        self.agent().await?.cancel(args).await
    }
    async fn ext_notification(&self, args: acp::ExtNotification) -> Result<(), acp::Error> {
        self.agent().await?.ext_notification(args).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn operations() -> Operations {
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        Operations::new(AcpAgentGatewaySender::new(tx))
    }

    #[tokio::test(flavor = "current_thread")]
    async fn initialize_answers_before_readiness_and_status_does_not_advance_work() {
        let ops = operations();
        let factory_started = Rc::new(Cell::new(false));
        let started = factory_started.clone();
        let agent = StdioAgent::new(
            ops.clone(),
            Box::new(move |progress| {
                Box::pin(async move {
                    started.set(true);
                    progress.phase("waiting_model");
                    futures::future::pending().await
                })
            }),
        );
        let init = acp::InitializeRequest::new(acp::ProtocolVersion::V1).client_capabilities(
            acp::ClientCapabilities::new().meta(json!({CAPABILITY: true}).as_object().cloned()),
        );
        let response = tokio::time::timeout(Duration::from_millis(100), agent.initialize(init))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(response.meta.unwrap()["remotecode/readyRequired"], true);
        assert!(
            !factory_started.get(),
            "communication precedes dependency preparation"
        );
        let ready = agent.ext_method(acp::ExtRequest::new(
            "remotecode/ready",
            serde_json::value::to_raw_value(&json!({"_meta":{"remotecode/operationId":"boot"}}))
                .unwrap()
                .into(),
        ));
        tokio::pin!(ready);
        assert!(
            tokio::time::timeout(Duration::from_millis(10), &mut ready)
                .await
                .is_err()
        );
        assert!(factory_started.get());
        let first = ops.snapshot("boot");
        let probe = || {
            acp::ExtRequest::new(
                "remotecode/status",
                serde_json::value::to_raw_value(&json!({"operationId":"boot"}))
                    .unwrap()
                    .into(),
            )
        };
        let a = agent.ext_method(probe()).await.unwrap();
        tokio::time::sleep(Duration::from_millis(5)).await;
        let b = agent.ext_method(probe()).await.unwrap();
        let a: Value = serde_json::from_str(a.0.get()).unwrap();
        let b: Value = serde_json::from_str(b.0.get()).unwrap();
        assert_eq!(a["phase"], "waiting_model");
        assert_eq!(a["sequence"], b["sequence"]);
        assert!(
            b["lastProgressAgeMs"].as_u64().unwrap() >= a["lastProgressAgeMs"].as_u64().unwrap()
        );
        assert_eq!(first["sequence"], b["sequence"]);
        let new_session = agent.new_session(acp::NewSessionRequest::new(std::env::temp_dir()));
        assert!(
            tokio::time::timeout(Duration::from_millis(10), new_session)
                .await
                .is_err(),
            "sessions cannot bypass unfinished dependencies"
        );
    }

    #[test]
    fn real_progress_is_scoped_to_the_current_session_and_lease() {
        let ops = operations();
        let meta = json!({"remotecode/operationId":"a"});
        let progress = ops.begin(
            meta.as_object(),
            Some(&acp::SessionId::new("session-a")),
            "executing_tool",
        );
        let emit_tool = |update| {
            let (tx, _rx) = tokio::sync::oneshot::channel();
            ops.observe(&AcpClientMessage::SessionNotification(
                distill_acp_lib::AcpArgs {
                    request: acp::SessionNotification::new(
                        acp::SessionId::new("session-a"),
                        update,
                    ),
                    response_tx: tx,
                },
            ));
        };
        emit_tool(acp::SessionUpdate::ToolCall(acp::ToolCall::new(
            "one", "one",
        )));
        emit_tool(acp::SessionUpdate::ToolCall(acp::ToolCall::new(
            "two", "two",
        )));
        emit_tool(acp::SessionUpdate::ToolCallUpdate(
            acp::ToolCallUpdate::new(
                "one",
                acp::ToolCallUpdateFields::new().status(acp::ToolCallStatus::Completed),
            ),
        ));
        assert_eq!(
            ops.snapshot("a")["phase"],
            "executing_tool",
            "another tool still owns work"
        );
        let old = ops.snapshot("a");
        let (tx, _rx) = tokio::sync::oneshot::channel();
        let notification = acp::SessionNotification::new(
            acp::SessionId::new("session-b"),
            acp::SessionUpdate::AgentMessageChunk(acp::ContentChunk::new(acp::ContentBlock::Text(
                acp::TextContent::new("other"),
            ))),
        );
        ops.observe(&AcpClientMessage::SessionNotification(
            distill_acp_lib::AcpArgs {
                request: notification,
                response_tx: tx,
            },
        ));
        assert_eq!(ops.snapshot("a")["sequence"], old["sequence"]);
        progress.phase("waiting_authorization");
        assert_eq!(ops.snapshot("a")["phase"], "waiting_authorization");
        assert!(
            ops.snapshot("a")["sequence"].as_u64().unwrap() > old["sequence"].as_u64().unwrap()
        );
        drop(progress);
        assert_eq!(ops.snapshot("a")["state"], "finished");
    }
}
