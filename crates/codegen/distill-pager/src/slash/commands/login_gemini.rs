//! Sign in to Gemini through the official Antigravity CLI.

use crate::app::actions::Action;
use crate::slash::command::{CommandExecCtx, CommandResult, SlashCommand, slash_meta};

pub struct LoginGeminiCommand;

impl SlashCommand for LoginGeminiCommand {
    slash_meta! {
        name: "login-gemini",
        description: "Sign in to Gemini through the official Antigravity CLI",
        usage: "/login-gemini",
    }

    fn run(&self, _ctx: &mut CommandExecCtx, _args: &str) -> CommandResult {
        CommandResult::Action(Action::LoginGemini)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::acp::model_state::ModelState;
    use crate::app::bundle::BundleState;
    use crate::app::ScreenMode;
    use crate::settings::PagerLocalSnapshot;

    #[test]
    fn dispatches_login_gemini_action_without_a_session() {
        let models = ModelState::default();
        let bundle = BundleState::default();
        let mut ctx = CommandExecCtx {
            models: &models,
            session_id: None,
            bundle_state: &bundle,
            screen_mode: ScreenMode::Minimal,
            billing_surface_visible: true,
            usage_command_visible: true,
            pager_state: PagerLocalSnapshot::default(),
        };

        assert!(matches!(
            LoginGeminiCommand.run(&mut ctx, ""),
            CommandResult::Action(Action::LoginGemini)
        ));
    }
}
