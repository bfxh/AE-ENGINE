//! Editor application state and core logic.
//!
//! The `EditorApp` struct is the central state container for the entire editor.
//! It owns the scene, camera, selection, command history, and UI state.

use crate::camera::EditorCamera;
use crate::commands::CommandHistory;
use crate::engine_bridge::EngineBridge;
use crate::gizmo::{Gizmo, GizmoMode};
use crate::panels::asset_browser::AssetBrowserPanel;
use crate::panels::hierarchy::HierarchyPanel;
use crate::panels::inspector::InspectorPanel;
use crate::panels::console::ConsolePanel;
use crate::panels::stats::StatsPanel;
use crate::panels::about::AboutPanel;
use crate::panels::search::SearchPanel;
use crate::panels::world_settings::WorldSettingsPanel;
use crate::panels::layers::LayersPanel;
use crate::panels::settings_panel::SettingsPanel;
use crate::panels::viewport::ViewportPanel;
use crate::panels::material_editor::MaterialEditorPanel;
use crate::panels::terrain_editor::TerrainEditorPanel;
use crate::panels::animation_timeline::AnimationTimelinePanel;
use crate::panels::particle_editor::ParticleEditorPanel;
use crate::panels::physics_debug::PhysicsDebugPanel;
use crate::panels::bookmarks::BookmarksPanel;
use crate::panels::view_modes::ViewModesPanel;
use crate::panels::measure_tool::MeasureToolPanel;
use crate::panels::sequencer::SequencerPanel;
use crate::panels::behavior_tree_editor::BehaviorTreeEditorPanel;
use crate::panels::shader_graph::ShaderGraphPanel;
use crate::panels::landscape_spline::LandscapeSplinePanel;
use crate::panels::foliage_editor::FoliageEditorPanel;
use crate::panels::skeleton_editor::SkeletonEditorPanel;
use crate::panels::anim_blueprint::AnimBlueprintPanel;
use crate::panels::control_rig::ControlRigPanel;
use crate::panels::variant_manager::VariantManagerPanel;
use crate::panels::reference_viewer::ReferenceViewerPanel;
use crate::panels::visual_logger::VisualLoggerPanel;
use crate::panels::curve_editor::CurveEditorPanel;
use crate::panels::uv_editor::UvEditorPanel;
use crate::panels::node_editor::NodeEditorPanel;
use crate::panels::dialogue_tree::DialogueTreePanel;
use crate::panels::quest_editor::QuestEditorPanel;
use crate::panels::navmesh_editor::NavmeshEditorPanel;
use crate::panels::audio_mixer::AudioMixerPanel;
use crate::panels::lighting_panel::LightingPanel;
use crate::panels::post_process::PostProcessPanel;
use crate::panels::constraints_panel::ConstraintsPanel;
use crate::panels::modifiers_panel::ModifiersPanel;
use crate::panels::spreadsheet::SpreadsheetPanel;
use crate::panels::geometry_nodes::GeometryNodesPanel;
use crate::panels::compositor::CompositorPanel;
use crate::panels::mcp_debug::McpDebugPanel;
use crate::mcp::bridge::McpHttpBridge;
use crate::mcp::server::{McpServer, ToolContext};
use crate::mcp::transport::{MemoryTransport, MemoryTransportHandle};
use crate::plugin::builtin;
use crate::plugin::registry::PluginRegistry;
use crate::scene::Scene;
use crate::selection::Selection;

/// Actions that can be queued for deferred execution in the event loop.
///
/// File dialog operations (Open, Save As) and exit confirmation must be
/// handled outside of egui closures because `rfd::FileDialog` needs access
/// to the winit event loop.
#[derive(Debug, Clone)]
pub enum EditorAction {
    /// Create a new empty scene.
    NewScene,
    /// Open a scene file via file dialog.
    OpenScene,
    /// Open a scene from a specific file path (e.g., from Recent Files).
    OpenSceneFromPath(String),
    /// Save the current scene (uses Save As if no path set).
    SaveScene,
    /// Save the scene to a new file path.
    SaveSceneAs,
    /// Undo the last command.
    Undo,
    /// Redo the last undone command.
    Redo,
    /// Delete the currently selected node.
    DeleteSelected,
    /// Duplicate the currently selected node (and its subtree).
    DuplicateSelected,
    /// Start inline rename of the currently selected node.
    RenameSelected,
    /// Copy the currently selected node (and its subtree) to the clipboard.
    CopySelected,
    /// Paste the clipboard content as a new node.
    Paste,
    /// Focus the camera on the selected node.
    FocusSelection,
    /// Clear the current selection.
    Deselect,
    /// Request application exit (checks dirty flag).
    Exit,

    // ---- Gizmo mode switches ----
    /// Switch gizmo to Translate mode.
    GizmoTranslate,
    /// Switch gizmo to Rotate mode.
    GizmoRotate,
    /// Switch gizmo to Scale mode.
    GizmoScale,
}

/// Central application state for the Wasteland Editor.
pub struct EditorApp {
    /// The scene being edited.
    pub scene: Scene,

    /// Editor camera controller.
    pub camera: EditorCamera,

    /// Current selection state.
    pub selection: Selection,

    /// Command history for undo/redo.
    pub command_history: CommandHistory,

    /// Transform gizmo state.
    pub gizmo: Gizmo,

    /// (node_id, transform_before_gizmo_drag) — captured when a gizmo drag starts,
    /// committed as a SetTransformCommand when the drag ends.
    pub gizmo_drag_start: Option<(u64, crate::scene::NodeTransform)>,

    /// Asset browser panel state.
    pub asset_browser: AssetBrowserPanel,

    /// Hierarchy panel state (persistent across frames).
    pub hierarchy_panel: HierarchyPanel,

    /// Inspector panel state (tracks edit batches for undo).
    pub inspector_panel: InspectorPanel,

    /// Viewport panel state (show_grid, view_mode, etc.).
    pub viewport_panel: ViewportPanel,

    /// Console panel state.
    pub console_panel: Option<ConsolePanel>,

    /// Statistics panel state.
    pub stats_panel: Option<StatsPanel>,

    /// About dialog state.
    pub about_panel: Option<AboutPanel>,

    /// Search panel state.
    pub search_panel: Option<SearchPanel>,

    /// World settings panel state.
    pub world_settings_panel: Option<WorldSettingsPanel>,

    /// Layers panel state.
    pub layers_panel: Option<LayersPanel>,

    /// Settings panel state.
    pub settings_panel: Option<SettingsPanel>,

    /// Material editor panel.
    pub material_editor_panel: Option<MaterialEditorPanel>,

    /// Terrain editor panel.
    pub terrain_editor_panel: Option<TerrainEditorPanel>,

    /// Animation timeline panel.
    pub animation_timeline_panel: Option<AnimationTimelinePanel>,

    /// Particle editor panel.
    pub particle_editor_panel: Option<ParticleEditorPanel>,

    /// Physics debug panel.
    pub physics_debug_panel: Option<PhysicsDebugPanel>,

    /// Bookmarks panel.
    pub bookmarks_panel: Option<BookmarksPanel>,

    /// View modes panel.
    pub view_modes_panel: Option<ViewModesPanel>,

    /// Measure tool panel.
    pub measure_tool_panel: Option<MeasureToolPanel>,
    pub sequencer_panel: Option<SequencerPanel>,
    pub behavior_tree_editor_panel: Option<BehaviorTreeEditorPanel>,
    pub shader_graph_panel: Option<ShaderGraphPanel>,
    pub landscape_spline_panel: Option<LandscapeSplinePanel>,
    pub foliage_editor_panel: Option<FoliageEditorPanel>,
    pub skeleton_editor_panel: Option<SkeletonEditorPanel>,
    pub anim_blueprint_panel: Option<AnimBlueprintPanel>,
    pub control_rig_panel: Option<ControlRigPanel>,
    pub variant_manager_panel: Option<VariantManagerPanel>,
    pub reference_viewer_panel: Option<ReferenceViewerPanel>,
    pub visual_logger_panel: Option<VisualLoggerPanel>,
    pub curve_editor_panel: Option<CurveEditorPanel>,
    pub uv_editor_panel: Option<UvEditorPanel>,
    pub node_editor_panel: Option<NodeEditorPanel>,
    pub dialogue_tree_panel: Option<DialogueTreePanel>,
    pub quest_editor_panel: Option<QuestEditorPanel>,
    pub navmesh_editor_panel: Option<NavmeshEditorPanel>,
    pub audio_mixer_panel: Option<AudioMixerPanel>,
    pub lighting_panel: Option<LightingPanel>,
    pub post_process_panel: Option<PostProcessPanel>,
    pub constraints_panel: Option<ConstraintsPanel>,
    pub modifiers_panel: Option<ModifiersPanel>,
    pub spreadsheet_panel: Option<SpreadsheetPanel>,
    pub geometry_nodes_panel: Option<GeometryNodesPanel>,
    pub compositor_panel: Option<CompositorPanel>,

    /// Engine bridge for editor ↔ engine communication.
    pub engine_bridge: EngineBridge,

    /// Plugin registry: owns all registered plugins, panels, and tools.
    pub plugin_registry: PluginRegistry,

    /// MCP server: handles AI tool calls over JSON-RPC.
    pub mcp_server: McpServer,

    /// Handle to the MCP transport, allowing external code (MCP skill, IPC
    /// bridge) to push JSON-RPC requests into the editor and read responses.
    /// `None` if no transport has been attached.
    pub mcp_transport_handle: Option<MemoryTransportHandle>,

    /// MCP debug panel (manual JSON-RPC request/response inspection).
    pub mcp_debug_panel: Option<McpDebugPanel>,

    /// Optional HTTP bridge exposing the MCP transport to external AI
    /// skills. Started in `new()` on `127.0.0.1:0` (random port). Drop
    /// to shut down.
    pub mcp_http_bridge: Option<McpHttpBridge>,

    /// Path to the currently open scene file (None if unsaved).
    pub scene_path: Option<String>,

    /// Whether the scene has unsaved changes.
    pub dirty: bool,

    /// Whether the application should exit at the next opportunity.
    pub should_exit: bool,

    /// Viewport rectangle in screen coordinates: (x, y, width, height).
    pub viewport_rect: Option<(f32, f32, f32, f32)>,

    /// egui TextureId of the wgpu-rendered 3D viewport (set by main loop each frame).
    pub viewport_texture_id: Option<egui::TextureId>,

    /// Size of the viewport texture (width, height) in pixels.
    pub viewport_texture_size: (u32, u32),

    /// Whether the mouse is currently hovering over the viewport.
    pub viewport_hovered: bool,

    /// Pending action to be executed in the event loop.
    pub pending_action: Option<EditorAction>,

    /// Pending delete confirmation: node id awaiting user confirmation.
    /// Set when `confirm_deletes` is enabled and a delete was requested.
    pub pending_delete_confirmation: Option<u64>,

    /// Pending rename request: node id that should start inline editing.
    /// Set by the RenameSelected action, consumed by HierarchyPanel.
    pub pending_rename: Option<u64>,

    /// Clipboard content: the copied subtree as a flat list of nodes (root first).
    /// Set by CopySelected, consumed by Paste.
    pub clipboard: Option<Vec<crate::scene::SceneNode>>,

    /// Last applied font size (to detect changes in SettingsPanel.font_size).
    pub last_font_size: f32,

    /// Frame counter for periodic settings save (saves every ~600 frames).
    pub settings_save_counter: u32,

    /// Auto-save timer: accumulates elapsed seconds since last auto-save.
    pub auto_save_timer: f32,

    /// Frame counter for animations.
    pub frame_counter: u64,
}

impl EditorApp {
    /// Create a new EditorApp with default state.
    pub fn new() -> Self {
        // Set up the MCP transport first so the server and its external
        // handle share the same inbox/outbox.
        let (transport, mcp_handle) = MemoryTransport::new_with_handle();
        let mut mcp_server = McpServer::new();
        mcp_server.set_transport(Box::new(transport));

        // Register built-in plugins (scene-stats, mcp-status, builtin-tools).
        // Finalization is deferred to `init_plugins()` because
        // `finish_registration` needs `&mut EditorApp`.
        let mut plugin_registry = PluginRegistry::new();
        builtin::register_all(&mut plugin_registry);

        // Start the HTTP bridge on an ephemeral port. Failure is non-fatal:
        // the editor still works without remote MCP access.
        let mcp_http_bridge = match McpHttpBridge::start(mcp_handle.clone(), "127.0.0.1:0") {
            Ok(b) => {
                // Write the bound address to a well-known file so external AI
                // clients (Python scripts, etc.) can discover the port.
                let port_file = std::env::temp_dir().join("ae_editor_mcp_port.txt");
                let addr = b.bound_addr().to_string();
                let _ = std::fs::write(&port_file, &addr);
                log::info!("MCP bridge port written to {}", port_file.display());
                Some(b)
            }
            Err(e) => {
                log::warn!("Failed to start MCP HTTP bridge: {}", e);
                None
            }
        };

        Self {
            scene: Scene::new_empty(),
            camera: EditorCamera::default(),
            selection: Selection::new(),
            command_history: CommandHistory::new(100),
            gizmo: Gizmo::new(),
            gizmo_drag_start: None,
            asset_browser: AssetBrowserPanel::default(),
            hierarchy_panel: HierarchyPanel::default(),
            inspector_panel: InspectorPanel::default(),
            viewport_panel: ViewportPanel::default(),
            console_panel: Some(ConsolePanel::default()),
            stats_panel: Some(StatsPanel::default()),
            about_panel: Some(AboutPanel::default()),
            search_panel: Some(SearchPanel::default()),
            world_settings_panel: Some(WorldSettingsPanel::default()),
            layers_panel: Some(LayersPanel::default()),
            settings_panel: Some(crate::settings::load_settings().unwrap_or_default()),
            material_editor_panel: Some(MaterialEditorPanel::default()),
            terrain_editor_panel: Some(TerrainEditorPanel::default()),
            animation_timeline_panel: Some(AnimationTimelinePanel::default()),
            particle_editor_panel: Some(ParticleEditorPanel::default()),
            physics_debug_panel: Some(PhysicsDebugPanel::default()),
            bookmarks_panel: Some(BookmarksPanel::default()),
            view_modes_panel: Some(ViewModesPanel::default()),
            measure_tool_panel: Some(MeasureToolPanel::default()),
            sequencer_panel: Some(SequencerPanel::default()),
            behavior_tree_editor_panel: Some(BehaviorTreeEditorPanel::default()),
            shader_graph_panel: Some(ShaderGraphPanel::default()),
            landscape_spline_panel: Some(LandscapeSplinePanel::default()),
            foliage_editor_panel: Some(FoliageEditorPanel::default()),
            skeleton_editor_panel: Some(SkeletonEditorPanel::default()),
            anim_blueprint_panel: Some(AnimBlueprintPanel::default()),
            control_rig_panel: Some(ControlRigPanel::default()),
            variant_manager_panel: Some(VariantManagerPanel::default()),
            reference_viewer_panel: Some(ReferenceViewerPanel::default()),
            visual_logger_panel: Some(VisualLoggerPanel::default()),
            curve_editor_panel: Some(CurveEditorPanel::default()),
            uv_editor_panel: Some(UvEditorPanel::default()),
            node_editor_panel: Some(NodeEditorPanel::default()),
            dialogue_tree_panel: Some(DialogueTreePanel::default()),
            quest_editor_panel: Some(QuestEditorPanel::default()),
            navmesh_editor_panel: Some(NavmeshEditorPanel::default()),
            audio_mixer_panel: Some(AudioMixerPanel::default()),
            lighting_panel: Some(LightingPanel::default()),
            post_process_panel: Some(PostProcessPanel::default()),
            constraints_panel: Some(ConstraintsPanel::default()),
            modifiers_panel: Some(ModifiersPanel::default()),
            spreadsheet_panel: Some(SpreadsheetPanel::default()),
            geometry_nodes_panel: Some(GeometryNodesPanel::default()),
            compositor_panel: Some(CompositorPanel::default()),
            engine_bridge: EngineBridge::new(),
            plugin_registry: plugin_registry,
            mcp_server: mcp_server,
            mcp_transport_handle: Some(mcp_handle),
            mcp_debug_panel: Some(McpDebugPanel::default()),
            mcp_http_bridge: mcp_http_bridge,
            scene_path: None,
            dirty: false,
            should_exit: false,
            viewport_rect: None,
            viewport_texture_id: None,
            viewport_texture_size: (1, 1),
            viewport_hovered: false,
            pending_action: None,
            pending_delete_confirmation: None,
            pending_rename: None,
            clipboard: None,
            last_font_size: 14.0,
            settings_save_counter: 0,
            auto_save_timer: 0.0,
            frame_counter: 0,
        }
    }

    // ------------------------------------------------------------------
    // Scene lifecycle methods
    // ------------------------------------------------------------------

    /// Reset the editor to a new empty scene.
    ///
    /// If the current scene is dirty, the caller should prompt for save first
    /// (handled by `request_exit`-style flow, or called directly for Ctrl+N).
    pub fn new_scene(&mut self) {
        self.scene.reset();
        self.scene_path = None;
        self.dirty = false;
        self.selection.clear();
        self.command_history.clear();
        log::info!("New scene created");
    }

    /// Open a scene from the given file path.
    pub fn open_scene_from_path(&mut self, path: &str) {
        match crate::scene_io::load_scene(std::path::Path::new(path)) {
            Ok(scene) => {
                self.scene = scene;
                self.scene_path = Some(path.to_string());
                self.dirty = false;
                self.selection.clear();
                self.command_history.clear();
                if let Some(ref mut s) = self.settings_panel {
                    s.add_recent_file(path);
                }
                log::info!("Scene loaded from {}", path);
            },
            Err(e) => {
                log::error!("Failed to load scene from {}: {}", path, e);
            },
        }
    }

    /// Save the scene. Uses Save As if no path is set.
    pub fn save_scene(&mut self) {
        if let Some(ref path) = self.scene_path.clone() {
            match crate::scene_io::save_scene(&self.scene, std::path::Path::new(path)) {
                Ok(()) => {
                    self.dirty = false;
                    log::info!("Scene saved to {}", path);
                },
                Err(e) => {
                    log::error!("Failed to save scene: {}", e);
                },
            }
        } else {
            // No path set — caller should use save_scene_as() instead.
            log::info!("No scene path set; use Save As to choose a location");
        }
    }

    /// Save the scene to a new file path.
    pub fn save_scene_to_path(&mut self, path: &str) {
        match crate::scene_io::save_scene(&self.scene, std::path::Path::new(path)) {
            Ok(()) => {
                self.scene_path = Some(path.to_string());
                self.dirty = false;
                if let Some(ref mut s) = self.settings_panel {
                    s.add_recent_file(path);
                }
                log::info!("Scene saved to {}", path);
            },
            Err(e) => {
                log::error!("Failed to save scene: {}", e);
            },
        }
    }

    /// Check if the scene is dirty and queue exit or prompt.
    ///
    /// If dirty, sets `pending_action` to trigger a save dialog flow.
    /// If not dirty, sets `should_exit` directly.
    pub fn request_exit(&mut self) {
        // Persist settings before exiting.
        if let Some(ref settings) = self.settings_panel {
            if let Err(e) = crate::settings::save_settings(settings) {
                log::warn!("Failed to save settings on exit: {}", e);
            }
        }
        if self.dirty {
            // In a full implementation, we would show a save confirmation dialog.
            // For now, log a warning and exit anyway (the user can Ctrl+S first).
            log::warn!(
                "Scene has unsaved changes. Save before exiting (Ctrl+S) or use File > Save."
            );
            // Still exit — but mark that we warned.
            self.should_exit = true;
        } else {
            self.should_exit = true;
        }
    }

    /// Finalize plugin registration. Must be called once right after `new()`.
    ///
    /// Calls `PluginRegistry::finish_registration`, which invokes `on_register`
    /// on each plugin and collects their contributed panels and tools.
    pub fn init_plugins(&mut self) {
        let mut registry = std::mem::take(&mut self.plugin_registry);
        registry.finish_registration(self);
        self.plugin_registry = registry;
        log::info!(
            "Editor initialised: {} plugins registered",
            self.plugin_registry.plugin_count()
        );
    }

    /// Push a JSON-RPC request string into the MCP transport inbox.
    pub fn push_mcp_request(&mut self, request: &str) {
        if let Some(ref handle) = self.mcp_transport_handle {
            handle.push_message(request);
        } else {
            log::warn!("No MCP transport handle attached; dropping request");
        }
    }

    /// Pop the oldest MCP response string from the transport outbox.
    pub fn pop_mcp_response(&mut self) -> Option<String> {
        self.mcp_transport_handle.as_ref()?.pop_response()
    }

    /// Drain all pending MCP responses into a Vec (oldest first).
    pub fn drain_mcp_responses(&mut self) -> Vec<String> {
        self.mcp_transport_handle
            .as_ref()
            .map(|h| h.drain_responses())
            .unwrap_or_default()
    }

    // ------------------------------------------------------------------
    // Edit operations with undo support
    // ------------------------------------------------------------------

    /// Add a child node with undo record.
    pub fn add_child_with_undo(
        &mut self,
        parent_id: u64,
        name: &str,
        node_type: crate::scene::NodeType,
    ) -> Option<u64> {
        let new_id = self.scene.add_child(parent_id, name);
        if let Some(id) = new_id {
            if let Some(node) = self.scene.find_node_mut(id) {
                node.node_type = node_type;
            }
            let create_cmd = crate::commands::CreateNodeCommand::new(parent_id, id);
            let _ = self.command_history.execute(Box::new(create_cmd), &mut self.scene);
            self.dirty = true;
            let auto_select = self
                .settings_panel
                .as_ref()
                .map(|s| s.auto_select_new_nodes)
                .unwrap_or(true);
            if auto_select {
                self.selection.select(id);
            }
        }
        new_id
    }

    /// Duplicate a node (and its subtree) with undo record.
    /// The clone is added as a sibling of the source under the same parent.
    pub fn duplicate_node_with_undo(&mut self, source_id: u64) -> Option<u64> {
        let parent_id = self.scene.find_node(source_id).and_then(|n| n.parent);
        let new_id = self.scene.duplicate_subtree(source_id);
        if let Some(id) = new_id {
            let dup_cmd = crate::commands::DuplicateNodeCommand::new(id, parent_id);
            let _ = self.command_history.execute(Box::new(dup_cmd), &mut self.scene);
            self.dirty = true;
            let auto_select = self
                .settings_panel
                .as_ref()
                .map(|s| s.auto_select_new_nodes)
                .unwrap_or(true);
            if auto_select {
                self.selection.select(id);
            }
        }
        new_id
    }

    /// Commit a finished gizmo drag as a single SetTransformCommand for undo.
    /// Called after `gizmo.end_drag()` when the user releases the mouse.
    pub fn commit_gizmo_drag(&mut self) {
        if let Some((node_id, old_t)) = self.gizmo_drag_start.take() {
            if let Some(node) = self.scene.find_node(node_id) {
                let new_t = node.transform.clone();
                if new_t != old_t {
                    let cmd = crate::commands::SetTransformCommand::new(node_id, old_t, new_t);
                    let _ = self.command_history.execute(Box::new(cmd), &mut self.scene);
                }
            }
        }
    }

    /// Copy the selected node (and its subtree) to the internal clipboard.
    /// Does not touch the system clipboard — this is an editor-internal copy.
    pub fn copy_selected(&mut self) {
        if let Some(id) = self.selection.selected_id {
            if id == 0 {
                return;
            }
            let subtree = self.scene.collect_subtree_nodes(id);
            if !subtree.is_empty() {
                log::info!("Copied node {} ({} nodes)", id, subtree.len());
                self.clipboard = Some(subtree);
            }
        }
    }

    /// Paste the clipboard content as a new subtree.
    /// The pasted node is added under the currently selected node (or root if
    /// nothing selected). Wrapped in a DuplicateNodeCommand for undo/redo.
    pub fn paste_from_clipboard(&mut self) {
        let clipboard_nodes = match &self.clipboard {
            Some(nodes) if !nodes.is_empty() => nodes.clone(),
            _ => return,
        };

        // Paste under the selected node if it exists, otherwise under root (0).
        let parent_id = self.selection.selected_id.unwrap_or(0);
        if self.scene.find_node(parent_id).is_none() {
            return;
        }

        let new_root = self.scene.paste_subtree(&clipboard_nodes, parent_id);
        if let Some(id) = new_root {
            let paste_cmd = crate::commands::DuplicateNodeCommand::new(id, Some(parent_id));
            let _ = self.command_history.execute(Box::new(paste_cmd), &mut self.scene);
            self.dirty = true;
            let auto_select = self
                .settings_panel
                .as_ref()
                .map(|s| s.auto_select_new_nodes)
                .unwrap_or(true);
            if auto_select {
                self.selection.select(id);
            }
            log::info!("Pasted node {} ({} nodes)", id, clipboard_nodes.len());
        }
    }

    // ------------------------------------------------------------------
    // Edit operations
    // ------------------------------------------------------------------

    /// Delete the currently selected node.
    pub fn delete_selected(&mut self) {
        if let Some(id) = self.selection.selected_id {
            // Don't allow deleting the root node.
            if id == 0 {
                log::warn!("Cannot delete the root node.");
                return;
            }

            // Snapshot the node and parent before deleting for undo.
            let parent_id = self.scene.find_node(id).and_then(|n| n.parent);
            let stored = self.scene.find_node(id).cloned();

            if let Some(node) = stored {
                let delete_cmd = crate::commands::DeleteNodeCommand::new(id, parent_id, node);
                let _ = self.command_history.execute(Box::new(delete_cmd), &mut self.scene);
            }

            // Also remove from selection.
            self.selection.clear();
            self.dirty = true;
            log::info!("Deleted node {}", id);
        }
    }

    /// Focus the camera on the currently selected node.
    pub fn focus_on_selection(&mut self) {
        if let Some(id) = self.selection.selected_id {
            if let Some(node) = self.scene.find_node(id) {
                self.camera.focus_on(node.transform.translation);
            }
        }
    }

    // ------------------------------------------------------------------
    // Action execution (called from the event loop)
    // ------------------------------------------------------------------

    /// Execute a pending editor action.
    ///
    /// This should be called from the winit event loop (e.g., in
    /// `MainEventsCleared`) so that `rfd` dialogs have access to the
    /// event loop.
    pub fn execute_pending_action(&mut self) {
        let action = match self.pending_action.take() {
            Some(a) => a,
            None => return,
        };

        match action {
            EditorAction::NewScene => {
                if self.dirty {
                    log::warn!("Discarding unsaved changes for new scene.");
                }
                self.new_scene();
            },
            EditorAction::OpenScene => {
                self.execute_open_dialog();
            },
            EditorAction::OpenSceneFromPath(path) => {
                self.open_scene_from_path(&path);
            },
            EditorAction::SaveScene => {
                if self.scene_path.is_some() {
                    self.save_scene();
                } else {
                    self.execute_save_as_dialog();
                }
            },
            EditorAction::SaveSceneAs => {
                self.execute_save_as_dialog();
            },
            EditorAction::Undo => {
                if let Err(e) = self.command_history.undo(&mut self.scene) {
                    log::error!("Undo failed: {}", e);
                } else {
                    self.dirty = true;
                }
            },
            EditorAction::Redo => {
                if let Err(e) = self.command_history.redo(&mut self.scene) {
                    log::error!("Redo failed: {}", e);
                } else {
                    self.dirty = true;
                }
            },
            EditorAction::DeleteSelected => {
                let confirm = self
                    .settings_panel
                    .as_ref()
                    .map(|s| s.confirm_deletes)
                    .unwrap_or(true);
                if confirm && self.selection.selected_id.is_some() {
                    self.pending_delete_confirmation = self.selection.selected_id;
                } else {
                    self.delete_selected();
                }
            },
            EditorAction::DuplicateSelected => {
                if let Some(id) = self.selection.selected_id {
                    if id != 0 {
                        self.duplicate_node_with_undo(id);
                    }
                }
            },
            EditorAction::RenameSelected => {
                if let Some(id) = self.selection.selected_id {
                    if id != 0 {
                        self.pending_rename = Some(id);
                    }
                }
            },
            EditorAction::CopySelected => {
                self.copy_selected();
            },
            EditorAction::Paste => {
                self.paste_from_clipboard();
            },
            EditorAction::FocusSelection => {
                self.focus_on_selection();
            },
            EditorAction::Deselect => {
                self.selection.clear();
            },
            EditorAction::Exit => {
                self.request_exit();
            },
            EditorAction::GizmoTranslate => {
                self.gizmo.mode = GizmoMode::Translate;
            },
            EditorAction::GizmoRotate => {
                self.gizmo.mode = GizmoMode::Rotate;
            },
            EditorAction::GizmoScale => {
                self.gizmo.mode = GizmoMode::Scale;
            },
        }
    }

    /// Open a file dialog to choose a scene file to open.
    fn execute_open_dialog(&mut self) {
        let file = rfd::FileDialog::new()
            .add_filter("Wasteland Scene", &["ae", "json"])
            .add_filter("All Files", &["*"])
            .pick_file();

        if let Some(path) = file {
            let path_str = path.to_string_lossy().to_string();
            self.open_scene_from_path(&path_str);
        }
    }

    /// Open a file dialog to choose where to save the scene.
    fn execute_save_as_dialog(&mut self) {
        let file = rfd::FileDialog::new()
            .add_filter("Wasteland Scene", &["ae", "json"])
            .add_filter("All Files", &["*"])
            .set_file_name(&self.scene.name)
            .save_file();

        if let Some(path) = file {
            let path_str = path.to_string_lossy().to_string();
            self.save_scene_to_path(&path_str);
        }
    }

    // ------------------------------------------------------------------
    // Rendering
    // ------------------------------------------------------------------

    /// Render all editor UI panels for the current frame.
    ///
    /// Call this once per frame from the main loop, after beginning the
    /// egui frame.
    pub fn render(&mut self, ctx: &egui::Context) {
        self.frame_counter = self.frame_counter.wrapping_add(1);
        self.engine_bridge.tick(self.frame_counter);

        // Drive plugins: per-frame update before UI renders.
        // Use mem::take to avoid double-mutable-borrow of self (registry + app).
        let mut registry = std::mem::take(&mut self.plugin_registry);
        registry.update(self, ctx);
        self.plugin_registry = registry;

        // Pump MCP server: dispatch incoming AI requests against the live scene.
        // Split-borrow distinct fields so the borrow checker is satisfied.
        let scene = &mut self.scene;
        let selection = &mut self.selection;
        let scene_path = &mut self.scene_path;
        let dirty = &mut self.dirty;
        let mcp_server = &mut self.mcp_server;
        mcp_server.poll(ToolContext { scene, selection, scene_path, dirty });

        crate::panels::render_all_panels(ctx, self);

        // Modal confirmation dialog for node deletion.
        self.render_delete_confirmation(ctx);

        // Sync panel state to actual editor components (camera, egui ctx, viewport).
        self.sync_panel_settings(ctx);

        // Periodically persist settings to disk (every ~600 frames ≈ 10s @ 60fps).
        self.settings_save_counter = self.settings_save_counter.wrapping_add(1);
        if self.settings_save_counter % 600 == 0 {
            if let Some(ref settings) = self.settings_panel {
                if let Err(e) = crate::settings::save_settings(settings) {
                    log::warn!("Failed to save settings: {}", e);
                }
            }
        }

        // Auto-save the scene if enabled, dirty, and has a path.
        let dt = ctx.input(|i| i.stable_dt);
        if dt > 0.0 {
            let (auto_enabled, auto_interval) = self
                .settings_panel
                .as_ref()
                .map(|s| (s.auto_save_enabled, s.auto_save_interval))
                .unwrap_or((false, 300.0));
            if auto_enabled {
                self.auto_save_timer += dt;
                if self.auto_save_timer >= auto_interval && self.dirty && self.scene_path.is_some() {
                    self.auto_save_timer = 0.0;
                    log::info!("Auto-saving scene...");
                    self.save_scene();
                }
            }
        }
    }

    /// Render the delete confirmation modal dialog.
    ///
    /// Shown when `pending_delete_confirmation` is set (i.e., user triggered
    /// delete while `confirm_deletes` is enabled).
    fn render_delete_confirmation(&mut self, ctx: &egui::Context) {
        let node_id = match self.pending_delete_confirmation {
            Some(id) => id,
            None => return,
        };

        let node_name = self
            .scene
            .find_node(node_id)
            .map(|n| n.name.clone())
            .unwrap_or_else(|| "(unknown)".to_string());

        let mut confirmed = false;
        let mut cancelled = false;

        egui::Window::new("Confirm Delete")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.add_space(4.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new("⚠ Delete this node?")
                            .strong()
                            .color(egui::Color32::from_rgb(255, 180, 80)),
                    );
                    ui.add_space(6.0);
                    ui.label(format!("Name: {}", node_name));
                    ui.label(format!("ID:   {}", node_id));
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button(egui::RichText::new("Delete").color(egui::Color32::from_rgb(255, 100, 100))).clicked() {
                            confirmed = true;
                        }
                        ui.add_space(12.0);
                        if ui.button("Cancel").clicked() {
                            cancelled = true;
                        }
                    });
                });
            });

        if confirmed {
            self.pending_delete_confirmation = None;
            self.delete_selected();
        } else if cancelled {
            self.pending_delete_confirmation = None;
        } else if self.scene.find_node(node_id).is_none() {
            // Node was removed by other means; dismiss the dialog.
            self.pending_delete_confirmation = None;
        }
    }

    /// Sync panel settings to actual editor components.
    ///
    /// SettingsPanel → EditorCamera (move_speed, sensitivity), egui ctx (theme, ui_scale).
    /// ViewModesPanel ↔ ViewportPanel (grid, stats, labels), EditorCamera (fov, near, far).
    /// Bidirectional sync for ViewModesPanel: when open it controls viewport; when closed,
    /// viewport toolbar values propagate back.
    fn sync_panel_settings(&mut self, ctx: &egui::Context) {
        // SettingsPanel → camera + egui ctx + hierarchy
        if let Some(ref settings) = self.settings_panel {
            self.camera.move_speed = settings.camera_speed * 5.0;
            self.camera.sensitivity = 0.005 * settings.camera_sensitivity;
            match settings.theme_mode {
                0 => ctx.set_visuals(egui::Visuals::dark()),
                1 => ctx.set_visuals(egui::Visuals::light()),
                _ => {}
            }
            let scale = settings.ui_scale.max(0.1);
            if (ctx.pixels_per_point() - scale).abs() > 0.001 {
                ctx.set_pixels_per_point(scale);
            }
            // Font size: scale all text styles when the user changes it.
            // Default egui body size is 14.0; we scale all named sizes proportionally.
            let desired_font = settings.font_size.max(8.0);
            if (self.last_font_size - desired_font).abs() > 0.01 {
                let factor = desired_font / 14.0;
                let mut style = (*ctx.style()).clone();
                for font_id in style.text_styles.values_mut() {
                    font_id.size = (font_id.size * factor).max(6.0);
                }
                ctx.set_style(style);
                self.last_font_size = desired_font;
            }
            self.hierarchy_panel.show_ids = settings.show_node_ids;
            self.command_history.set_max_undo(settings.undo_history_limit as usize);
        }

        // ViewModesPanel ↔ viewport_panel + camera
        if let Some(ref mut vm) = self.view_modes_panel {
            if vm.visible {
                self.viewport_panel.show_grid = vm.show_grid;
                self.viewport_panel.show_stats_overlay = vm.show_stats;
                self.viewport_panel.show_labels = vm.show_names;
                self.camera.fov = vm.fov;
                self.camera.near = vm.near_plane;
                self.camera.far = vm.far_plane;
            } else {
                vm.show_grid = self.viewport_panel.show_grid;
                vm.show_stats = self.viewport_panel.show_stats_overlay;
                vm.show_names = self.viewport_panel.show_labels;
                vm.fov = self.camera.fov;
                vm.near_plane = self.camera.near;
                vm.far_plane = self.camera.far;
            }
        }
    }
}

impl Default for EditorApp {
    fn default() -> Self {
        Self::new()
    }
}
