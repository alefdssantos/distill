// Modified for Distill by Samuel Fajreldines, 2026.
//! `/mode`: select the session execution mode (Auto, or one fixed mode).

use crate::app::actions::Action;
use crate::slash::command::{
    AppCtx, ArgItem, CommandExecCtx, CommandResult, SlashCommand, slash_meta,
};
use distill_tool_types::{ExecutionModeChoice, ExecutionModeOperation};

pub struct ModeCommand;

const MODE_ARGS: [(&str, ExecutionModeChoice, &str); 5] = [
    ("auto", ExecutionModeChoice::Auto, "Jev adapts the mode to each action"),
    ("normal", ExecutionModeChoice::Normal, "Standard execution"),
    ("ultrafast", ExecutionModeChoice::Ultrafast, "Direct execution and parallel independent work"),
    ("ultracode", ExecutionModeChoice::Ultracode, "Careful reasoning, refactoring and complex logic"),
    ("ultrasave", ExecutionModeChoice::Ultrasave, "Prioritize token economy"),
];

impl SlashCommand for ModeCommand {
    slash_meta! {
        name: "mode",
        description: "Select execution mode: auto (Jev adapts per action), normal, ultrafast, ultracode, ultrasave",
        usage: "/mode <auto|normal|ultrafast|ultracode|ultrasave>",
        takes_args: true,
        args_required: true,
        session_scoped: true,
        arg_placeholder: "<auto|normal|ultrafast|ultracode|ultrasave>",
    }

    fn suggest_args(&self, _ctx: &AppCtx, _args_query: &str) -> Option<Vec<ArgItem>> {
        Some(
            MODE_ARGS
                .iter()
                .map(|(name, _, description)| ArgItem {
                    display: (*name).to_string(),
                    match_text: (*name).to_string(),
                    insert_text: (*name).to_string(),
                    description: (*description).to_string(),
                })
                .collect(),
        )
    }

    fn run(&self, _ctx: &mut CommandExecCtx, args: &str) -> CommandResult {
        let arg = args.trim().to_ascii_lowercase();
        match MODE_ARGS.iter().find(|(name, _, _)| *name == arg) {
            Some((_, choice, _)) => CommandResult::Action(Action::SetExecutionMode {
                mode: *choice,
                operation: ExecutionModeOperation::Set,
            }),
            None if arg.is_empty() => CommandResult::Error(format!("usage: {}", self.usage())),
            None => CommandResult::Error(format!(
                "unknown argument '{arg}'; usage: {}",
                self.usage()
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slash::commands::{builtin_commands, tests::make_ctx};
    use crate::slash::registry::CommandRegistry;
    use crate::acp::model_state::ModelState;

    fn run(args: &str) -> CommandResult {
        let reg = CommandRegistry::new(builtin_commands());
        let cmd = reg.get("mode").expect("mode command is registered").clone();
        let models = ModelState::default();
        let mut ctx = make_ctx(&models);
        cmd.run(&mut ctx, args)
    }

    #[test]
    fn mode_args_select_the_matching_choice() {
        use ExecutionModeChoice::*;
        for (arg, expected) in [
            ("auto", Auto),
            ("normal", Normal),
            ("ultrafast", Ultrafast),
            ("ultracode", Ultracode),
            ("ultrasave", Ultrasave),
            (" UltraSave ", Ultrasave),
        ] {
            let CommandResult::Action(Action::SetExecutionMode { mode, operation }) = run(arg)
            else {
                panic!("expected an execution-mode action for {arg:?}");
            };
            assert_eq!(mode, expected, "arg {arg:?} must select its own choice");
            assert_eq!(operation, ExecutionModeOperation::Set);
        }
    }

    #[test]
    fn mode_rejects_empty_and_unknown_args_with_usage() {
        for arg in ["", "   ", "on", "off", "toggle", "fast"] {
            let CommandResult::Error(message) = run(arg) else {
                panic!("expected an error for {arg:?}");
            };
            assert!(message.contains("/mode <auto|normal|ultrafast|ultracode|ultrasave>"));
        }
    }

    #[test]
    fn mode_is_registered_and_offers_all_five_choices() {
        let registry = CommandRegistry::new(builtin_commands());
        assert!(registry.get("ultracode").is_none());
        assert!(registry.get("ultrafast").is_none());
        assert!(registry.get("ultrasave").is_none());
        assert!(crate::slash::is_command_complete("/mode auto", &registry));
        assert!(!crate::slash::is_command_complete("/mode", &registry));
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
        let items = ModeCommand.suggest_args(&ctx, "").expect("arg items");
        let names: Vec<_> = items.iter().map(|item| item.insert_text.as_str()).collect();
        assert_eq!(names, ["auto", "normal", "ultrafast", "ultracode", "ultrasave"]);
    }
}
