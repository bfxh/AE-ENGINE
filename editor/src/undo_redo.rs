//! Advanced undo/redo system — placeholder for P2.
//!
//! Will extend the basic command history with:
//! - Command grouping (macro commands)
//! - Serialisable command history for session restore
//! - Branching undo tree
//! - Memory-bounded history with automatic pruning
//!
//! **Status**: Coming in P2.
//!
//! For now, re-exports the basic CommandHistory from the commands module.

/// Placeholder for the advanced undo/redo manager (P2).
pub struct UndoRedoManager;

impl UndoRedoManager {
    /// Create a new undo/redo manager (placeholder).
    pub fn new() -> Self {
        log::debug!("Advanced UndoRedoManager: not yet implemented (planned for P2)");
        Self
    }
}

impl Default for UndoRedoManager {
    fn default() -> Self {
        Self::new()
    }
}
