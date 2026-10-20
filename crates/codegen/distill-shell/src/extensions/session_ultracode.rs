// Modified for Distill by Samuel Fajreldines, 2026.
//! Exclusive session execution mode; the old UltraCode endpoint remains an alias.
use agent_client_protocol as acp;
use distill_tool_types::{ExecutionMode, ExecutionModeOperation};
use serde::{Deserialize, Serialize};

use super::{ExtResult, parse_params, to_raw_response};
use crate::agent::MvpAgent;

pub const SET_METHOD: &str = "x.ai/session/ultracode/set";
pub const MODE_SET_METHOD: &str = "x.ai/session/execution_mode/set";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetModeRequest {
    session_id: String,
    mode: ExecutionMode,
    #[serde(default)]
    operation: ExecutionModeOperation,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetSessionUltracodeRequest {
    session_id: String,
    enabled: Option<bool>,
}

#[derive(Serialize, Deserialize)]
struct ModeState {
    mode: ExecutionMode,
}

#[derive(Serialize, Deserialize)]
struct SetSessionUltracodeResponse {
    enabled: bool,
}

pub async fn handle(agent: &MvpAgent, args: &acp::ExtRequest) -> ExtResult {
    let (session_id, requested_mode, enabled) = match args.method.as_ref() {
        MODE_SET_METHOD => {
            let request: SetModeRequest = parse_params(args)?;
            (
                request.session_id,
                Some((request.mode, request.operation)),
                None,
            )
        }
        SET_METHOD => {
            let request: SetSessionUltracodeRequest = parse_params(args)?;
            (request.session_id, None, request.enabled)
        }
        _ => return Err(acp::Error::method_not_found()),
    };
    let Some(handle) = agent
        .session_handle_waiting_for_load(&acp::SessionId::new(session_id.as_str()))
        .await
    else {
        return Err(acp::Error::resource_not_found(Some(format!(
            "session not found: {session_id}"
        ))));
    };
    if handle.tool_context.subagent_depth > 0 {
        return Err(acp::Error::invalid_params()
            .data("Execution mode can only be changed on the root session"));
    }
    let current = handle.execution_mode.load();
    let mode = match requested_mode {
        Some((target, operation)) => match operation {
            ExecutionModeOperation::Set => target,
            ExecutionModeOperation::Toggle if current != target => target,
            ExecutionModeOperation::Toggle | ExecutionModeOperation::Disable
                if current == target =>
            {
                ExecutionMode::Normal
            }
            _ => current,
        },
        None => {
            if enabled.unwrap_or(current != ExecutionMode::Ultracode) {
                ExecutionMode::Ultracode
            } else if current == ExecutionMode::Ultracode {
                ExecutionMode::Normal
            } else {
                current
            }
        }
    };
    // Persist before acknowledging. There is no await between read and commit.
    persist_mode(&handle.info, mode)
        .map_err(|error| acp::Error::internal_error().data(error.to_string()))?;
    handle.execution_mode.store(mode);
    if requested_mode.is_some() {
        to_raw_response(&ModeState { mode })
    } else {
        to_raw_response(&SetSessionUltracodeResponse {
            enabled: mode == ExecutionMode::Ultracode,
        })
    }
}

pub(crate) fn load_mode(info: &crate::session::info::Info) -> std::io::Result<ExecutionMode> {
    let dir = crate::session::persistence::session_dir(info);
    match std::fs::read(dir.join("execution_mode.json")) {
        Ok(bytes) => serde_json::from_slice::<ModeState>(&bytes)
            .map(|state| state.mode)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            // Sessions saved before exclusive modes keep their existing selection.
            match std::fs::read(dir.join("ultracode.json")) {
                Ok(bytes) => serde_json::from_slice::<SetSessionUltracodeResponse>(&bytes)
                    .map(|state| {
                        if state.enabled {
                            ExecutionMode::Ultracode
                        } else {
                            ExecutionMode::Normal
                        }
                    })
                    .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error)),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    Ok(ExecutionMode::Normal)
                }
                Err(error) => Err(error),
            }
        }
        Err(error) => Err(error),
    }
}

pub(crate) fn persist_mode(
    info: &crate::session::info::Info,
    mode: ExecutionMode,
) -> std::io::Result<()> {
    let dir = crate::session::persistence::ensure_owner_only_session_dir(info)?;
    let bytes = serde_json::to_vec(&ModeState { mode })
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    crate::session::storage::write_bytes_atomic(&dir.join("execution_mode.json"), &bytes)
}
