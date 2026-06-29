use crate::WastelandWorld;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Node, rename=WastelandChemistryNode)]
struct WastelandChemistryNode {
    world_ref: Option<Gd<WastelandWorld>>,

    #[var]
    reaction_speed: f32,

    #[var]
    ambient_ph: f32,

    active_reactions: i64,
    element_count: i64,
    #[allow(dead_code)]
    catalyst_count: i64,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandChemistryNode {
    fn init(base: Base<Node>) -> Self {
        Self {
            world_ref: None,
            reaction_speed: 1.0,
            ambient_ph: 7.0,
            active_reactions: 0,
            element_count: 0,
            catalyst_count: 0,
            base,
        }
    }

    fn ready(&mut self) {
        if let Some(parent) = self.base().get_parent() {
            if let Ok(world) = parent.try_cast::<WastelandWorld>() {
                self.world_ref = Some(world);
            }
        }
    }

    fn process(&mut self, _delta: f64) {
        self.sync_from_world();
    }
}

#[godot_api]
impl WastelandChemistryNode {
    fn sync_from_world(&mut self) {
        if let Some(ref world) = self.world_ref {
            let stats = world.bind().get_stats();
            self.active_reactions =
                stats.get("active_reactions").map(|v| v.to::<i64>()).unwrap_or(0);
            self.element_count = stats.get("element_count").map(|v| v.to::<i64>()).unwrap_or(0);
        }
    }

    #[func]
    fn get_reaction_count(&self) -> i64 {
        self.active_reactions
    }

    #[func]
    fn get_element_count(&self) -> i64 {
        self.element_count
    }
}
