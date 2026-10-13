use crate::app::actions::Action;
use crate::slash::command::{CommandExecCtx, CommandResult, SlashCommand, slash_meta};
pub struct AccountsCommand;
impl SlashCommand for AccountsCommand {
    slash_meta! { name: "accounts", description: "Manage accounts, quota and automatic model routing", usage: "/accounts", }
    fn run(&self, _ctx: &mut CommandExecCtx, _args: &str) -> CommandResult {
        CommandResult::Action(Action::OpenAccounts)
    }
}
