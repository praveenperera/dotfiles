//! Keybindings module stub for future expansion
//!
//! Will eventually support:
//! - Custom key mappings
//! - Keymap configuration
//! - Chord sequences

use ratatui::crossterm::event::{KeyCode, KeyModifiers};

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Key {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl Key {
    #[allow(dead_code)]
    pub fn new(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::NONE,
        }
    }

    #[allow(dead_code)]
    pub fn with_ctrl(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::CONTROL,
        }
    }
}
