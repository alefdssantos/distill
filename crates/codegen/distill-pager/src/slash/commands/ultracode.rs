// Modified for Distill by Samuel Fajreldines, 2026.
//! Exclusive session execution modes. Explicit reasoning effort remains independent.

use crate::app::actions::Action;
use crate::slash::command::{AppCtx, CommandExecCtx, CommandResult, SlashCommand, slash_meta};
use distill_tool_types::{ExecutionMode, ExecutionModeOperation};

pub struct UltracodeCommand;
pub struct UltrafastCommand;

impl SlashCommand for UltracodeCommand {
    slash_meta! {
        name: "ultracode",
        description: "Select UltraCode: orchestrate work across parallel subagents",
        usage: "/ultracode [on|off]",
        takes_args: true,
        session_scoped: true,
        arg_placeholder: "[on|off]",
    }

    fn takes_args_now(&self, _ctx: &AppCtx) -> bool {
        false
    }

    fn run(&self, _ctx: &mut CommandExecCtx, args: &str) -> CommandResult {
        select_mode(args, ExecutionMode::Ultracode, self.usage())
    }
}

impl SlashCommand for UltrafastCommand {
    slash_meta! {
        name: "ultrafast",
        description: "Select UltraFast: direct execution and parallel independent work",
        usage: "/ultrafast [on|off]",
        takes_args: true,
        session_scoped: true,
        arg_placeholder: "[on|off]",
    }

    fn takes_args_now(&self, _ctx: &AppCtx) -> bool {
        false
    }

    fn run(&self, _ctx: &mut CommandExecCtx, args: &str) -> CommandResult {
        select_mode(args, ExecutionMode::Ultrafast, self.usage())
    }
}

fn select_mode(args: &str, target: ExecutionMode, usage: &str) -> CommandResult {
    let operation = match args.trim().to_ascii_lowercase().as_str() {
        "" => ExecutionModeOperation::Toggle,
        "on" => ExecutionModeOperation::Set,
        "off" => ExecutionModeOperation::Disable,
        other => {
            return CommandResult::Error(format!("unknown argument '{other}'; usage: {usage}"));
        }
    };
    CommandResult::Action(Action::SetExecutionMode {
        mode: target,
        operation,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::acp::model_state::ModelState;
    use crate::slash::commands::{builtin_commands, tests::make_ctx};
    use crate::slash::registry::CommandRegistry;

    fn run(command: &str, args: &str, mode: ExecutionMode) -> CommandResult {
        let reg = CommandRegistry::new(builtin_commands());
        let cmd = reg.get(command).expect("execution-mode command is registered").clone();
        let mut models = ModelState::default();
        models.execution_mode = mode;
        let mut ctx = make_ctx(&models);
        cmd.run(&mut ctx, args)
    }

    fn assert_operation(
        command: &str,
        args: &str,
        current: ExecutionMode,
        expected_mode: ExecutionMode,
        expected_operation: ExecutionModeOperation,
    ) {
        let CommandResult::Action(Action::SetExecutionMode { mode, operation }) =
            run(command, args, current)
        else {
            panic!("expected an execution-mode action");
        };
        assert_eq!(mode, expected_mode);
        assert_eq!(operation, expected_operation);
    }

    #[test]
    fn execution_mode_commands_resolve_toggle_and_disable_on_the_server() {
        use ExecutionMode::*;
        use ExecutionModeOperation::*;
        assert_operation("ultrafast", "on", Ultracode, Ultrafast, Set);
        assert_operation("ultracode", "on", Ultrafast, Ultracode, Set);
        // The client may still show Normal while the previous "on" is pending.
        assert_operation("ultrafast", " OFF ", Normal, Ultrafast, Disable);
        assert_operation("ultracode", "off", Ultrafast, Ultracode, Disable);
        assert_operation("ultrafast", "", Normal, Ultrafast, Toggle);
        assert_operation("ultrafast", "", Ultrafast, Ultrafast, Toggle);
        assert_operation("ultracode", "", Ultracode, Ultracode, Toggle);
        assert!(matches!(run("ultrafast", "bogus", Normal), CommandResult::Error(_)));
        assert!(matches!(run("ultracode", "bogus", Normal), CommandResult::Error(_)));
    }

    #[test]
    fn execution_mode_menu_pick_runs_without_args() {
        let models = ModelState::default();
        let ctx = AppCtx {
            models: &models,
            cwd: std::path::Path::new("."),
            has_session_announcements: false,
            billing_surface_visible: true,
            usage_command_visible: true,
            workflows_available: true,
            saved_workflows: &[],
            workflow_runs: &[],
            screen_mode: crate::app::ScreenMode::Fullscreen,
            current_title: None,
        };
        assert!(!UltracodeCommand.takes_args_now(&ctx));
        assert!(!UltrafastCommand.takes_args_now(&ctx));
        let registry = CommandRegistry::new(builtin_commands());
        assert!(crate::slash::is_command_complete("/ultracode", &registry));
        assert!(crate::slash::is_command_complete("/ultrafast", &registry));
    }
}
