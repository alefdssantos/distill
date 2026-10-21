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

/// The root and its task descendants observe the same, indivisible mode.
#[derive(Debug, Default)]
pub struct AtomicExecutionMode(AtomicU8);

impl AtomicExecutionMode {
    pub fn new(mode: ExecutionMode) -> Self {
        Self(AtomicU8::new(mode as u8))
    }

    pub fn load(&self) -> ExecutionMode {
        match self.0.load(Ordering::Relaxed) {
            0 => ExecutionMode::Normal,
            1 => ExecutionMode::Ultrafast,
            2 => ExecutionMode::Ultracode,
            3 => ExecutionMode::Ultrasave,
            _ => unreachable!("only ExecutionMode values can be stored"),
        }
    }

    pub fn store(&self, mode: ExecutionMode) {
        self.0.store(mode as u8, Ordering::Relaxed);
    }
}
