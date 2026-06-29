//! Keyboard shortcut handler.
//!
//! Maps key combinations to editor actions.

use crate::app::EditorAction;
use winit::keyboard::{Key, ModifiersState};

/// Handles keyboard shortcuts and dispatches them as editor actions.
#[derive(Debug, Default)]
pub struct ShortcutHandler {
    /// Tracked modifiers state.
    pub modifiers: ModifiersState,
}

/// The action triggered keyboard shortcut.
#[derive(Debug, Clone, Copy)]
pub enum ShortcutAction {
    NewScene,
    OpenScene,
    SaveScene,
    SaveSceneAs,
    Undo,
    Redo,
    Delete,
    Duplicate,
    Rename,
    Copy,
    Paste,
    FocusSelection,
    Deselect,
    Exit,
}

impl ShortcutHandler {
    /// Create a new shortcut handler.
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the tracked modifier state.
    pub fn set_modifiers(&mut self, state: ModifiersState) {
        self.modifiers = state;
    }

    /// Process a key press and return the corresponding action, if any.
    pub fn handle_key_press(&self, key: &Key) -> Option<ShortcutAction> {
        let ctrl = self.modifiers.control_key();
        let shift = self.modifiers.shift_key();

        match key {
            // Ctrl+N → New Scene
            Key::Character(c) if ctrl && !shift && c.eq_ignore_ascii_case("n") => {
                Some(ShortcutAction::NewScene)
            },
            // Ctrl+O → Open Scene
            Key::Character(c) if ctrl && !shift && c.eq_ignore_ascii_case("o") => {
                Some(ShortcutAction::OpenScene)
            },
            // Ctrl+S → Save (or Save As if no path)
            Key::Character(c) if ctrl && !shift && c.eq_ignore_ascii_case("s") => {
                Some(ShortcutAction::SaveScene)
            },
            // Ctrl+Shift+S → Save As
            Key::Character(c) if ctrl && shift && c.eq_ignore_ascii_case("s") => {
                Some(ShortcutAction::SaveSceneAs)
            },
            // Ctrl+Z → Undo
            Key::Character(c) if ctrl && !shift && c.eq_ignore_ascii_case("z") => {
                Some(ShortcutAction::Undo)
            },
            // Ctrl+Y (or Ctrl+Shift+Z) → Redo
            Key::Character(c) if ctrl && !shift && c.eq_ignore_ascii_case("y") => {
                Some(ShortcutAction::Redo)
            },
            Key::Character(c) if ctrl && shift && c.eq_ignore_ascii_case("z") => {
                Some(ShortcutAction::Redo)
            },
            // Delete → Delete selected node
            Key::Named(winit::keyboard::NamedKey::Delete) => Some(ShortcutAction::Delete),
            // Ctrl+D → Duplicate selected node
            Key::Character(c) if ctrl && !shift && c.eq_ignore_ascii_case("d") => {
                Some(ShortcutAction::Duplicate)
            },
            // Ctrl+C → Copy selected node
            Key::Character(c) if ctrl && !shift && c.eq_ignore_ascii_case("c") => {
                Some(ShortcutAction::Copy)
            },
            // Ctrl+V → Paste from clipboard
            Key::Character(c) if ctrl && !shift && c.eq_ignore_ascii_case("v") => {
                Some(ShortcutAction::Paste)
            },
            // F2 → Rename selected node
            Key::Named(winit::keyboard::NamedKey::F2) => Some(ShortcutAction::Rename),
            // F → Focus on selection
            Key::Character(c) if !ctrl && c.eq_ignore_ascii_case("f") => {
                Some(ShortcutAction::FocusSelection)
            },
            // Escape → Deselect
            Key::Named(winit::keyboard::NamedKey::Escape) => Some(ShortcutAction::Deselect),
            _ => None,
        }
    }

    /// Convert a shortcut action to an EditorAction (for the pending action queue).
    pub fn to_editor_action(action: ShortcutAction) -> EditorAction {
        match action {
            ShortcutAction::NewScene => EditorAction::NewScene,
            ShortcutAction::OpenScene => EditorAction::OpenScene,
            ShortcutAction::SaveScene => EditorAction::SaveScene,
            ShortcutAction::SaveSceneAs => EditorAction::SaveSceneAs,
            ShortcutAction::Undo => EditorAction::Undo,
            ShortcutAction::Redo => EditorAction::Redo,
            ShortcutAction::Delete => EditorAction::DeleteSelected,
            ShortcutAction::Duplicate => EditorAction::DuplicateSelected,
            ShortcutAction::Rename => EditorAction::RenameSelected,
            ShortcutAction::Copy => EditorAction::CopySelected,
            ShortcutAction::Paste => EditorAction::Paste,
            ShortcutAction::FocusSelection => EditorAction::FocusSelection,
            ShortcutAction::Deselect => EditorAction::Deselect,
            ShortcutAction::Exit => EditorAction::Exit,
        }
    }
}
