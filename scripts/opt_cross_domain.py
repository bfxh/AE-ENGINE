p = r'D:\rj\wasteland_project\wasteland_engine\src\lib.rs'
with open(p, 'r', encoding='utf-8') as f:
    src = f.read()

changes = []

# 1. Add collision_grid field
old = '    pub spatial_hash: crate::architecture::spatial_hash::SpatialHashGrid,\n\n    pub perf_stats: PerfStats,\n}'
new = '    pub spatial_hash: crate::architecture::spatial_hash::SpatialHashGrid,\n    pub collision_grid: crate::architecture::spatial_hash::SpatialHashGrid,\n\n    pub perf_stats: PerfStats,\n}'
if old in src:
    src = src.replace(old, new)
    changes.append('1. collision_grid field added')
else:
    print('ERROR: struct field not found')

# 2. Initialize collision_grid in new()
old2 = '            spatial_hash: crate::architecture::spatial_hash::SpatialHashGrid::new(10.0),\n        };'
new2 = '            spatial_hash: crate::architecture::spatial_hash::SpatialHashGrid::new(10.0),\n            collision_grid: crate::architecture::spatial_hash::SpatialHashGrid::new(0.5),\n        };'
if old2 in src:
    src = src.replace(old2, new2)
    changes.append('2. collision_grid init added')

# 3. Lower collision frequency: 6Hz -> 3Hz
old3 = '        if self.tick_count % 10 == 0 {\n            let particle_collisions = self.detect_particle_collisions();'
new3 = '        if self.tick_count % 20 == 0 {\n            let particle_collisions = self.detect_particle_collisions();'
if old3 in src:
    src = src.replace(old3, new3)
    changes.append('3. collision freq 6Hz -> 3Hz')

# 4. Add early-exit guards in handle_collision_damage
old4 = '    fn handle_collision_damage(&mut self, event: &CollisionDamageEvent, dt: f32) {\n        let fp_pos = FixedVec3::from_glam(event.position);\n        let fp_damage = FixedPoint::from_f32(event.damage);\n        let fp_radius = FixedPoint::from_f32(event.radius);\n\n        match event.damage_type {'
new4 = '    fn handle_collision_damage(&mut self, event: &CollisionDamageEvent, dt: f32) {\n        let fp_pos = FixedVec3::from_glam(event.position);\n        let fp_damage = FixedPoint::from_f32(event.damage);\n        let fp_radius = FixedPoint::from_f32(event.radius);\n        let has_voxels = !self.simulation.physics.voxel_grids.is_empty();\n        let has_npcs = !self.game_logic.npc_system.npcs.is_empty();\n\n        match event.damage_type {'
if old4 in src:
    src = src.replace(old4, new4)
    changes.append('4. has_voxels/has_npcs guards added')

# 5. Guard Kinetic damage
old5 = '            CrossDomainDamageType::Kinetic | CrossDomainDamageType::Piercing => {\n                for grid in &mut self.simulation.physics.voxel_grids {'
new5 = '            CrossDomainDamageType::Kinetic | CrossDomainDamageType::Piercing => {\n                if !has_voxels { return; }\n                for grid in &mut self.simulation.physics.voxel_grids {'
if old5 in src:
    src = src.replace(old5, new5)
    changes.append('5. Kinetic damage guarded')

# 6. Guard Thermal damage
old6 = '            CrossDomainDamageType::Thermal => {\n                for grid in &mut self.simulation.physics.voxel_grids {'
new6 = '            CrossDomainDamageType::Thermal => {\n                if !has_voxels { return; }\n                for grid in &mut self.simulation.physics.voxel_grids {'
if old6 in src:
    src = src.replace(old6, new6)
    changes.append('6. Thermal damage guarded')

# 7. Guard NPC loop
old7 = '        let max_dist = event.radius * 3.0;\n        for npc in &mut self.game_logic.npc_system.npcs {'
new7 = '        if !has_npcs { return; }\n        let max_dist = event.radius * 3.0;\n        for npc in &mut self.game_logic.npc_system.npcs {'
if old7 in src:
    src = src.replace(old7, new7)
    changes.append('7. NPC loop guarded')

# 8. Guard handle_chemical_reaction
old8 = '    fn handle_chemical_reaction(&mut self, event: &ChemicalReactionEvent, dt: f32) {\n        let fp_pos = FixedVec3::from_glam(event.position);\n        let fp_energy = FixedPoint::from_f32(event.energy_released);\n\n        match event.reaction_type {'
new8 = '    fn handle_chemical_reaction(&mut self, event: &ChemicalReactionEvent, dt: f32) {\n        let fp_pos = FixedVec3::from_glam(event.position);\n        let fp_energy = FixedPoint::from_f32(event.energy_released);\n        let has_voxels = !self.simulation.physics.voxel_grids.is_empty();\n        let has_ecosystems = !self.game_logic.ecosystems.is_empty();\n        let has_npcs = !self.game_logic.npc_system.npcs.is_empty();\n\n        match event.reaction_type {'
if old8 in src:
    src = src.replace(old8, new8)
    changes.append('8. handle_chemical_reaction guards added')

# 9. Guard explosion voxel loop
old9 = '            CrossDomainReactionType::Explosion => {\n                for grid in &mut self.simulation.physics.voxel_grids {'
new9 = '            CrossDomainReactionType::Explosion => {\n                if !has_voxels { self.global_temperature += event.energy_released * 0.0001; return; }\n                for grid in &mut self.simulation.physics.voxel_grids {'
if old9 in src:
    src = src.replace(old9, new9)
    changes.append('9. Explosion guarded')

with open(p, 'w', encoding='utf-8', newline='\n') as f:
    f.write(src)
for c in changes:
    print(c)
print(f'Total: {len(changes)} changes applied')
