//! Open a Gemini session in this terminal, without leaving Distill.

use crate::app::actions::Action;
use crate::slash::command::{CommandExecCtx, CommandResult, SlashCommand, slash_meta};

pub struct GeminiCommand;

impl SlashCommand for GeminiCommand {
    slash_meta! {
        name: "gemini",
        description: "Open a Gemini session in this window",
        usage: "/gemini",
    }

    fn run(&self, _ctx: &mut CommandExecCtx, _args: &str) -> CommandResult {
        CommandResult::Action(Action::OpenGeminiSession)
    }
}
