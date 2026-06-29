$path = "D:\rj\wasteland_project\wasteland_engine\src\lib.rs"
$lines = [System.IO.File]::ReadAllLines($path)
$propagateStart = 265
$thermalStart = 573

$newCode = @"
    /// 发布跨域事件到 EventBus
    fn publish_cross_domain_events(&mut self) {
        for event in self.simulation.physics.collision_events.iter() {
            if event.is_significant() {
                let damages = event.calculate_damage();
                for damage in &damages {
                    let cd_type = match damage.damage_type {
                        DamageType::Explosive => CrossDomainDamageType::Explosive,
                        DamageType::Kinetic => CrossDomainDamageType::Kinetic,
                        DamageType::Piercing => CrossDomainDamageType::Piercing,
                        DamageType::Thermal => CrossDomainDamageType::Thermal,
                        DamageType::Chemical => CrossDomainDamageType::Chemical,
                        DamageType::Radiation => CrossDomainDamageType::Radiation,
                        _ => continue,
                    };
                    self.event_bus.publish(Box::new(CollisionDamageEvent {
                        damage_type: cd_type,
                        position: damage.point.to_glam(),
                        radius: damage.radius.to_f32(),
                        damage: damage.damage.to_f32(),
                    }));
                }
            }
        }
        self.simulation.physics.collision_events.clear();

        for result in self.simulation.chemistry.completed_reactions.iter() {
            let cd_reaction_type = match result.reaction.reaction_type {
                ReactionType::Explosion => CrossDomainReactionType::Explosion,
                ReactionType::Combustion => CrossDomainReactionType::Combustion,
                ReactionType::RadioactiveDecay => CrossDomainReactionType::RadioactiveDecay,
                ReactionType::Corrosion => CrossDomainReactionType::Corrosion,
                ReactionType::Oxidation => CrossDomainReactionType::Oxidation,
                _ => CrossDomainReactionType::Other,
            };
            let byproducts: Vec<ChemicalByproductInfo> = result.byproducts.iter().map(|b| {
                let hazard = match b.hazard {
                    HazardType::Radiation => CrossDomainHazardType::Radiation,
                    HazardType::ToxicFumes => CrossDomainHazardType::ToxicFumes,
                    HazardType::BiologicalContamination => CrossDomainHazardType::BiologicalContamination,
                    _ => CrossDomainHazardType::Other,
                };
                ChemicalByproductInfo { hazard, amount: b.amount, spread_radius: b.spread_radius, duration: b.duration }
            }).collect();
            self.event_bus.publish(Box::new(ChemicalReactionEvent {
                reaction_type: cd_reaction_type,
                position: result.position,
                energy_released: result.energy_released,
                byproducts,
            }));
        }
        self.simulation.chemistry.completed_reactions.clear();
    }

    /// 处理 EventBus 中的跨域事件
    fn process_cross_domain_events(&mut self, dt: f32) {
        let events = self.event_bus.drain_events();
        for event in events {
            let etype = event.event_type();
            match etype {
                architecture::COLLISION_DAMAGE => {
                    if let Some(e) = event.as_any().downcast_ref::<CollisionDamageEvent>() {
                        let e = e.clone();
                        self.handle_collision_damage(&e, dt);
                    }
                }
                architecture::CHEMICAL_REACTION => {
                    if let Some(e) = event.as_any().downcast_ref::<ChemicalReactionEvent>() {
                        let e = e.clone();
                        self.handle_chemical_reaction(&e, dt);
                    }
                }
                _ => {}
            }
        }
    }

    /// 处理碰撞伤害事件
    fn handle_collision_damage(&mut self, event: &CollisionDamageEvent, dt: f32) {
        let fp_pos = FixedVec3::from_glam(event.position);
        let fp_damage = FixedPoint::from_f32(event.damage);
        let fp_radius = FixedPoint::from_f32(event.radius);

        match event.damage_type {
            CrossDomainDamageType::Explosive => {
                self.simulation.chemistry.trigger_reaction(
                    ChemicalReaction::explosion_tnt(), event.position, event.damage * 0.01,
                );
            }
            CrossDomainDamageType::Kinetic | CrossDomainDamageType::Piercing => {
                for grid in &mut self.simulation.physics.voxel_grids {
                    let destroyed = grid.damage_sphere(fp_pos, fp_radius, fp_damage * FixedPoint::from_f32(0.1));
                    if !destroyed.is_empty() {
                        let origin = destroyed[0];
                        for pos in &destroyed[1..] {
                            grid.fracture_propagate(*pos, fp_damage * FixedPoint::from_f32(0.05));
                        }
                        grid.fracture_propagate(origin, fp_damage * FixedPoint::from_f32(0.1));
                    }
                }
            }
            CrossDomainDamageType::Thermal => {
                for grid in &mut self.simulation.physics.voxel_grids {
                    grid.apply_heat(fp_pos, fp_radius, FixedPoint::from_f32(600.0), FixedPoint::from_f32(dt));
                    grid.thermal_conduction_step(FixedPoint::from_f32(dt * 10.0));
                }
            }
            CrossDomainDamageType::Chemical => {
                self.simulation.chemistry.trigger_reaction(
                    ChemicalReaction::acid_corrosion(), event.position, event.damage * 0.05,
                );
            }
            CrossDomainDamageType::Radiation => {
                self.global_radiation = (self.global_radiation + event.damage * 0.1).min(1000.0);
            }
            _ => {}
        }

        let max_dist = event.radius * 3.0;
        for npc in &mut self.game_logic.npc_system.npcs {
            if !npc.alive { continue; }
            let dist = (npc.position - event.position).length();
            if dist < max_dist {
                let falloff = 1.0 - dist / max_dist;
                let npc_damage = event.damage * falloff * 0.1;
                let dmg_type = match event.damage_type {
                    CrossDomainDamageType::Explosive => "explosive",
                    CrossDomainDamageType::Kinetic => "physical",
                    CrossDomainDamageType::Thermal => "thermal",
                    CrossDomainDamageType::Chemical => "chemical",
                    CrossDomainDamageType::Radiation => "radiation",
                    _ => "physical",
                };
                npc.apply_damage(npc_damage, dmg_type);
                let knockback = (npc.position - event.position).normalize_or_zero() * falloff * 10.0;
                npc.velocity += knockback;
            }
        }
    }

    /// 处理化学反应事件
    fn handle_chemical_reaction(&mut self, event: &ChemicalReactionEvent, dt: f32) {
        let fp_pos = FixedVec3::from_glam(event.position);
        let fp_energy = FixedPoint::from_f32(event.energy_released);

        match event.reaction_type {
            CrossDomainReactionType::Explosion => {
                for grid in &mut self.simulation.physics.voxel_grids {
                    let destroyed = grid.damage_sphere(fp_pos, FixedPoint::from_f32(5.0), fp_energy * FixedPoint::from_f32(0.1));
                    if !destroyed.is_empty() {
                        grid.fracture_propagate(destroyed[0], fp_energy * FixedPoint::from_f32(0.05));
                    }
                }
                self.global_temperature += event.energy_released * 0.0001;
            }
            CrossDomainReactionType::Combustion => {
                self.global_temperature += event.energy_released * 0.0001;
                for grid in &mut self.simulation.physics.voxel_grids {
                    grid.apply_heat(fp_pos, FixedPoint::from_f32(3.0), fp_energy * FixedPoint::from_f32(0.5), FixedPoint::from_f32(dt));
                }
            }
            CrossDomainReactionType::RadioactiveDecay => {
                let rads = event.energy_released * 0.01;
                self.global_radiation = (self.global_radiation + rads).min(1000.0);
                let fp_rads = FixedPoint::from_f32(rads);
                for grid in &mut self.simulation.physics.voxel_grids {
                    grid.apply_radiation(fp_pos, FixedPoint::from_f32(50.0), fp_rads, FixedPoint::from_f32(dt));
                }
                for ecosystem in &mut self.game_logic.ecosystems {
                    ecosystem.radiation_level = self.global_radiation;
                }
            }
            CrossDomainReactionType::Corrosion => {
                for grid in &mut self.simulation.physics.voxel_grids {
                    grid.damage_sphere(fp_pos, FixedPoint::from_f32(2.0), fp_energy * FixedPoint::from_f32(0.01));
                }
            }
            CrossDomainReactionType::Oxidation => {
                for grid in &mut self.simulation.physics.voxel_grids {
                    grid.apply_heat(fp_pos, FixedPoint::from_f32(1.0), fp_energy * FixedPoint::from_f32(0.5), FixedPoint::from_f32(dt));
                }
            }
            _ => {}
        }

        for byproduct in &event.byproducts {
            match byproduct.hazard {
                CrossDomainHazardType::Radiation => {
                    for ecosystem in &mut self.game_logic.ecosystems {
                        ecosystem.radiation_level += byproduct.amount * 0.1;
                    }
                    for npc in &mut self.game_logic.npc_system.npcs {
                        if !npc.alive { continue; }
                        let dist = (npc.position - event.position).length();
                        if dist < byproduct.spread_radius {
                            npc.apply_damage(byproduct.amount * 0.5 * (1.0 - dist / byproduct.spread_radius), "radiation");
                        }
                    }
                }
                CrossDomainHazardType::ToxicFumes | CrossDomainHazardType::BiologicalContamination => {
                    for ecosystem in &mut self.game_logic.ecosystems {
                        for org in &mut ecosystem.organisms {
                            let dist = (org.position - event.position).length();
                            if dist < byproduct.spread_radius {
                                org.metabolism.add_toxin(
                                    format!("{:?}_toxin", byproduct.hazard),
                                    byproduct.amount * (1.0 - dist / byproduct.spread_radius),
                                    ToxinSource::Chemical, byproduct.duration,
                                );
                            }
                        }
                    }
                    for npc in &mut self.game_logic.npc_system.npcs {
                        if !npc.alive { continue; }
                        let dist = (npc.position - event.position).length();
                        if dist < byproduct.spread_radius {
                            npc.apply_damage(byproduct.amount * 0.3 * (1.0 - dist / byproduct.spread_radius), "toxin");
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// 生态系统与体素场交互（每帧常规查询）
    fn update_ecosystem_voxel_interactions(&mut self, dt: f32) {
        for ecosystem in &mut self.game_logic.ecosystems {
            for i in 0..ecosystem.organisms.len() {
                if ecosystem.organisms[i].state == OrganismState::Dead { continue; }
                let org_pos = ecosystem.organisms[i].position;
                let fp_org_pos = FixedVec3::from_glam(org_pos);
                for grid in &self.simulation.physics.voxel_grids {
                    if let Some(voxel_pos) = grid.world_to_voxel(fp_org_pos) {
                        if let Some(voxel) = grid.get_voxel(voxel_pos) {
                            ecosystem.organisms[i].radiation_dose += voxel.radiation_level.to_f32() * dt * 0.1;
                            if voxel.temperature > FixedPoint::from_f32(350.0) {
                                ecosystem.organisms[i].take_damage(
                                    (voxel.temperature - FixedPoint::from_f32(350.0)).to_f32() * dt * 0.1, "thermal",
                                );
                            }
                        }
                    }
                }
            }
        }
    }
"@

$newLines = @()
$newLines += $lines[0..($propagateStart - 1)]
$newLines += $newCode -split "`n"
$newLines += $lines[$thermalStart..($lines.Length - 1)]

$utf8NoBom = New-Object System.Text.UTF8Encoding $false
[System.IO.File]::WriteAllLines($path, $newLines, $utf8NoBom)
Write-Output "Done. Replaced lines $($propagateStart+1)-$thermalStart with $($newCode.Split("`n").Count) new lines"