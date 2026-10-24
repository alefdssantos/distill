//! The model setup screen opened by the prompt footer's `change`, the welcome menu and `/tiers`.
//!
//! It follows the onboarding layout: an explanation card, then one list of choices. Auto routing
//! and accounts come first because, while Auto is on, they decide which models run; the Main,
//! Worker and Utility picks below them apply whenever Auto is off. Model and effort picks are a
//! draft until Save, which submits the same `model effort` strings `/tiers` has always used.

use crate::acp::ModelState;
use crate::input::line_editor::{LineEditOutcome, LineEditor};
use crate::theme::Theme;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};
use distill_pager_render::render::color::blend_color;
use distill_shell::accounts::AutoStrategy;
use distill_shell::sampling::types::{ReasoningEffort, ReasoningEffortOption};
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Margin, Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Widget, Wrap};

const HOME_AUTO: usize = 0;
const HOME_ACCOUNTS: usize = 1;
const HOME_MAIN: usize = 2;
const HOME_WORKER: usize = 3;
const HOME_UTILITY: usize = 4;
const HOME_EXECUTION_MODE: usize = 5;
const HOME_SAVE: usize = 6;
const HOME_ROWS: usize = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    Main,
    Worker,
}

impl Tier {
    fn name(self) -> &'static str {
        match self {
            Self::Main => "Main",
            Self::Worker => "Worker",
        }
    }

    fn home_row(self) -> usize {
        match self {
            Self::Main => HOME_MAIN,
            Self::Worker => HOME_WORKER,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    Home,
    Model(Tier),
    Effort(Tier, String),
    Utility,
}

#[derive(Debug, PartialEq, Eq)]
pub enum TierEditorOutcome {
    Submitted {
        main: String,
        worker: String,
        utility: String,
        execution_mode: distill_tool_types::ExecutionModeChoice,
    },
    Cancelled,
    Changed,
    Unchanged,
    ToggleAutoRouting,
    OpenAccounts,
}

impl TierEditorOutcome {
    /// How the host applies an outcome: whether to close the screen, the action to
    /// dispatch, and whether to redraw.
    pub fn host(self) -> (bool, Option<crate::app::actions::Action>, bool) {
        use crate::app::actions::Action;
        match self {
            Self::Submitted {
                main,
                worker,
                utility,
                execution_mode,
            } => (
                false,
                Some(Action::SetTierEditor {
                    main,
                    worker,
                    utility,
                    execution_mode,
                }),
                true,
            ),
            Self::Cancelled => (true, None, true),
            Self::Changed => (false, None, true),
            Self::Unchanged => (false, None, false),
            Self::ToggleAutoRouting => (false, Some(Action::ToggleAutoRouting), true),
            Self::OpenAccounts => (true, Some(Action::OpenAccounts), true),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ModelChoice {
    pub id: String,
    pub name: String,
    pub efforts: Vec<ReasoningEffortOption>,
}

/// The saved Auto routing, summarized for display. Credentials are never read.
#[derive(Debug, Clone, Default)]
pub struct AutoSummary {
    pub enabled: bool,
    pub strategy: AutoStrategy,
    pub accounts: usize,
    /// `(account, main model id, worker model id)` per route, in priority order.
    pub routes: Vec<(String, String, String)>,
}

impl AutoSummary {
    pub fn load() -> Self {
        let store = distill_shell::accounts::AccountStore::default();
        let accounts = store.list().unwrap_or_default();
        let routing = store.load_routing().unwrap_or_default();
        let routes = routing
            .options
            .iter()
            .map(|option| {
                let account = accounts
                    .iter()
                    .find(|account| account.id == option.account_id)
                    .map_or_else(
                        || option.account_id.clone(),
                        |account| {
                            format!(
                                "{} · {}",
                                super::accounts_modal::provider_name(account.provider),
                                account.label
                            )
                        },
                    );
                (
                    account,
                    option.main_model.clone(),
                    option.worker_model().to_owned(),
                )
            })
            .collect();
        Self {
            enabled: routing.enabled,
            strategy: routing.strategy,
            accounts: accounts.len(),
            routes,
        }
    }
}

type Submission = (String, String, String, distill_tool_types::ExecutionModeChoice);

#[derive(Debug)]
pub struct TierEditorState {
    pub step: Step,
    pub selected: usize,
    pub main: Option<String>,
    pub main_effort: Option<ReasoningEffort>,
    pub worker: Option<String>,
    pub worker_effort: Option<ReasoningEffort>,
    pub utility: LineEditor,
    pub execution_mode: distill_tool_types::ExecutionModeChoice,
    pub auto: AutoSummary,
    pub auto_pending: bool,
    /// Display names of the pair Auto chose for this session.
    pub routed: Option<(String, String)>,
    pub models: Vec<ModelChoice>,
    pub utility_entries: Vec<(String, String)>,
    pub query: LineEditor,
    /// Message under the list and whether it is an error.
    pub status: Option<(String, bool)>,
    initial: Submission,
    confirm_discard: bool,
    scroll: usize,
    hit_rows: Vec<(Rect, usize)>,
}

impl TierEditorState {
    pub fn from_models(models: &ModelState) -> Self {
        let utility = distill_shell::jev::local_config_cached()
            .model
            .clone()
            .unwrap_or_default();
        Self::from_parts(
            models,
            AutoSummary::load(),
            utility,
            crate::slash::commands::provider_status::openrouter_entries(),
            || {
                distill_shell::config::load_effective_config()
                    .ok()?
                    .get("models")?
                    .get("default")?
                    .as_str()
                    .map(str::to_owned)
                    .filter(|id| !id.trim().is_empty())
            },
        )
    }

    /// `configured_main` supplies the saved Main when Auto routing owns the session
    /// (its current model is `auto`).
    pub fn from_parts(
        models: &ModelState,
        auto: AutoSummary,
        utility: String,
        utility_entries: Vec<(String, String)>,
        configured_main: impl FnOnce() -> Option<String>,
    ) -> Self {
        let main = match models.current_model_id_str() {
            Some("auto") => configured_main(),
            other => other.map(str::to_owned),
        };
        let mut editor = Self {
            step: Step::Home,
            selected: HOME_MAIN,
            main,
            main_effort: if models.effort_auto {
                None
            } else {
                models.reasoning_effort
            },
            worker: models.worker_model.as_ref().map(|id| id.0.to_string()),
            worker_effort: models.worker_effort,
            utility: LineEditor::default(),
            execution_mode: models.execution_mode_choice,
            auto,
            auto_pending: false,
            routed: models
                .auto_route
                .as_ref()
                .map(|(main, worker)| (models.display_name(main), models.display_name(worker))),
            models: models
                .available
                .iter()
                .filter(|(id, _)| id.0.as_ref() != "auto")
                .map(|(id, info)| ModelChoice {
                    id: id.0.to_string(),
                    name: info.name.clone(),
                    efforts: models.reasoning_effort_options_for(id),
                })
                .collect(),
            utility_entries,
            query: LineEditor::default(),
            status: None,
            initial: Default::default(),
            confirm_discard: false,
            scroll: 0,
            hit_rows: Vec::new(),
        };
        editor.utility.set_text(utility);
        editor.initial = editor.submission();
        editor
    }

    fn submission(&self) -> Submission {
        let pick = |id: &Option<String>, effort: Option<ReasoningEffort>| {
            id.as_ref().map_or_else(String::new, |id| {
                format!(
                    "{id} {}",
                    crate::acp::model_state::effort_setting_label(effort)
                )
            })
        };
        (
            pick(&self.main, self.main_effort),
            pick(&self.worker, self.worker_effort),
            self.utility.text().trim().to_owned(),
            self.execution_mode,
        )
    }

    pub fn dirty(&self) -> bool {
        self.submission() != self.initial
    }

    fn name_of(&self, id: &str) -> String {
        self.models
            .iter()
            .find(|model| model.id == id)
            .map_or_else(|| id.to_owned(), |model| model.name.clone())
    }

    fn filtered(&self) -> Vec<&ModelChoice> {
        let query = self.query.text().trim().to_lowercase();
        self.models
            .iter()
            .filter(|model| {
                query.is_empty()
                    || model.name.to_lowercase().contains(&query)
                    || model.id.to_lowercase().contains(&query)
            })
            .collect()
    }

    fn efforts_for(&self, id: &str) -> &[ReasoningEffortOption] {
        self.models
            .iter()
            .find(|model| model.id == id)
            .map_or(&[], |model| model.efforts.as_slice())
    }

    fn row_count(&self) -> usize {
        match &self.step {
            Step::Home => HOME_ROWS,
            Step::Model(tier) => self.filtered().len() + usize::from(*tier == Tier::Worker),
            Step::Effort(_, id) => 1 + self.efforts_for(id).len(),
            Step::Utility => self.utility_entries.len() + 2,
        }
    }

    /// Apply the result of an Auto routing toggle started here.
    pub fn auto_routing_result(&mut self, result: Result<bool, String>) {
        self.auto_pending = false;
        match result {
            Ok(enabled) => {
                self.auto.enabled = enabled;
                self.routed = None;
                self.status = Some((
                    if enabled {
                        "Auto routing is on. The next turn uses the first route that fits."
                    } else {
                        "Auto routing is off. Your Main and Worker run from now on."
                    }
                    .to_owned(),
                    false,
                ));
            }
            Err(error) => self.status = Some((error, true)),
        }
    }

    fn go(&mut self, step: Step, selected: usize) {
        self.step = step;
        self.selected = selected;
        self.scroll = 0;
    }

    fn back_home(&mut self, row: usize) {
        self.query.reset();
        self.go(Step::Home, row);
    }

    fn navigate(&mut self, code: KeyCode, shift: bool) -> bool {
        let count = self.row_count().max(1);
        self.selected = match code {
            KeyCode::Up => (self.selected + count - 1) % count,
            KeyCode::BackTab => (self.selected + count - 1) % count,
            KeyCode::Tab if shift => (self.selected + count - 1) % count,
            KeyCode::Down | KeyCode::Tab => (self.selected + 1) % count,
            KeyCode::Home => 0,
            KeyCode::End => count - 1,
            KeyCode::PageUp => self.selected.saturating_sub(5),
            KeyCode::PageDown => (self.selected + 5).min(count - 1),
            _ => return false,
        };
        true
    }

    fn save(&mut self) -> TierEditorOutcome {
        if !self.dirty() {
            return TierEditorOutcome::Cancelled;
        }
        let (main, worker, utility, execution_mode) = self.submission();
        TierEditorOutcome::Submitted {
            main,
            worker,
            utility,
            execution_mode,
        }
    }

    fn activate(&mut self) -> TierEditorOutcome {
        match self.step.clone() {
            Step::Home => self.activate_home(self.selected),
            Step::Model(tier) => self.pick_model(tier),
            Step::Effort(tier, id) => self.pick_effort(tier, &id),
            Step::Utility => self.pick_utility(),
        }
    }

    fn activate_home(&mut self, row: usize) -> TierEditorOutcome {
        self.selected = row;
        match row {
            HOME_AUTO => {
                if self.auto_pending {
                    return TierEditorOutcome::Unchanged;
                }
                if !self.auto.enabled && self.auto.routes.is_empty() {
                    return self.open_accounts();
                }
                self.auto_pending = true;
                self.status = Some((
                    if self.auto.enabled {
                        "Turning Auto routing off…"
                    } else {
                        "Turning Auto routing on…"
                    }
                    .to_owned(),
                    false,
                ));
                TierEditorOutcome::ToggleAutoRouting
            }
            HOME_ACCOUNTS => self.open_accounts(),
            HOME_MAIN | HOME_WORKER => {
                let tier = if row == HOME_MAIN {
                    Tier::Main
                } else {
                    Tier::Worker
                };
                let current = match tier {
                    Tier::Main => self.main.clone(),
                    Tier::Worker => self.worker.clone(),
                };
                let offset = usize::from(tier == Tier::Worker);
                let selected = current
                    .and_then(|id| self.models.iter().position(|model| model.id == id))
                    .map_or(0, |index| index + offset);
                self.query.reset();
                self.go(Step::Model(tier), selected);
                TierEditorOutcome::Changed
            }
            HOME_UTILITY => {
                let text = self.utility.text().trim();
                let selected = if text.is_empty() {
                    self.utility_entries.len() + 1
                } else {
                    self.utility_entries
                        .iter()
                        .position(|(key, _)| key == text)
                        .map_or(0, |index| index + 1)
                };
                self.go(Step::Utility, selected);
                TierEditorOutcome::Changed
            }
            HOME_EXECUTION_MODE => {
                use distill_tool_types::ExecutionModeChoice as C;
                self.execution_mode = match self.execution_mode {
                    C::Auto => C::Normal,
                    C::Normal => C::Ultrafast,
                    C::Ultrafast => C::Ultracode,
                    C::Ultracode => C::Ultrasave,
                    C::Ultrasave => C::Auto,
                };
                TierEditorOutcome::Changed
            }
            _ => self.save(),
        }
    }

    fn open_accounts(&mut self) -> TierEditorOutcome {
        if self.dirty() {
            self.status = Some((
                "Save your model changes first (s), then open Accounts.".to_owned(),
                true,
            ));
            return TierEditorOutcome::Changed;
        }
        TierEditorOutcome::OpenAccounts
    }

    fn pick_model(&mut self, tier: Tier) -> TierEditorOutcome {
        if tier == Tier::Worker && self.selected == 0 {
            self.worker = None;
            self.worker_effort = None;
            self.status = Some((
                "Worker removed: Main does all the work. Press s to save.".to_owned(),
                false,
            ));
            self.back_home(HOME_WORKER);
            return TierEditorOutcome::Changed;
        }
        let offset = usize::from(tier == Tier::Worker);
        let Some(choice) = self.filtered().get(self.selected - offset).map(|model| {
            (model.id.clone(), model.efforts.is_empty())
        }) else {
            return TierEditorOutcome::Unchanged;
        };
        let (id, no_efforts) = choice;
        if no_efforts {
            self.set_pick(tier, id, None);
            return TierEditorOutcome::Changed;
        }
        let current_effort = match tier {
            Tier::Main if self.main.as_deref() == Some(id.as_str()) => self.main_effort,
            Tier::Worker if self.worker.as_deref() == Some(id.as_str()) => self.worker_effort,
            _ => None,
        };
        let selected = current_effort
            .and_then(|level| {
                self.efforts_for(&id)
                    .iter()
                    .position(|option| option.value == level)
            })
            .map_or(0, |index| index + 1);
        self.go(Step::Effort(tier, id), selected);
        TierEditorOutcome::Changed
    }

    fn pick_effort(&mut self, tier: Tier, id: &str) -> TierEditorOutcome {
        let effort = self
            .selected
            .checked_sub(1)
            .and_then(|index| self.efforts_for(id).get(index))
            .map(|option| option.value);
        self.set_pick(tier, id.to_owned(), effort);
        TierEditorOutcome::Changed
    }

    fn set_pick(&mut self, tier: Tier, id: String, effort: Option<ReasoningEffort>) {
        let name = self.name_of(&id);
        match tier {
            Tier::Main => {
                self.main = Some(id);
                self.main_effort = effort;
            }
            Tier::Worker => {
                self.worker = Some(id);
                self.worker_effort = effort;
            }
        }
        self.status = Some((
            format!(
                "{}: {name} · {}. Press s to save.",
                tier.name(),
                crate::acp::model_state::effort_setting_label(effort)
            ),
            false,
        ));
        self.back_home(tier.home_row());
    }

    fn pick_utility(&mut self) -> TierEditorOutcome {
        if let Some((key, _)) = self
            .selected
            .checked_sub(1)
            .and_then(|index| self.utility_entries.get(index))
        {
            let key = key.clone();
            self.utility.set_text(key);
        } else if self.selected > self.utility_entries.len() {
            self.utility.reset();
        }
        let text = self.utility.text().trim();
        self.status = Some((
            if text.is_empty() {
                "Utility model off. Press s to save.".to_owned()
            } else {
                format!("Utility: {text}. Press s to save.")
            },
            false,
        ));
        self.back_home(HOME_UTILITY);
        TierEditorOutcome::Changed
    }

    pub fn handle_key(&mut self, key: &KeyEvent) -> TierEditorOutcome {
        if key.kind == KeyEventKind::Release {
            return TierEditorOutcome::Unchanged;
        }
        let control = key.modifiers.contains(KeyModifiers::CONTROL);
        if control && matches!(key.code, KeyCode::Char('c' | 'd' | 'q')) {
            return TierEditorOutcome::Cancelled;
        }
        if control && key.code == KeyCode::Char('s') {
            return self.save();
        }
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);
        if key.code != KeyCode::Esc {
            self.confirm_discard = false;
        }
        match self.step.clone() {
            Step::Home => match key.code {
                KeyCode::Esc => {
                    if self.dirty() && !self.confirm_discard {
                        self.confirm_discard = true;
                        self.status = Some((
                            "Unsaved changes. Press Esc again to discard them, or s to save."
                                .to_owned(),
                            true,
                        ));
                        return TierEditorOutcome::Changed;
                    }
                    TierEditorOutcome::Cancelled
                }
                KeyCode::Enter | KeyCode::Right | KeyCode::Char(' ') => self.activate(),
                KeyCode::Char('s') => self.save(),
                KeyCode::Char('a') => self.activate_home(HOME_AUTO),
                code if self.navigate(code, shift) => TierEditorOutcome::Changed,
                _ => TierEditorOutcome::Unchanged,
            },
            Step::Model(tier) => match key.code {
                KeyCode::Esc => {
                    self.back_home(tier.home_row());
                    TierEditorOutcome::Changed
                }
                KeyCode::Enter => self.activate(),
                KeyCode::Backspace | KeyCode::Left if self.query.text().is_empty() => {
                    self.back_home(tier.home_row());
                    TierEditorOutcome::Changed
                }
                code if self.navigate(code, shift) => TierEditorOutcome::Changed,
                _ => self.edit_query(key, tier),
            },
            Step::Effort(tier, _) => match key.code {
                KeyCode::Esc | KeyCode::Left | KeyCode::Backspace => {
                    self.activate_home(tier.home_row())
                }
                KeyCode::Enter | KeyCode::Right | KeyCode::Char(' ') => self.activate(),
                code if self.navigate(code, shift) => TierEditorOutcome::Changed,
                _ => TierEditorOutcome::Unchanged,
            },
            Step::Utility => match key.code {
                KeyCode::Esc => {
                    self.back_home(HOME_UTILITY);
                    TierEditorOutcome::Changed
                }
                KeyCode::Enter => self.activate(),
                KeyCode::Left | KeyCode::Backspace if self.selected != 0 => {
                    self.back_home(HOME_UTILITY);
                    TierEditorOutcome::Changed
                }
                code if self.navigate(code, shift) => TierEditorOutcome::Changed,
                _ if self.selected == 0 => edit_outcome(
                    self.utility
                        .handle_key_with_insert_policy(key, |character| !character.is_control()),
                ),
                _ => TierEditorOutcome::Unchanged,
            },
        }
    }

    fn edit_query(&mut self, key: &KeyEvent, tier: Tier) -> TierEditorOutcome {
        let outcome = self
            .query
            .handle_key_with_insert_policy(key, |character| !character.is_control());
        if outcome == LineEditOutcome::TextChanged {
            // The first match is the likely pick; the Worker list keeps None above it.
            self.selected = usize::from(tier == Tier::Worker && !self.filtered().is_empty());
            self.scroll = 0;
        }
        edit_outcome(outcome)
    }

    pub fn insert_paste(&mut self, text: &str) -> TierEditorOutcome {
        match self.step {
            Step::Model(tier) => {
                let outcome = self.query.insert_paste(text);
                if outcome == LineEditOutcome::TextChanged {
                    self.selected =
                        usize::from(tier == Tier::Worker && !self.filtered().is_empty());
                }
                edit_outcome(outcome)
            }
            Step::Utility if self.selected == 0 => edit_outcome(self.utility.insert_paste(text)),
            _ => TierEditorOutcome::Unchanged,
        }
    }

    pub fn handle_mouse(&mut self, kind: MouseEventKind, column: u16, row: u16) -> TierEditorOutcome {
        match kind {
            MouseEventKind::ScrollUp => {
                self.navigate(KeyCode::Up, false);
                TierEditorOutcome::Changed
            }
            MouseEventKind::ScrollDown => {
                self.navigate(KeyCode::Down, false);
                TierEditorOutcome::Changed
            }
            MouseEventKind::Down(MouseButton::Left) => {
                let position = Position::new(column, row);
                let Some(index) = self
                    .hit_rows
                    .iter()
                    .find(|(rect, _)| rect.contains(position))
                    .map(|(_, index)| *index)
                else {
                    return TierEditorOutcome::Unchanged;
                };
                self.confirm_discard = false;
                self.selected = index;
                self.activate()
            }
            _ => TierEditorOutcome::Unchanged,
        }
    }
}

fn edit_outcome(outcome: LineEditOutcome) -> TierEditorOutcome {
    match outcome {
        LineEditOutcome::Unhandled => TierEditorOutcome::Unchanged,
        LineEditOutcome::HandledNoChange
        | LineEditOutcome::CursorChanged
        | LineEditOutcome::TextChanged => TierEditorOutcome::Changed,
    }
}

/// One list line: a heading when `index` is `None`, otherwise a selectable row.
struct ListLine {
    index: Option<usize>,
    label: Vec<Span<'static>>,
    value: Vec<Span<'static>>,
}

fn heading(text: impl Into<String>, theme: &Theme) -> ListLine {
    ListLine {
        index: None,
        label: vec![Span::styled(
            text.into(),
            Style::default()
                .fg(theme.running)
                .add_modifier(Modifier::BOLD),
        )],
        value: vec![],
    }
}

fn row(index: usize, label: impl Into<String>, value: Vec<Span<'static>>) -> ListLine {
    ListLine {
        index: Some(index),
        label: vec![Span::raw(label.into())],
        value,
    }
}

fn tag(text: impl Into<String>, theme: &Theme) -> Span<'static> {
    Span::styled(
        format!(" {} ", text.into()),
        Style::default().fg(theme.text_secondary).bg(theme.bg_light),
    )
}

fn dim(text: impl Into<String>, theme: &Theme) -> Span<'static> {
    Span::styled(text.into(), Style::default().fg(theme.gray_bright))
}

fn selection_style(theme: &Theme) -> Style {
    let background = blend_color(theme.bg_base, theme.accent_system, 0.18)
        .map(crate::theme::quantize)
        .unwrap_or(theme.bg_highlight);
    Style::default().fg(theme.text_primary).bg(background)
}

/// Text with a block cursor at the editor's cursor.
fn editor_spans(editor: &LineEditor, theme: &Theme) -> Vec<Span<'static>> {
    let text = editor.text();
    let cursor = editor.cursor_byte().min(text.len());
    let (before, rest) = text.split_at(cursor);
    let mut chars = rest.chars();
    let under = chars.next().map_or_else(|| " ".to_owned(), String::from);
    vec![
        Span::styled(before.to_owned(), Style::default().fg(theme.text_primary)),
        Span::styled(
            under,
            Style::default()
                .fg(theme.text_primary)
                .add_modifier(Modifier::REVERSED),
        ),
        Span::styled(chars.as_str().to_owned(), Style::default().fg(theme.text_primary)),
    ]
}

fn on_off(on: bool, color: ratatui::style::Color, theme: &Theme) -> Vec<Span<'static>> {
    if on {
        vec![Span::styled(
            "● On",
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )]
    } else {
        vec![dim("○ Off", theme)]
    }
}

fn plural(count: usize, word: &str) -> String {
    format!("{count} {word}{}", if count == 1 { "" } else { "s" })
}

fn step_title(step: &Step) -> String {
    match step {
        Step::Home => "Models".to_owned(),
        Step::Model(tier) => format!("{} model", tier.name()),
        Step::Effort(tier, _) => format!("{} effort", tier.name()),
        Step::Utility => "Utility model".to_owned(),
    }
}

fn intro(state: &TierEditorState, theme: &Theme) -> (&'static str, Vec<Line<'static>>) {
    let strong = |text: String, color| {
        Line::from(Span::styled(
            text,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ))
    };
    let mut lines = Vec::new();
    let icon = match &state.step {
        Step::Home => {
            if state.auto.enabled {
                lines.push(strong(
                    "Auto routing is on: your accounts choose the models.".into(),
                    theme.accent_success,
                ));
                lines.push(Line::from(match &state.routed {
                    Some((main, worker)) => format!("This session runs {main} → {worker}."),
                    None => "The next turn picks the first route that fits.".to_owned(),
                }));
                lines.push(Line::from(dim(
                    match state.auto.strategy {
                        AutoStrategy::OrderedFirst => {
                            "Rule: the first route, in order, that still has quota."
                        }
                        AutoStrategy::HighestRemaining => {
                            "Rule: the route with the most quota left."
                        }
                    },
                    theme,
                )));
                for (number, (account, main, worker)) in state.auto.routes.iter().take(3).enumerate()
                {
                    lines.push(Line::from(vec![
                        Span::styled(
                            format!(" {}. ", number + 1),
                            Style::default().fg(theme.running),
                        ),
                        Span::raw(format!("{account}  ")),
                        dim(
                            format!("{} / {}", state.name_of(main), state.name_of(worker)),
                            theme,
                        ),
                    ]));
                }
                if state.auto.routes.len() > 3 {
                    lines.push(Line::from(dim(
                        format!(" +{} more in Accounts", state.auto.routes.len() - 3),
                        theme,
                    )));
                }
            } else {
                lines.push(strong(
                    "Auto routing is off: the Main and Worker below always run.".into(),
                    theme.text_primary,
                ));
                lines.push(Line::from(
                    "Main plans, delegates and reviews. Worker runs the delegated work.",
                ));
                lines.push(Line::from(dim(
                    if state.auto.routes.is_empty() {
                        "Auto can switch models by account quota. Add a route in Accounts to try it."
                            .to_owned()
                    } else {
                        format!(
                            "{} ready. Turn Auto on to switch models by account quota.",
                            plural(state.auto.routes.len(), "route")
                        )
                    },
                    theme,
                )));
            }
            "◈"
        }
        Step::Model(tier) => {
            lines.push(strong(
                format!("Choose the {} model", tier.name()),
                theme.text_primary,
            ));
            lines.push(Line::from(match tier {
                Tier::Main => "It plans, delegates and reviews every session.",
                Tier::Worker => "Usually a cheaper model. It runs the work Main delegates.",
            }));
            if state.auto.enabled {
                lines.push(Line::from(Span::styled(
                    "Auto routing is on, so this pick runs when Auto is off.",
                    Style::default().fg(theme.running),
                )));
            }
            let mut filter = vec![Span::styled(
                "Filter › ",
                Style::default().fg(theme.accent_system),
            )];
            if state.query.text().is_empty() {
                filter.push(Span::styled(
                    " ",
                    Style::default().add_modifier(Modifier::REVERSED),
                ));
                filter.push(dim(
                    format!(" type to search {}", plural(state.models.len(), "model")),
                    theme,
                ));
            } else {
                filter.extend(editor_spans(&state.query, theme));
            }
            lines.push(Line::from(filter));
            "⇄"
        }
        Step::Effort(_, id) => {
            lines.push(strong(
                format!("How much should {} think?", state.name_of(id)),
                theme.text_primary,
            ));
            lines.push(Line::from(
                "Auto lets Jev pick the effort for each call: the usual balance of cost and quality.",
            ));
            lines.push(Line::from(dim(
                "Higher effort is slower and costs more tokens.",
                theme,
            )));
            "✦"
        }
        Step::Utility => {
            lines.push(strong("Choose the Utility model".into(), theme.text_primary));
            lines.push(Line::from(
                "A low-cost model for short or fallback work, such as summaries.",
            ));
            lines.push(Line::from(dim(
                "Pick an OpenRouter entry, or type a chain like vendor/model,vendor/other.",
                theme,
            )));
            "◇"
        }
    };
    (icon, lines)
}

fn list(state: &TierEditorState, theme: &Theme) -> (String, Vec<ListLine>) {
    let mut lines = Vec::new();
    let changed = |index: usize| {
        let now = state.submission();
        let differs = match index {
            HOME_MAIN => now.0 != state.initial.0,
            HOME_WORKER => now.1 != state.initial.1,
            HOME_UTILITY => now.2 != state.initial.2,
            _ => now.3 != state.initial.3,
        };
        if differs {
            vec![Span::styled(" •", Style::default().fg(theme.running))]
        } else {
            vec![]
        }
    };
    let pick_value = |id: &Option<String>, effort: Option<ReasoningEffort>, empty: &str| {
        id.as_ref().map_or_else(
            || vec![dim(empty.to_owned(), theme)],
            |id| {
                vec![
                    Span::raw(state.name_of(id)),
                    dim(
                        format!(
                            " · {}",
                            crate::acp::model_state::effort_setting_label(effort)
                        ),
                        theme,
                    ),
                ]
            },
        )
    };
    let title = match &state.step {
        Step::Home => {
            lines.push(heading("Routing", theme));
            let auto_value = if state.auto_pending {
                vec![dim("switching…", theme)]
            } else if !state.auto.enabled && state.auto.routes.is_empty() {
                vec![dim("Set up in Accounts →", theme)]
            } else {
                on_off(state.auto.enabled, theme.accent_success, theme)
            };
            lines.push(row(HOME_AUTO, "Auto routing", auto_value));
            lines.push(row(
                HOME_ACCOUNTS,
                "Accounts & routes",
                vec![dim(
                    format!(
                        "{} · {}  →",
                        plural(state.auto.accounts, "account"),
                        plural(state.auto.routes.len(), "route")
                    ),
                    theme,
                )],
            ));
            lines.push(heading(
                if state.auto.enabled {
                    "Models  (used when Auto is off)"
                } else {
                    "Models"
                },
                theme,
            ));
            let mut main = pick_value(&state.main, state.main_effort, "Not set: choose one");
            main.extend(changed(HOME_MAIN));
            lines.push(row(HOME_MAIN, "Main", main));
            let mut worker = pick_value(
                &state.worker,
                state.worker_effort,
                "None: Main does all the work",
            );
            worker.extend(changed(HOME_WORKER));
            lines.push(row(HOME_WORKER, "Worker", worker));
            let utility = state.utility.text().trim();
            let mut utility_value = if utility.is_empty() {
                vec![dim("None", theme)]
            } else {
                vec![Span::raw(utility.to_owned())]
            };
            utility_value.extend(changed(HOME_UTILITY));
            lines.push(row(HOME_UTILITY, "Utility", utility_value));
            lines.push(heading("Mode", theme));
            let mut execution_mode = vec![Span::styled(
                state.execution_mode.label(),
                Style::default().fg(theme.accent_thinking),
            )];
            execution_mode.extend(changed(HOME_EXECUTION_MODE));
            execution_mode.push(dim("  (Enter to change)", theme));
            lines.push(row(HOME_EXECUTION_MODE, "Execution", execution_mode));
            lines.push(ListLine {
                index: None,
                label: vec![],
                value: vec![],
            });
            lines.push(if state.dirty() {
                row(
                    HOME_SAVE,
                    "Save changes",
                    vec![Span::styled("• unsaved", Style::default().fg(theme.running))],
                )
            } else {
                row(HOME_SAVE, "Close", vec![dim("no changes", theme)])
            });
            "Set up how Distill picks models".to_owned()
        }
        Step::Model(tier) => {
            let current = match tier {
                Tier::Main => state.main.as_deref(),
                Tier::Worker => state.worker.as_deref(),
            };
            let filtered = state.filtered();
            if *tier == Tier::Worker {
                let mut value = vec![dim("Main does all the work", theme)];
                if current.is_none() {
                    value.push(Span::styled(" ✓", Style::default().fg(theme.accent_success)));
                }
                lines.push(row(0, "None", value));
            }
            let offset = usize::from(*tier == Tier::Worker);
            if filtered.is_empty() {
                lines.push(heading(
                    format!(
                        "No model matches \u{201c}{}\u{201d}. Backspace edits the filter.",
                        state.query.text()
                    ),
                    theme,
                ));
            }
            for (index, model) in filtered.iter().enumerate() {
                let mut value = vec![tag(model.id.clone(), theme)];
                if current == Some(model.id.as_str()) {
                    value.push(Span::styled(" ✓", Style::default().fg(theme.accent_success)));
                }
                lines.push(row(index + offset, model.name.clone(), value));
            }
            format!("Models ({}/{})", filtered.len(), state.models.len())
        }
        Step::Effort(_, id) => {
            lines.push(row(0, "Auto", vec![dim("recommended", theme)]));
            for (index, option) in state.efforts_for(id).iter().enumerate() {
                lines.push(row(
                    index + 1,
                    option.label.clone(),
                    option
                        .description
                        .clone()
                        .map(|text| vec![dim(text, theme)])
                        .unwrap_or_default(),
                ));
            }
            "Effort".to_owned()
        }
        Step::Utility => {
            let value = if state.selected == 0 {
                editor_spans(&state.utility, theme)
            } else if state.utility.text().is_empty() {
                vec![dim("type a model chain", theme)]
            } else {
                vec![Span::raw(state.utility.text().to_owned())]
            };
            lines.push(row(0, "Custom", value));
            for (index, (key, model)) in state.utility_entries.iter().enumerate() {
                lines.push(row(index + 1, key.clone(), vec![tag(model.clone(), theme)]));
            }
            lines.push(row(
                state.utility_entries.len() + 1,
                "None",
                vec![dim("turn the Utility model off", theme)],
            ));
            "Utility model".to_owned()
        }
    };
    (title, lines)
}

fn footer(step: &Step, width: u16, theme: &Theme) -> Line<'static> {
    let key = |text: &str| {
        Span::styled(
            format!(" {text} "),
            Style::default()
                .fg(theme.text_primary)
                .bg(theme.bg_highlight)
                .add_modifier(Modifier::BOLD),
        )
    };
    let keys: &[(&str, &str)] = match step {
        Step::Home => &[
            ("↑↓", "Move"),
            ("Enter", "Open"),
            ("a", "Auto"),
            ("s", "Save"),
            ("Esc", "Close"),
        ],
        Step::Model(_) => &[
            ("Type", "Filter"),
            ("↑↓", "Move"),
            ("Enter", "Choose"),
            ("Esc", "Back"),
        ],
        Step::Effort(..) => &[("↑↓", "Move"), ("Enter", "Choose"), ("Esc", "Back")],
        Step::Utility => &[
            ("Type", "Edit"),
            ("↑↓", "Move"),
            ("Enter", "Choose"),
            ("Esc", "Back"),
        ],
    };
    if width < 50 {
        return Line::from(
            keys.iter()
                .map(|(key, label)| format!("{key} {label}"))
                .collect::<Vec<_>>()
                .join(" · "),
        );
    }
    let mut spans = Vec::new();
    for (name, label) in keys {
        spans.push(key(name));
        spans.push(Span::raw(format!(" {label}  ")));
    }
    Line::from(spans)
}

fn popup_area(area: Rect, compact: bool, wanted_height: u16) -> Rect {
    if compact {
        return area;
    }
    let width = area.width.saturating_sub(4).min(108);
    let height = area.height.saturating_sub(2).min(wanted_height);
    Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    }
}

pub fn render_tier_editor_overlay(
    buf: &mut Buffer,
    area: Rect,
    _window: &mut super::modal_window::ModalWindowState,
    state: &mut TierEditorState,
    compact: bool,
    theme: &Theme,
) {
    state.hit_rows.clear();
    // Fit the frame to the current step so short lists do not leave an empty panel.
    let (_, intro_estimate) = intro(state, theme);
    let (_, list_estimate) = list(state, theme);
    let min_list = if matches!(state.step, Step::Model(_)) { 12 } else { 6 };
    let wanted = intro_estimate.len().max(3) + list_estimate.len().max(min_list) + 11;
    let popup = popup_area(area, compact, wanted.min(u16::MAX as usize) as u16);
    Clear.render(popup, buf);
    if popup.width < 30 || popup.height < 12 {
        Paragraph::new("Model setup needs a larger terminal. Resize it, or press Esc.")
            .style(Style::default().fg(theme.text_primary).bg(theme.bg_base))
            .wrap(Wrap { trim: true })
            .render(popup, buf);
        return;
    }
    let title = Line::from(vec![
        Span::styled(
            " DISTILL ",
            Style::default()
                .fg(theme.running)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("·", Style::default().fg(theme.gray_dim)),
        Span::styled(
            format!(" {} ", step_title(&state.step)),
            Style::default().fg(theme.text_secondary),
        ),
    ]);
    let badge = if state.auto.enabled {
        Span::styled(
            " Auto on ",
            Style::default()
                .fg(theme.accent_success)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(" Auto off ", Style::default().fg(theme.gray_bright))
    };
    let block = Block::default()
        .title(title)
        .title(Line::from(badge).alignment(Alignment::Right))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.prompt_border))
        .style(Style::default().bg(theme.bg_base));
    let inner = block.inner(popup);
    block.render(popup, buf);
    let content = inner.inner(Margin::new(1, 0));

    // Footer keys, with a divider when there is room.
    let footer_height = if content.height >= 20 { 2 } else { 1 };
    let footer_area = Rect {
        y: content.bottom().saturating_sub(footer_height),
        height: footer_height,
        ..content
    };
    if footer_height > 1 {
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(theme.prompt_border))
            .render(footer_area, buf);
    }
    Paragraph::new(footer(&state.step, content.width, theme))
        .style(Style::default().fg(theme.text_secondary))
        .render(
            Rect {
                y: footer_area.bottom() - 1,
                height: 1,
                ..footer_area
            },
            buf,
        );

    // Status line above the footer.
    let status = state.status.clone().or_else(|| {
        (state.step == Step::Home && state.dirty())
            .then(|| ("Unsaved changes. Press s to save.".to_owned(), false))
    });
    let status_paragraph = status.as_ref().map(|(text, error)| {
        Paragraph::new(Line::from(Span::styled(
            text.clone(),
            Style::default().fg(if *error {
                theme.accent_error
            } else {
                theme.accent_success
            }),
        )))
        .wrap(Wrap { trim: true })
    });
    let status_height = status_paragraph
        .as_ref()
        .map_or(0, |paragraph| paragraph.line_count(content.width).clamp(1, 2) as u16);
    if let Some(paragraph) = status_paragraph {
        paragraph.render(
            Rect {
                y: footer_area.y.saturating_sub(status_height),
                height: status_height,
                ..content
            },
            buf,
        );
    }
    let body = Rect {
        height: footer_area
            .y
            .saturating_sub(status_height)
            .saturating_sub(content.y),
        ..content
    };

    // Explanation card with the step icon, as in onboarding.
    let (icon, intro_lines) = intro(state, theme);
    let show_icon = body.width >= 60;
    let text_width = body.width.saturating_sub(if show_icon { 12 } else { 4 }).max(1);
    let intro_paragraph = Paragraph::new(intro_lines)
        .style(Style::default().fg(theme.text_secondary))
        .wrap(Wrap { trim: false });
    let intro_height = (intro_paragraph.line_count(text_width) as u16 + 2)
        .max(if show_icon { 5 } else { 3 })
        .min(body.height.saturating_sub(5).max(3));
    let intro_area = Rect {
        height: intro_height,
        ..body
    };
    let intro_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.prompt_border));
    let intro_inner = intro_block.inner(intro_area).inner(Margin::new(1, 0));
    intro_block.render(intro_area, buf);
    let text_area = if show_icon {
        let icon_bg = blend_color(theme.bg_base, theme.accent_thinking, 0.12)
            .map(crate::theme::quantize)
            .unwrap_or(theme.bg_highlight);
        Paragraph::new(icon)
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.accent_thinking).bg(icon_bg))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme.accent_thinking)),
            )
            .render(
                Rect::new(intro_inner.x, intro_inner.y, 5, intro_inner.height.min(3)),
                buf,
            );
        Rect {
            x: intro_inner.x + 8,
            width: intro_inner.width.saturating_sub(8),
            ..intro_inner
        }
    } else {
        intro_inner
    };
    intro_paragraph.render(text_area, buf);

    // Choices panel.
    let choices_area = Rect {
        y: intro_area.bottom(),
        height: body.bottom().saturating_sub(intro_area.bottom()),
        ..body
    };
    let (panel_title, lines) = list(state, theme);
    let choices_block = Block::default()
        .title(Line::from(format!(" {panel_title} ")).style(Style::default().fg(theme.running)))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.prompt_border));
    let rows_area = choices_block.inner(choices_area).inner(Margin::new(1, 0));
    choices_block.render(choices_area, buf);
    let viewport = rows_area.height as usize;
    if viewport == 0 {
        return;
    }
    if let Some(position) = lines
        .iter()
        .position(|line| line.index == Some(state.selected))
    {
        // Keep the heading above the first rows visible when scrolled to the top.
        let first_row = lines.iter().position(|line| line.index.is_some());
        if Some(position) == first_row {
            state.scroll = 0;
        } else if position < state.scroll {
            state.scroll = position;
        } else if position >= state.scroll + viewport {
            state.scroll = position + 1 - viewport;
        }
    }
    state.scroll = state.scroll.min(lines.len().saturating_sub(viewport));
    let selection = selection_style(theme);
    for (offset, line) in lines.iter().skip(state.scroll).take(viewport).enumerate() {
        let y = rows_area.y + offset as u16;
        let rect = Rect::new(rows_area.x, y, rows_area.width, 1);
        let Some(index) = line.index else {
            Line::from(line.label.clone()).render(Rect { x: rect.x + 1, ..rect }, buf);
            continue;
        };
        let selected = index == state.selected;
        let mut spans = vec![Span::styled(
            if selected { " ◉ " } else { " ○ " },
            Style::default().fg(if selected {
                theme.accent_system
            } else {
                theme.text_secondary
            }),
        )];
        spans.extend(line.label.iter().cloned());
        let left = Line::from(spans.clone()).width();
        let value_width = Line::from(line.value.clone()).width();
        let room = usize::from(rect.width);
        if left + value_width + 2 <= room {
            spans.push(Span::raw(" ".repeat(room - left - value_width)));
            spans.extend(line.value.iter().cloned());
        } else {
            spans.push(Span::raw("  "));
            spans.extend(line.value.iter().cloned());
        }
        let style = if selected {
            selection
        } else {
            Style::default().fg(theme.text_primary)
        };
        if selected {
            buf.set_style(rect, selection);
        }
        Line::from(spans).style(style).render(rect, buf);
        if selected {
            buf[(rect.x, y)].set_symbol("▏").set_fg(theme.accent_system);
        }
        state.hit_rows.push((rect, index));
    }
}

#[cfg(test)]
mod tests {
    use super::{AutoSummary, Step, Tier, TierEditorOutcome, TierEditorState};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind};
    use distill_shell::sampling::types::ReasoningEffort;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn type_text(state: &mut TierEditorState, text: &str) {
        for character in text.chars() {
            state.handle_key(&key(KeyCode::Char(character)));
        }
    }

    fn models() -> crate::acp::ModelState {
        let mut models = crate::acp::ModelState::default();
        for (id, name) in [
            ("chatgpt/gpt-6-sol", "GPT-6-Sol"),
            ("chatgpt/gpt-6-luna", "GPT-6-Luna"),
            ("claude/claude-opus-5-5", "Claude Opus 5.5"),
        ] {
            let id = agent_client_protocol::ModelId::new(id);
            models.available.insert(
                id.clone(),
                agent_client_protocol::ModelInfo::new(id, name).meta(
                    serde_json::json!({ "supportsReasoningEffort": true })
                        .as_object()
                        .cloned(),
                ),
            );
        }
        models.current = Some(agent_client_protocol::ModelId::new("chatgpt/gpt-6-sol"));
        models.effort_auto = true;
        models.worker_model = Some(agent_client_protocol::ModelId::new("chatgpt/gpt-6-luna"));
        models.worker_effort = Some(ReasoningEffort::Medium);
        models
    }

    fn routes(enabled: bool, count: usize) -> AutoSummary {
        AutoSummary {
            enabled,
            accounts: count,
            routes: (0..count)
                .map(|_| {
                    (
                        "Claude · work".to_owned(),
                        "claude/claude-opus-5-5".to_owned(),
                        "claude/claude-opus-5-5".to_owned(),
                    )
                })
                .collect(),
            ..Default::default()
        }
    }

    fn editor(auto: AutoSummary) -> TierEditorState {
        TierEditorState::from_parts(&models(), auto, String::new(), vec![], || None)
    }

    /// Saving without edits must not rewrite anything: the screen opens on the
    /// saved picks in the same `model effort` form Save submits.
    #[test]
    fn opens_on_the_saved_picks_and_closes_clean_without_submitting() {
        let mut state = editor(routes(false, 0));
        assert_eq!(state.submission().0, "chatgpt/gpt-6-sol auto");
        assert_eq!(state.submission().1, "chatgpt/gpt-6-luna medium");
        assert!(!state.dirty());
        assert_eq!(
            state.handle_key(&key(KeyCode::Char('s'))),
            TierEditorOutcome::Cancelled
        );
    }

    /// With Auto on the session model is `auto`; the Main shown and saved must be
    /// the configured one, never the routing placeholder.
    #[test]
    fn auto_session_edits_the_configured_main_not_the_auto_placeholder() {
        let mut models = models();
        models.current = Some(agent_client_protocol::ModelId::new("auto"));
        let state = TierEditorState::from_parts(&models, routes(true, 1), String::new(), vec![], || {
            Some("claude/claude-opus-5-5".to_owned())
        });
        assert_eq!(state.main.as_deref(), Some("claude/claude-opus-5-5"));
    }

    /// Main → filter → model → effort returns home with a draft that Save submits.
    #[test]
    fn picking_a_main_model_and_effort_then_saving_submits_the_pair() {
        let mut state = editor(routes(false, 0));
        assert_eq!(state.selected, 2, "opens on the Main row");
        state.handle_key(&key(KeyCode::Enter));
        assert_eq!(state.step, Step::Model(Tier::Main));
        type_text(&mut state, "opus");
        assert_eq!(state.filtered().len(), 1);
        state.handle_key(&key(KeyCode::Enter));
        assert!(matches!(state.step, Step::Effort(Tier::Main, _)));
        let high = 1 + state
            .efforts_for("claude/claude-opus-5-5")
            .iter()
            .position(|option| option.value == ReasoningEffort::High)
            .expect("built-in effort menu offers high");
        for _ in 0..high {
            state.handle_key(&key(KeyCode::Down));
        }
        state.handle_key(&key(KeyCode::Enter));
        assert_eq!(state.step, Step::Home);
        assert!(state.dirty());
        assert_eq!(
            state.handle_key(&key(KeyCode::Char('s'))),
            TierEditorOutcome::Submitted {
                main: "claude/claude-opus-5-5 high".into(),
                worker: "chatgpt/gpt-6-luna medium".into(),
                utility: String::new(),
                execution_mode: distill_tool_types::ExecutionModeChoice::Auto,
            }
        );
    }

    #[test]
    fn execution_mode_cycles_exclusively_and_saves_the_selected_mode() {
        use distill_tool_types::ExecutionModeChoice as Choice;
        let mut state = editor(routes(false, 0));
        state.selected = super::HOME_EXECUTION_MODE;
        for expected in [
            Choice::Normal,
            Choice::Ultrafast,
            Choice::Ultracode,
            Choice::Ultrasave,
            Choice::Auto,
        ] {
            assert_eq!(state.handle_key(&key(KeyCode::Enter)), TierEditorOutcome::Changed);
            assert_eq!(state.execution_mode, expected);
        }
        assert!(!state.dirty(), "cycling back to Auto restores the initial selection");
        for _ in 0..4 {
            state.handle_key(&key(KeyCode::Enter));
        }
        let outcome = state.handle_key(&key(KeyCode::Char('s')));
        assert!(matches!(
            outcome,
            TierEditorOutcome::Submitted { execution_mode: Choice::Ultrasave, .. }
        ));
    }

    /// The Worker list starts with None, which hands all work to Main.
    #[test]
    fn worker_none_clears_the_worker() {
        let mut state = editor(routes(false, 0));
        state.handle_key(&key(KeyCode::Down));
        state.handle_key(&key(KeyCode::Enter));
        assert_eq!(state.step, Step::Model(Tier::Worker));
        state.handle_key(&key(KeyCode::Home));
        state.handle_key(&key(KeyCode::Enter));
        assert_eq!(state.submission().1, "");
    }

    /// Auto toggles in place when routes exist; without routes the row leads to
    /// Accounts, where routes are created.
    #[test]
    fn auto_row_toggles_with_routes_and_opens_accounts_without_them() {
        let mut state = editor(routes(false, 2));
        assert_eq!(
            state.handle_key(&key(KeyCode::Char('a'))),
            TierEditorOutcome::ToggleAutoRouting
        );
        assert_eq!(
            state.handle_key(&key(KeyCode::Char('a'))),
            TierEditorOutcome::Unchanged,
            "no second toggle while one is in flight"
        );
        state.auto_routing_result(Ok(true));
        assert!(state.auto.enabled && !state.auto_pending);

        let mut state = editor(routes(false, 0));
        assert_eq!(
            state.handle_key(&key(KeyCode::Char('a'))),
            TierEditorOutcome::OpenAccounts
        );
    }

    /// Esc steps back from a picker, and closing with unsaved picks needs a second Esc.
    #[test]
    fn escape_backs_out_of_pickers_and_guards_unsaved_changes() {
        let mut state = editor(routes(false, 0));
        state.handle_key(&key(KeyCode::Enter));
        assert_eq!(
            state.handle_key(&key(KeyCode::Esc)),
            TierEditorOutcome::Changed
        );
        assert_eq!(state.step, Step::Home);
        assert_eq!(
            state.handle_key(&key(KeyCode::Esc)),
            TierEditorOutcome::Cancelled,
            "nothing changed, so one Esc closes"
        );

        state.activate_home(super::HOME_EXECUTION_MODE);
        assert_eq!(
            state.handle_key(&key(KeyCode::Esc)),
            TierEditorOutcome::Changed
        );
        assert_eq!(
            state.handle_key(&key(KeyCode::Esc)),
            TierEditorOutcome::Cancelled
        );
    }

    /// Every rendered row is clickable, and the layout survives a tiny terminal.
    #[test]
    fn renders_clickable_rows_and_degrades_in_a_small_terminal() {
        use ratatui::buffer::Buffer;
        use ratatui::layout::Rect;
        let theme = crate::theme::Theme::current();
        let mut window = crate::views::modal_window::ModalWindowState::new();
        let mut state = editor(routes(true, 5));
        let area = Rect::new(0, 0, 100, 34);
        let mut buf = Buffer::empty(area);
        super::render_tier_editor_overlay(&mut buf, area, &mut window, &mut state, false, &theme);
        let text: String = buf.content().iter().map(|cell| cell.symbol()).collect();
        for expected in ["Auto routing is on", "Accounts & routes", "+2 more", "Execution", "Auto"] {
            assert!(text.contains(expected), "missing {expected}");
        }
        assert_eq!(state.hit_rows.len(), super::HOME_ROWS);
        let (rect, _) = state.hit_rows[super::HOME_WORKER];
        assert_eq!(
            state.handle_mouse(MouseEventKind::Down(MouseButton::Left), rect.x + 3, rect.y),
            TierEditorOutcome::Changed
        );
        assert_eq!(state.step, Step::Model(Tier::Worker));

        let small = Rect::new(0, 0, 28, 10);
        let mut buf = Buffer::empty(small);
        super::render_tier_editor_overlay(&mut buf, small, &mut window, &mut state, false, &theme);
        assert!(state.hit_rows.is_empty());
    }
}
