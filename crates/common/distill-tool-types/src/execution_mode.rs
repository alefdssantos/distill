// Modified for Distill by Samuel Fajreldines, 2026.
use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
};

use serde::{Deserialize, Serialize};

/// One session execution policy. Modes cannot be enabled simultaneously.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum ExecutionMode {
    #[default]
    Normal,
    Ultrafast,
    Ultracode,
    Ultrasave,
}

/// Relative commands are resolved against the server's current session mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionModeOperation {
    #[default]
    Set,
    Toggle,
    Disable,
}

impl ExecutionMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Ultrafast => "UltraFast",
            Self::Ultracode => "UltraCode",
            Self::Ultrasave => "UltraSave",
        }
    }
}

/// The user's selection: a fixed mode, or Auto, where Jev picks the effective
/// `ExecutionMode` per request and may change it during execution.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionModeChoice {
    #[default]
    Auto,
    Normal,
    Ultrafast,
    Ultracode,
    Ultrasave,
}

impl ExecutionModeChoice {
    /// The fixed mode, or `None` for Auto.
    pub fn fixed(self) -> Option<ExecutionMode> {
        match self {
            Self::Auto => None,
            Self::Normal => Some(ExecutionMode::Normal),
            Self::Ultrafast => Some(ExecutionMode::Ultrafast),
            Self::Ultracode => Some(ExecutionMode::Ultracode),
            Self::Ultrasave => Some(ExecutionMode::Ultrasave),
        }
    }

    pub fn label(self) -> &'static str {
        self.fixed().map_or("Auto", ExecutionMode::label)
    }
}

impl From<ExecutionMode> for ExecutionModeChoice {
    fn from(mode: ExecutionMode) -> Self {
        match mode {
            ExecutionMode::Normal => Self::Normal,
            ExecutionMode::Ultrafast => Self::Ultrafast,
            ExecutionMode::Ultracode => Self::Ultracode,
            ExecutionMode::Ultrasave => Self::Ultrasave,
        }
    }
}

/// The user's selection is shared by the root and every descendant. In Auto,
/// each agent chooses its own effective mode for the action it is doing.
#[derive(Debug, Default)]
pub struct AtomicExecutionMode {
    selection: Arc<AtomicU8>,
    auto_mode: AtomicU8,
}

const AUTO_BIT: u8 = 0x80;

impl AtomicExecutionMode {
    pub fn new(mode: ExecutionMode) -> Self {
        Self {
            selection: Arc::new(AtomicU8::new(mode as u8)),
            auto_mode: AtomicU8::new(mode as u8),
        }
    }

    /// Inherit live user choices without sharing Auto decisions with siblings.
    pub fn inherited(&self) -> Self {
        Self {
            selection: self.selection.clone(),
            auto_mode: AtomicU8::new(self.load() as u8),
        }
    }

    fn decode(value: u8) -> ExecutionMode {
        match value {
            0 => ExecutionMode::Normal,
            1 => ExecutionMode::Ultrafast,
            2 => ExecutionMode::Ultracode,
            3 => ExecutionMode::Ultrasave,
            _ => unreachable!("only ExecutionMode values can be stored"),
        }
    }

    /// The effective mode, whether fixed or chosen by Auto.
    pub fn load(&self) -> ExecutionMode {
        let selection = self.selection.load(Ordering::Relaxed);
        Self::decode(if selection == AUTO_BIT {
            self.auto_mode.load(Ordering::Relaxed)
        } else {
            selection
        })
    }

    pub fn is_auto(&self) -> bool {
        self.selection.load(Ordering::Relaxed) == AUTO_BIT
    }

    /// Fixes the mode and leaves Auto.
    pub fn store(&self, mode: ExecutionMode) {
        self.selection.store(mode as u8, Ordering::Relaxed);
    }

    /// Select Auto, initially keeping this effective mode.
    pub fn store_auto(&self, mode: ExecutionMode) {
        self.auto_mode.store(mode as u8, Ordering::Relaxed);
        self.selection.store(AUTO_BIT, Ordering::Relaxed);
    }

    /// Apply a decision without ever replacing a user's fixed choice.
    pub fn pick_auto(&self, mode: ExecutionMode) -> bool {
        if !self.is_auto() {
            return false;
        }
        self.auto_mode.store(mode as u8, Ordering::Relaxed);
        self.is_auto()
    }

    /// The user's selection: Auto or the fixed mode.
    pub fn choice(&self) -> ExecutionModeChoice {
        let selection = self.selection.load(Ordering::Relaxed);
        if selection == AUTO_BIT {
            ExecutionModeChoice::Auto
        } else {
            Self::decode(selection).into()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_keeps_the_effective_mode_until_a_fixed_choice_replaces_it() {
        let mode = AtomicExecutionMode::default();
        assert_eq!(mode.choice(), ExecutionModeChoice::Normal);
        mode.store_auto(ExecutionMode::Ultrasave);
        assert!(mode.is_auto());
        assert_eq!(mode.load(), ExecutionMode::Ultrasave);
        assert_eq!(mode.choice(), ExecutionModeChoice::Auto);
        mode.store(ExecutionMode::Ultracode);
        assert!(!mode.is_auto());
        assert_eq!(mode.choice(), ExecutionModeChoice::Ultracode);
        assert_eq!(
            serde_json::to_value(ExecutionModeChoice::Auto).unwrap(),
            "auto"
        );
    }

    #[test]
    fn descendants_inherit_user_choices_but_adapt_auto_independently() {
        let root = AtomicExecutionMode::default();
        root.store_auto(ExecutionMode::Ultracode);
        let child = root.inherited();
        let sibling = root.inherited();
        let grandchild = child.inherited();
        assert!(child.pick_auto(ExecutionMode::Ultrafast));
        assert!(grandchild.pick_auto(ExecutionMode::Ultrasave));
        assert_eq!(root.load(), ExecutionMode::Ultracode);
        assert_eq!(sibling.load(), ExecutionMode::Ultracode);
        assert_eq!(child.load(), ExecutionMode::Ultrafast);
        assert_eq!(grandchild.load(), ExecutionMode::Ultrasave);
        root.store(ExecutionMode::Normal);
        for agent in [&root, &child, &sibling, &grandchild] {
            assert_eq!(agent.choice(), ExecutionModeChoice::Normal);
            assert!(!agent.pick_auto(ExecutionMode::Ultracode));
            assert_eq!(agent.load(), ExecutionMode::Normal);
        }
        root.store_auto(ExecutionMode::Ultracode);
        assert_eq!(child.choice(), ExecutionModeChoice::Auto);
        assert!(child.pick_auto(ExecutionMode::Ultrafast));
        assert_eq!(root.load(), ExecutionMode::Ultracode);
    }
}
