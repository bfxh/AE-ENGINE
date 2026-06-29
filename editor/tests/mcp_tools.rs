//! Integration tests for the 15 MCP tools.
//!
//! Each test exercises one tool against a fresh Scene + Selection,
//! covering normal, boundary, and error scenarios.

use slime_editor::mcp::tools::execute_tool;
use slime_editor::scene::{LightType, NodeType, Scene};
use slime_editor::selection::Selection;
use serde_json::json;

fn fresh_state() -> (Scene, Selection, Option<String>, bool) {
    (Scene::new_empty(), Selection::new(), None, false)
}

fn run(name: &str, args: serde_json::Value, scene: &mut Scene, sel: &mut Selection, path: &mut Option<String>, dirty: &mut bool) -> slime_editor::mcp::tools::McpToolResult {
    execute_tool(name, &args, scene, sel, path, dirty)
}

// ============================================================
// get_scene_tree
// ============================================================

#[test]
fn test_get_scene_tree_empty_scene() {
    let (mut scene, mut sel, mut path, mut dirty) = fresh_state();
    let result = run("get_scene_tree", json!({}), &mut scene, &mut sel, &mut path, &mut dirty);
    assert!(result.success, "get_scene_tree should succeed");
    assert!(result.data.is_object(), "result should be a JSON object");
}

#[test]
fn test_get_scene_tree_with_nodes() {
    let (mut scene, mut sel, mut path, mut dirty) = fresh_state();
    run("create_node", json!({"name":"Child","node_type":"empty","parent_id":0}), &mut scene, &mut sel, &mut path, &mut dirty);
    let result = run("get_scene_tree", json!({}), &mut scene, &mut sel, &mut path, &mut dirty);
    assert!(result.success);
}

// ============================================================
// create_node
// ============================================================

#[test]
fn test_create_node_empty_type() {
    let (mut scene, mut sel, mut path, mut dirty) = fresh_state();
    let result = run("create_node",
        json!({"name":"TestNode","node_type":"empty","parent_id":0}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    assert!(result.success, "create_node should succeed");
    assert_eq!(result.data["name"], "TestNode");
    assert!(dirty, "dirty flag should be set");
    assert_eq!(scene.nodes.len(), 2, "should have root + 1 child");
    let node = scene.find_node(1).unwrap();
    assert_eq!(node.name, "TestNode");
    assert!(matches!(node.node_type, NodeType::Empty));
}

#[test]
fn test_create_node_mesh_type() {
    let (mut scene, mut sel, mut path, mut dirty) = fresh_state();
    let result = run("create_node",
        json!({"name":"Cube","node_type":"mesh","parent_id":0,"path":"cube.glb","position":[1.0,2.0,3.0]}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    assert!(result.success);
    let node = scene.find_node(1).unwrap();
    assert!(matches!(node.node_type, NodeType::Mesh { .. }));
    if let NodeType::Mesh { path } = &node.node_type {
        assert_eq!(path, "cube.glb");
    }
    assert_eq!(node.transform.translation, glam::Vec3::new(1.0, 2.0, 3.0));
}

#[test]
fn test_create_node_light_type() {
    let (mut scene, mut sel, mut path, mut dirty) = fresh_state();
    let result = run("create_node",
        json!({"name":"Sun","node_type":"light","parent_id":0,"light_type":"directional","intensity":2.5,"color":[1.0,0.9,0.8]}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    assert!(result.success);
    let node = scene.find_node(1).unwrap();
    if let NodeType::Light { light_type, intensity, color } = &node.node_type {
        assert!(matches!(light_type, LightType::Directional));
        assert!((intensity - 2.5).abs() < 1e-6);
        assert_eq!(*color, glam::Vec3::new(1.0, 0.9, 0.8));
    } else {
        panic!("expected Light node");
    }
}

#[test]
fn test_create_node_camera_type() {
    let (mut scene, mut sel, mut path, mut dirty) = fresh_state();
    let result = run("create_node",
        json!({"name":"Cam","node_type":"camera","parent_id":0,"fov":75.0,"near":0.5,"far":500.0}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    assert!(result.success);
    let node = scene.find_node(1).unwrap();
    if let NodeType::Camera { fov, near, far } = &node.node_type {
        assert!((fov - 75.0).abs() < 1e-6);
        assert!((near - 0.5).abs() < 1e-6);
        assert!((far - 500.0).abs() < 1e-6);
    } else {
        panic!("expected Camera node");
    }
}

#[test]
fn test_create_node_invalid_parent() {
    let (mut scene, mut sel, mut path, mut dirty) = fresh_state();
    let result = run("create_node",
        json!({"name":"Orphan","node_type":"empty","parent_id":9999}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    assert!(!result.success, "should fail with invalid parent");
    assert!(result.error.is_some());
}

#[test]
fn test_create_node_defaults() {
    let (mut scene, mut sel, mut path, mut dirty) = fresh_state();
    let result = run("create_node",
        json!({"name":"Default","node_type":"empty"}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    assert!(result.success);
    assert_eq!(result.data["name"], "Default");
}

// ============================================================
// delete_node
// ============================================================

#[test]
fn test_delete_node_success() {
    let (mut scene, mut sel, mut path, mut dirty) = fresh_state();
    run("create_node", json!({"name":"ToDelete","node_type":"empty","parent_id":0}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    let result = run("delete_node", json!({"node_id":1}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    assert!(result.success);
    assert!(scene.find_node(1).is_none(), "node should be deleted");
}

#[test]
fn test_delete_node_root_blocked() {
    let (mut scene, mut sel, mut path, mut dirty) = fresh_state();
    let result = run("delete_node", json!({"node_id":0}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    assert!(!result.success, "should not delete root");
}

#[test]
fn test_delete_node_missing_id_param() {
    let (mut scene, mut sel, mut path, mut dirty) = fresh_state();
    let result = run("delete_node", json!({}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    assert!(!result.success);
}

#[test]
fn test_delete_node_clears_selection() {
    let (mut scene, mut sel, mut path, mut dirty) = fresh_state();
    run("create_node", json!({"name":"ToDel","node_type":"empty","parent_id":0}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    sel.select(1);
    run("delete_node", json!({"node_id":1}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    assert!(sel.selected_id.is_none(), "selection should be cleared");
}

// ============================================================
// set_node_property
// ============================================================

#[test]
fn test_set_node_property_name() {
    let (mut scene, mut sel, mut path, mut dirty) = fresh_state();
    run("create_node", json!({"name":"Old","node_type":"empty","parent_id":0}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    let result = run("set_node_property",
        json!({"node_id":1,"property":"name","value":"New"}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    assert!(result.success);
    assert_eq!(scene.find_node(1).unwrap().name, "New");
}

#[test]
fn test_set_node_property_translation() {
    let (mut scene, mut sel, mut path, mut dirty) = fresh_state();
    run("create_node", json!({"name":"Mover","node_type":"empty","parent_id":0}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    let result = run("set_node_property",
        json!({"node_id":1,"property":"translation","value":[5.0,10.0,15.0]}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    assert!(result.success);
    assert_eq!(scene.find_node(1).unwrap().transform.translation, glam::Vec3::new(5.0, 10.0, 15.0));
}

#[test]
fn test_set_node_property_unknown_property() {
    let (mut scene, mut sel, mut path, mut dirty) = fresh_state();
    run("create_node", json!({"name":"X","node_type":"empty","parent_id":0}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    let result = run("set_node_property",
        json!({"node_id":1,"property":"nonexistent","value":42}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    assert!(!result.success);
}

#[test]
fn test_set_node_property_missing_node() {
    let (mut scene, mut sel, mut path, mut dirty) = fresh_state();
    let result = run("set_node_property",
        json!({"node_id":9999,"property":"name","value":"Ghost"}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    assert!(!result.success);
}

// ============================================================
// get_node_properties
// ============================================================

#[test]
fn test_get_node_properties_success() {
    let (mut scene, mut sel, mut path, mut dirty) = fresh_state();
    run("create_node", json!({"name":"Probe","node_type":"empty","parent_id":0,"position":[1.0,2.0,3.0]}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    let result = run("get_node_properties", json!({"node_id":1}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    assert!(result.success);
    assert_eq!(result.data["name"], "Probe");
    assert_eq!(result.data["translation"], json!([1.0, 2.0, 3.0]));
}

#[test]
fn test_get_node_properties_not_found() {
    let (mut scene, mut sel, mut path, mut dirty) = fresh_state();
    let result = run("get_node_properties", json!({"node_id":9999}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    assert!(!result.success);
}

// ============================================================
// transform_node
// ============================================================

#[test]
fn test_transform_node_position() {
    let (mut scene, mut sel, mut path, mut dirty) = fresh_state();
    run("create_node", json!({"name":"T","node_type":"empty","parent_id":0}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    let result = run("transform_node",
        json!({"node_id":1,"position":[7.0,8.0,9.0],"rotation":[0.0,0.0,0.0,1.0],"scale":[2.0,2.0,2.0]}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    assert!(result.success);
    let node = scene.find_node(1).unwrap();
    assert_eq!(node.transform.translation, glam::Vec3::new(7.0, 8.0, 9.0));
    assert_eq!(node.transform.scale, glam::Vec3::new(2.0, 2.0, 2.0));
}

#[test]
fn test_transform_node_not_found() {
    let (mut scene, mut sel, mut path, mut dirty) = fresh_state();
    let result = run("transform_node",
        json!({"node_id":9999,"position":[1.0,1.0,1.0]}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    assert!(!result.success);
}

// ============================================================
// select_node + get_selection
// ============================================================

#[test]
fn test_select_and_get_selection() {
    let (mut scene, mut sel, mut path, mut dirty) = fresh_state();
    run("create_node", json!({"name":"Pick","node_type":"empty","parent_id":0}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    let r1 = run("select_node", json!({"node_id":1}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    assert!(r1.success);
    let r2 = run("get_selection", json!({}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    assert!(r2.success);
    assert_eq!(r2.data["selected_id"], 1);
}

#[test]
fn test_select_node_not_found() {
    let (mut scene, mut sel, mut path, mut dirty) = fresh_state();
    let result = run("select_node", json!({"node_id":9999}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    assert!(!result.success);
}

// ============================================================
// new_scene
// ============================================================

#[test]
fn test_new_scene_resets() {
    let (mut scene, mut sel, mut path, mut dirty) = fresh_state();
    run("create_node", json!({"name":"X","node_type":"empty","parent_id":0}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    path = Some("/tmp/test.wasteland".into());
    let result = run("new_scene", json!({}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    assert!(result.success);
    assert_eq!(scene.nodes.len(), 1, "should have only root");
    assert!(path.is_none(), "path should be cleared");
    assert!(!dirty, "dirty should be false");
}

// ============================================================
// validate_scene
// ============================================================

#[test]
fn test_validate_scene_fresh_is_valid() {
    let (mut scene, mut sel, mut path, mut dirty) = fresh_state();
    let result = run("validate_scene", json!({}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    assert!(result.success);
    assert_eq!(result.data["valid"], true);
}

#[test]
fn test_validate_scene_detects_issues() {
    let (mut scene, mut sel, mut path, mut dirty) = fresh_state();
    scene.nodes.clear();
    let result = run("validate_scene", json!({}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    assert!(result.success);
    assert_eq!(result.data["valid"], false);
    assert!(result.data["issues"].as_array().unwrap().len() > 0);
}

// ============================================================
// get_editor_state
// ============================================================

#[test]
fn test_get_editor_state() {
    let (mut scene, mut sel, mut path, mut dirty) = fresh_state();
    run("create_node", json!({"name":"S","node_type":"empty","parent_id":0}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    let result = run("get_editor_state", json!({}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    assert!(result.success);
    assert_eq!(result.data["node_count"], 2);
    assert_eq!(result.data["dirty"], true);
}

// ============================================================
// set_camera_view
// ============================================================

#[test]
fn test_set_camera_view_acknowledged() {
    let (mut scene, mut sel, mut path, mut dirty) = fresh_state();
    let result = run("set_camera_view",
        json!({"position":[0.0,5.0,10.0],"target":[0.0,0.0,0.0]}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    assert!(result.success);
    assert_eq!(result.data["acknowledged"], true);
}

// ============================================================
// batch_execute
// ============================================================

#[test]
fn test_batch_execute_multiple_creates() {
    let (mut scene, mut sel, mut path, mut dirty) = fresh_state();
    let result = run("batch_execute",
        json!({"commands":[
            {"tool":"create_node","args":{"name":"A","node_type":"empty","parent_id":0}},
            {"tool":"create_node","args":{"name":"B","node_type":"empty","parent_id":0}},
            {"tool":"create_node","args":{"name":"C","node_type":"empty","parent_id":0}}
        ]}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    assert!(result.success);
    assert_eq!(scene.nodes.len(), 4, "root + 3 children");
    let results = result.data["results"].as_array().unwrap();
    assert_eq!(results.len(), 3);
    for r in results {
        assert_eq!(r["success"], true);
    }
}

#[test]
fn test_batch_execute_stops_on_error() {
    let (mut scene, mut sel, mut path, mut dirty) = fresh_state();
    let result = run("batch_execute",
        json!({"commands":[
            {"tool":"create_node","args":{"name":"OK","node_type":"empty","parent_id":0}},
            {"tool":"delete_node","args":{"node_id":0}},
            {"tool":"create_node","args":{"name":"WontRun","node_type":"empty","parent_id":0}}
        ]}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    assert!(result.success, "batch itself succeeds");
    let results = result.data["results"].as_array().unwrap();
    assert_eq!(results.len(), 2, "should stop after error, 2 results (1 ok + 1 err)");
    assert_eq!(results[1]["success"], false);
}

#[test]
fn test_batch_execute_missing_commands_param() {
    let (mut scene, mut sel, mut path, mut dirty) = fresh_state();
    let result = run("batch_execute", json!({}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    assert!(!result.success);
}

// ============================================================
// unknown tool
// ============================================================

#[test]
fn test_unknown_tool_returns_error() {
    let (mut scene, mut sel, mut path, mut dirty) = fresh_state();
    let result = run("nonexistent_tool", json!({}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    assert!(!result.success);
    assert!(result.error.unwrap().contains("Unknown tool"));
}

// ============================================================
// save_scene / load_scene (file I/O)
// ============================================================

#[test]
fn test_save_and_load_scene_roundtrip() {
    let (mut scene, mut sel, mut path, mut dirty) = fresh_state();
    run("create_node", json!({"name":"Persist","node_type":"mesh","parent_id":0,"path":"cube.glb","position":[3.0,4.0,5.0]}),
        &mut scene, &mut sel, &mut path, &mut dirty);

    let tmp = std::env::temp_dir().join("slime_editor_test_scene.wasteland");
    let tmp_str = tmp.to_string_lossy().to_string();

    let save_result = run("save_scene", json!({"path": tmp_str}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    assert!(save_result.success, "save should succeed");
    assert!(!dirty, "dirty should be cleared after save");

    let (mut scene2, mut sel2, mut path2, mut dirty2) = fresh_state();
    let load_result = run("load_scene", json!({"path": tmp_str}),
        &mut scene2, &mut sel2, &mut path2, &mut dirty2);
    assert!(load_result.success, "load should succeed");
    assert_eq!(scene2.nodes.len(), 2, "should have root + 1 child");
    let node = scene2.find_node(1).unwrap();
    assert_eq!(node.name, "Persist");
    assert_eq!(node.transform.translation, glam::Vec3::new(3.0, 4.0, 5.0));

    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn test_load_scene_missing_file() {
    let (mut scene, mut sel, mut path, mut dirty) = fresh_state();
    let result = run("load_scene", json!({"path": "/nonexistent/path/scene.wasteland"}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    assert!(!result.success);
}

#[test]
fn test_save_scene_missing_path() {
    let (mut scene, mut sel, mut path, mut dirty) = fresh_state();
    let result = run("save_scene", json!({}),
        &mut scene, &mut sel, &mut path, &mut dirty);
    assert!(!result.success);
}
