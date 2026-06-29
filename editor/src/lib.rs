//! Wasteland Editor library facade.
//!
//! Re-exports all editor modules so integration tests (and future external
//! consumers) can access the public API without a binary entry point.

pub mod app;
pub mod camera;
pub mod commands;
pub mod engine_bridge;
pub mod engine_types;
pub mod gizmo;
pub mod mcp;
pub mod panels;
pub mod plugin;
pub mod render;
pub mod scene;
pub mod scene_io;
pub mod selection;
pub mod settings;
pub mod shortcut;
pub mod undo_redo;
