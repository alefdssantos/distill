// Modified for Distill by Samuel Fajreldines, 2026.
//! Exclusive session execution mode; the old UltraCode endpoint remains an alias.
use agent_client_protocol as acp;
use distill_tool_types::{ExecutionMode, ExecutionModeChoice, ExecutionModeOperation};
use serde::{Deserialize, Serialize};

use super::{ExtResult, parse_params, to_raw_response};
use crate::agent::MvpAgent;

pub const SET_METHOD: &str = "x.ai/session/ultracode/set";
pub const MODE_SET_METHOD: &str = "x.ai/session/execution_mode/set";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetModeRequest {
    session_id: String,
    mode: ExecutionModeChoice,
    #[serde(default)]
    operation: ExecutionModeOperation,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetSessionUltracodeRequest {
    session_id: String,
    enabled: Option<bool>,
}

/// `mode` is the effective mode; `choice` is absent in files written before Auto.
#[derive(Serialize, Deserialize)]
struct ModeState {
    mode: ExecutionMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    choice: Option<ExecutionModeChoice>,
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
    let current_choice = handle.execution_mode.choice();
    let choice = match requested_mode {
        Some((target, operation)) => match operation {
            ExecutionModeOperation::Set => target,
            ExecutionModeOperation::Toggle if current_choice != target => target,
            ExecutionModeOperation::Toggle | ExecutionModeOperation::Disable
                if current_choice == target =>
            {
                ExecutionModeChoice::Auto
            }
            _ => current_choice,
        },
        None => {
            if enabled.unwrap_or(current != ExecutionMode::Ultracode) {
                ExecutionModeChoice::Ultracode
            } else if current == ExecutionMode::Ultracode {
                ExecutionModeChoice::Normal
            } else {
                current_choice
            }
        }
    };
    // Auto keeps the mode it last picked until its next decision.
    let mode = choice.fixed().unwrap_or(current);
    // Persist before acknowledging. There is no await between read and commit.
    persist_mode(&handle.info, choice, mode)
        .map_err(|error| acp::Error::internal_error().data(error.to_string()))?;
    if choice == ExecutionModeChoice::Auto {
        handle.execution_mode.store_auto(mode);
    } else {
        handle.execution_mode.store(mode);
    }
    if requested_mode.is_some() {
        to_raw_response(&ModeState {
            mode,
            choice: Some(choice),
        })
    } else {
        to_raw_response(&SetSessionUltracodeResponse {
            enabled: mode == ExecutionMode::Ultracode,
        })
    }
}

/// Stores a loaded selection into the session's shared mode.
pub(crate) fn apply_mode(
    target: &distill_tool_types::AtomicExecutionMode,
    (choice, mode): (ExecutionModeChoice, ExecutionMode),
) {
    if choice == ExecutionModeChoice::Auto {
        target.store_auto(mode);
    } else {
        target.store(mode);
    }
}

/// The saved selection and effective mode. Sessions without a saved
/// selection start in Auto.
pub(crate) fn load_mode(
    info: &crate::session::info::Info,
) -> std::io::Result<(ExecutionModeChoice, ExecutionMode)> {
    let dir = crate::session::persistence::session_dir(info);
    let invalid = |error| std::io::Error::new(std::io::ErrorKind::InvalidData, error);
    match std::fs::read(dir.join("execution_mode.json")) {
        Ok(bytes) => serde_json::from_slice::<ModeState>(&bytes)
            .map(|state| (state.choice.unwrap_or(state.mode.into()), state.mode))
            .map_err(invalid),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            // Sessions saved before exclusive modes keep an explicit UltraCode.
            match std::fs::read(dir.join("ultracode.json")) {
                Ok(bytes) => serde_json::from_slice::<SetSessionUltracodeResponse>(&bytes)
                    .map(|state| {
                        if state.enabled {
                            (ExecutionModeChoice::Ultracode, ExecutionMode::Ultracode)
                        } else {
                            (ExecutionModeChoice::Auto, ExecutionMode::Normal)
                        }
                    })
                    .map_err(invalid),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    Ok((ExecutionModeChoice::Auto, ExecutionMode::Normal))
                }
                Err(error) => Err(error),
            }
        }
        Err(error) => Err(error),
    }
}

pub(crate) fn persist_mode(
    info: &crate::session::info::Info,
    choice: ExecutionModeChoice,
    mode: ExecutionMode,
) -> std::io::Result<()> {
    let dir = crate::session::persistence::ensure_owner_only_session_dir(info)?;
    let bytes = serde_json::to_vec(&ModeState {
        mode,
        choice: Some(choice),
    })
    .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    crate::session::storage::write_bytes_atomic(&dir.join("execution_mode.json"), &bytes)
}
