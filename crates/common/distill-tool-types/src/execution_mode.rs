// Modified for Distill by Samuel Fajreldines, 2026.
use std::sync::atomic::{AtomicU8, Ordering};

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

/// The root and its task descendants observe the same, indivisible mode.
/// The high bit records Auto; the low bits hold the effective mode.
#[derive(Debug, Default)]
pub struct AtomicExecutionMode(AtomicU8);

const AUTO_BIT: u8 = 0x80;

impl AtomicExecutionMode {
    pub fn new(mode: ExecutionMode) -> Self {
        Self(AtomicU8::new(mode as u8))
    }

    /// The effective mode, whether fixed or chosen by Auto.
    pub fn load(&self) -> ExecutionMode {
        match self.0.load(Ordering::Relaxed) & !AUTO_BIT {
            0 => ExecutionMode::Normal,
            1 => ExecutionMode::Ultrafast,
            2 => ExecutionMode::Ultracode,
            3 => ExecutionMode::Ultrasave,
            _ => unreachable!("only ExecutionMode values can be stored"),
        }
    }

    pub fn is_auto(&self) -> bool {
        self.0.load(Ordering::Relaxed) & AUTO_BIT != 0
    }

    /// Fixes the mode and leaves Auto.
    pub fn store(&self, mode: ExecutionMode) {
        self.0.store(mode as u8, Ordering::Relaxed);
    }

    /// Auto's pick; stays in Auto.
    pub fn store_auto(&self, mode: ExecutionMode) {
        self.0.store(mode as u8 | AUTO_BIT, Ordering::Relaxed);
    }

    /// The user's selection: Auto or the fixed mode.
    pub fn choice(&self) -> ExecutionModeChoice {
        if self.is_auto() {
            ExecutionModeChoice::Auto
        } else {
            self.load().into()
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
        assert_eq!(serde_json::to_value(ExecutionModeChoice::Auto).unwrap(), "auto");
    }
}
