//! Account modal dispatch stays synchronous; AccountStore access is an effect.
use crate::app::{
    actions::{AccountsTask, Action, Effect, SwitchModelError},
    app_view::{ActiveView, AppView},
};
use crate::views::{
    accounts_modal::{AccountsData, AccountsModalState, AccountsRequest},
    modal::ActiveModal,
};
use agent_client_protocol as acp;

pub(super) fn open(app: &mut AppView) -> Vec<Effect> {
    let mut effects = if matches!(app.active_view, ActiveView::Agent(_)) {
        vec![]
    } else {
        super::settings::ui::dispatch_open_settings(app, None)
    };
    let ActiveView::Agent(id) = app.active_view else {
        return effects;
    };
    let Some(agent) = app.agents.get_mut(&id) else {
        return effects;
    };
    let models = agent
        .session
        .models
        .available
        .iter()
        .filter(|(id, _)| id.0.as_ref() != "auto")
        .map(|(_, info)| info.clone())
        .collect();
    agent.active_modal = Some(ActiveModal::Accounts {
        state: Box::new(AccountsModalState::new(models)),
    });
    effects.extend(request(app, AccountsRequest::Load));
    effects
}
pub(super) fn request(app: &mut AppView, request: AccountsRequest) -> Vec<Effect> {
    let ActiveView::Agent(agent_id) = app.active_view else {
        return vec![];
    };
    let Some(agent) = super::ctx::get_active_agent_mut(app) else {
        return vec![];
    };
    let Some(ActiveModal::Accounts { state }) = agent.active_modal.as_mut() else {
        return vec![];
    };
    if matches!(request, AccountsRequest::Save(_)) && agent.session.model_switch_pending {
        state.message = Some("Wait for the current model switch, then retry Save.".into());
        return vec![];
    }
    if matches!(request, AccountsRequest::Save(_)) {
        state.update_models(agent.session.models.available.values().cloned());
    }
    let Some((seq, cancel)) = state.begin(request.clone()) else {
        return vec![];
    };
    vec![Effect::Accounts(AccountsTask {
        agent_id,
        instance: state.instance,
        seq,
        request,
        cancel,
    })]
}
pub(super) fn complete(
    app: &mut AppView,
    agent_id: crate::app::agent::AgentId,
    instance: u64,
    seq: u64,
    result: Result<AccountsData, String>,
) -> Vec<Effect> {
    let Some(agent) = app.agents.get_mut(&agent_id) else {
        return vec![];
    };
    let Some(ActiveModal::Accounts { state }) = agent.active_modal.as_mut() else {
        return vec![];
    };
    if state.instance != instance || state.request_seq != seq || state.busy.is_none() {
        return vec![];
    }
    let activate = result.is_ok()
        && matches!(state.busy.as_ref(), Some(AccountsRequest::Save(routing)) if routing.enabled);
    let catalog = result
        .as_ref()
        .ok()
        .and_then(|data| data.catalog.as_ref())
        .map(|catalog| {
            catalog
                .available_models
                .iter()
                .map(|info| (info.model_id.clone(), info.clone()))
                .collect::<indexmap::IndexMap<_, _>>()
        });
    if let Some(id) = state.complete(seq, result) {
        let request = AccountsRequest::Login(id);
        if let Some((seq, cancel)) = state.begin(request.clone()) {
            return vec![Effect::Accounts(AccountsTask {
                agent_id,
                instance,
                seq,
                request,
                cancel,
            })];
        }
    }
    if activate {
        state.auto_activation_pending = true;
        state.message = Some("Routing saved. Activating Auto in this session…".into());
    }
    if let Some(catalog) = catalog {
        agent.session.models.update_catalog(catalog.clone());
        if app.active_view == ActiveView::Agent(agent_id) {
            app.models.update_catalog(catalog);
        }
    }
    if activate {
        if app.active_view != ActiveView::Agent(agent_id) {
            activation_finished(
                app,
                agent_id,
                &acp::ModelId::new("auto"),
                &Err(SwitchModelError::Other("Session changed".into())),
            );
            return vec![];
        }
        return super::router::dispatch(
            Action::SwitchModel {
                model_id: acp::ModelId::new("auto"),
                effort: None,
            },
            app,
        );
    }
    vec![]
}
/// `/auto` saved: Auto on moves the active session to Auto; Auto off returns
/// an Auto-routed session to the selected Main and this instance's Worker.
pub(super) fn auto_routing_set(
    app: &mut AppView,
    result: Result<(bool, Option<acp::ModelId>), String>,
) -> Vec<Effect> {
    let (enabled, main) = match result {
        Ok(state) => state,
        Err(message) => {
            app.show_toast(&message);
            return vec![];
        }
    };
    app.show_toast(if enabled {
        "Auto routing on: accounts pick Main and Worker"
    } else {
        "Auto routing off: selected Main and Worker run"
    });
    let ActiveView::Agent(id) = app.active_view else {
        return vec![];
    };
    let Some(agent) = app.agents.get_mut(&id) else {
        return vec![];
    };
    let routed = agent.session.models.auto_route.is_some();
    let model_id = match (enabled, main) {
        (true, _) => acp::ModelId::new("auto"),
        (false, Some(main)) if routed => main,
        (false, _) => {
            agent.session.models.auto_route = None;
            app.models.auto_route = None;
            return vec![];
        }
    };
    let mut effects = super::router::dispatch(
        Action::SwitchModel {
            model_id,
            effort: None,
        },
        app,
    );
    if !enabled
        && let Some(session_id) = app
            .agents
            .get(&id)
            .and_then(|agent| agent.session.session_id.clone())
    {
        effects.push(Effect::SetSessionWorker {
            session_id,
            model_id: crate::acp::ModelState::configured_worker_model()
                .map_or_else(String::new, |worker| worker.0.to_string()),
            effort: crate::acp::ModelState::configured_worker_effort(),
        });
    }
    effects
}
/// A saved profile and successful model activation are separate acknowledgements.
pub(super) fn activation_finished(
    app: &mut AppView,
    agent_id: crate::app::agent::AgentId,
    model_id: &acp::ModelId,
    result: &Result<Option<u64>, SwitchModelError>,
) {
    if model_id.0.as_ref() != "auto" {
        return;
    }
    let Some(agent) = app.agents.get_mut(&agent_id) else {
        return;
    };
    let Some(ActiveModal::Accounts { state }) = agent.active_modal.as_mut() else {
        return;
    };
    if !state.auto_activation_pending {
        return;
    }
    state.auto_activation_pending = false;
    let message = if result.is_ok() {
        "Routing saved. Auto active in this session."
    } else {
        "Auto activation failed. Routing saved; current model kept. Retry Save."
    };
    state.message = Some(message.into());
    if result.is_err() {
        app.show_toast(message);
    }
}
