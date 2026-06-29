use glam::Vec3;
use wasteland_asset_pipeline::{AssetFormat, AssetPipeline, BlueprintImporter};
use wasteland_engine::{BodyType, FixedPoint, FixedQuat, FixedVec3, MaterialProperties, RigidBody};
use wasteland_metaentity::meta_entity::{EntityChanges, MetaEntity};
use wasteland_physics::world::PhysicsWorld;
use wasteland_registry::ResponseRegistry;
use wasteland_save_system::{SaveFormat, SaveMetadata, SaveSystem};
use wasteland_scheduler::{EventType, Scheduler};
use wasteland_unified_interface::{UnifiedWorld, WorldStorage};
use wasteland_engine::EngineUuid;

fn make_dynamic_rigid_body(id: EngineUuid, mass: f32, pos: Vec3, vel: Vec3) -> RigidBody {
    RigidBody {
        id,
        position: FixedVec3::from_f32(pos.x, pos.y, pos.z),
        rotation: FixedQuat::IDENTITY,
        velocity: FixedVec3::from_f32(vel.x, vel.y, vel.z),
        angular_velocity: FixedVec3::ZERO,
        mass: FixedPoint::from_f32(mass),
        material: MaterialProperties::default(),
        body_type: BodyType::Dynamic,
        is_sleeping: false,
        sleep_timer: FixedPoint::ZERO,
        forces: FixedVec3::ZERO,
        torque: FixedVec3::ZERO,
        linear_damping: FixedPoint::ZERO,
        angular_damping: FixedPoint::ZERO,
        mpss_index: None,
    }
}

#[test]
fn test_scheduler_with_registry() {
    let mut world = WorldStorage::with_capacity(100);
    let mut scheduler = Scheduler::new(4);
    let mut registry = ResponseRegistry::new();

    let iron = MetaEntity::iron(Vec3::ZERO, 0);
    let water = MetaEntity::water(Vec3::new(1.0, 0.0, 0.0), 0);
    let id1 = iron.id;
    let id2 = water.id;
    world.spawn_entity(iron);
    world.spawn_entity(water);

    registry.register_custom("iron_water_rust", |a, _b, _w| {
        let changes = vec![EntityChanges {
            entity_id: a.id,
            expected_version: a.version,
            position: None,
            velocity: None,
            angular_velocity: None,
            physics_changes: [("hardness".to_string(), -5.0)].into(),
            chemistry_changes: hashbrown::HashMap::new(),
            biology_changes: hashbrown::HashMap::new(),
            state_change: None,
            extension_changes: hashbrown::HashMap::new(),
            spawn_children: vec![],
            despawn: false,
        }];
        changes
    });

    scheduler.submit_event(EventType::Collision, vec![id1], vec![1, 2], 0, 0);
    scheduler.submit_event(EventType::Reaction, vec![id2], vec![3], 0, 1);

    let completed = scheduler.execute_frame(&mut world);
    assert!(!completed.is_empty());
}

#[test]
fn test_asset_import_to_entity() {
    let mut pipeline = AssetPipeline::new(16);
    pipeline.register_importer(Box::new(BlueprintImporter::new()));

    let blueprint = r#"{"name":"test_bp","version":1,"entities":[{"position":[1.0,2.0,3.0],"function_tags":["structural"],"extensions":{}}],"dependencies":[],"metadata":{}}"#;
    let entities = pipeline.import("test_bp", AssetFormat::BLUEPRINT, blueprint.as_bytes(), 0);
    assert!(entities.is_ok(), "Blueprint import failed: {:?}", entities.err());

    let entities = entities.unwrap();
    assert!(!entities.is_empty());
}

#[test]
fn test_save_load_roundtrip() {
    let save_dir = std::env::temp_dir().join("wasteland_integration_test");
    std::fs::create_dir_all(&save_dir).unwrap();
    let mut save_system = SaveSystem::new(save_dir.clone());
    let mut world = WorldStorage::with_capacity(100);

    let iron = MetaEntity::iron(Vec3::ZERO, 0);
    let water = MetaEntity::water(Vec3::new(2.0, 0.0, 0.0), 0);
    world.spawn_entity(iron);
    world.spawn_entity(water);

    let metadata = SaveMetadata {
        play_time: 0.0,
        tick_count: 0,
        player_position: Vec3::ZERO,
        seed: 0,
        tags: vec![],
    };
    let save_path = save_dir.join("integration_test.sav");

    let result = save_system.save(&world, metadata, &save_path, SaveFormat::Binary, false);
    assert!(result.is_ok(), "Save failed: {:?}", result.err());

    let loaded = save_system.load(&save_path);
    assert!(loaded.is_ok(), "Load failed: {:?}", loaded.err());

    let (_header, _metadata, entities) = loaded.unwrap();
    assert_eq!(entities.len(), 2);

    let _ = std::fs::remove_dir_all(&save_dir);
}

#[test]
fn test_deterministic_physics_fixed_point() {
    let mut world_a = PhysicsWorld::default();
    let mut world_b = PhysicsWorld::default();

    let id0 = EngineUuid::from_u64_pair(0, 0);
    let id1 = EngineUuid::from_u64_pair(1, 0);
    let id2 = EngineUuid::from_u64_pair(2, 0);

    for &id in &[id0, id1, id2] {
        let pos = Vec3::new(id.as_u64_pair().0 as f32 * 3.0, 10.0, 0.0);
        let body = make_dynamic_rigid_body(id, 1.0, pos, Vec3::new(0.0, -9.8, 0.0));
        world_a.add_rigid_body(body.clone());
        world_b.add_rigid_body(body);
    }

    for _ in 0..60 {
        world_a.step();
        world_b.step();
    }

    for &id in &[id0, id1, id2] {
        let body_a = world_a.rigid_bodies.iter().find(|b| b.id == id);
        let body_b = world_b.rigid_bodies.iter().find(|b| b.id == id);

        if let (Some(a), Some(b)) = (body_a, body_b) {
            let diff_x = (a.position.to_glam().x - b.position.to_glam().x).abs();
            let diff_y = (a.position.to_glam().y - b.position.to_glam().y).abs();
            let diff_z = (a.position.to_glam().z - b.position.to_glam().z).abs();
            assert!(diff_x < 0.001, "Determinism failed x: {}", diff_x);
            assert!(diff_y < 0.001, "Determinism failed y: {}", diff_y);
            assert!(diff_z < 0.001, "Determinism failed z: {}", diff_z);
        }
    }
}
