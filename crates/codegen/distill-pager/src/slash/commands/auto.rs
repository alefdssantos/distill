//! `/auto` toggles Auto routing (the account profile set up in `/accounts`).
//!
//! On, Auto picks each session's Main and Worker from the routing options.
//! Off, the selected Main and Worker always run.

use crate::app::actions::Action;
use crate::slash::command::{CommandExecCtx, CommandResult, SlashCommand, slash_meta};

/// Toggle Auto routing.
pub struct AutoCommand;

impl SlashCommand for AutoCommand {
    slash_meta! {
        name: "auto",
        description: "Toggle Auto routing (accounts pick Main and Worker)",
        usage: "/auto",
    }

    fn run(&self, _ctx: &mut CommandExecCtx, _args: &str) -> CommandResult {
        CommandResult::Action(Action::ToggleAutoRouting)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::acp::model_state::ModelState;
    use crate::app::bundle::BundleState;

    /// `/auto` is the Auto routing switch, never the permission mode.
    #[test]
    fn toggles_auto_routing() {
        let models = ModelState::default();
        let bundle = BundleState::default();
        let mut ctx = CommandExecCtx {
            models: &models,
            session_id: None,
            bundle_state: &bundle,
            screen_mode: crate::app::ScreenMode::Inline,
            billing_surface_visible: true,
            usage_command_visible: true,
            pager_state: Default::default(),
        };
        assert!(matches!(
            AutoCommand.run(&mut ctx, ""),
            CommandResult::Action(Action::ToggleAutoRouting)
        ));
    }
}
