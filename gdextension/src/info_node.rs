use crate::WastelandWorld;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Node)]
struct WastelandInfo {
    world_ref: Option<Gd<WastelandWorld>>,

    #[var]
    propagation_speed: f32,

    #[var]
    signal_decay: f32,

    #[var]
    max_distance: f32,

    active_signals: i64,
    node_count: i64,
    edge_count: i64,
    propagation_sources: Vec<(f32, f32, f32, GString, GString)>,
    knowledge_nodes: Vec<(GString, GString, GString)>,
    knowledge_edges: Vec<(GString, GString, GString, f32)>,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandInfo {
    fn init(base: Base<Node>) -> Self {
        Self {
            world_ref: None,
            propagation_speed: 1.0,
            signal_decay: 0.1,
            max_distance: 100.0,
            active_signals: 0,
            node_count: 0,
            edge_count: 0,
            propagation_sources: Vec::new(),
            knowledge_nodes: Vec::new(),
            knowledge_edges: Vec::new(),
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
impl WastelandInfo {
    fn sync_from_world(&mut self) {
        if let Some(ref world) = self.world_ref {
            let data = world.bind().get_stats();
            if let Some(v) = data.get("npc_count") {
                self.node_count = v.to::<i64>();
                self.edge_count = (v.to::<i64>() * 2).max(0);
            }
        }
    }

    #[func]
    fn inject_signal(
        &mut self,
        topic: GString,
        content: GString,
        x: f32,
        y: f32,
        z: f32,
        radius: f32,
    ) -> bool {
        if topic.to_string().is_empty() || radius <= 0.0 {
            return false;
        }
        self.propagation_sources.push((x, y, z, topic, content));
        self.active_signals = self.propagation_sources.len() as i64;
        true
    }

    #[func]
    fn query_knowledge(&self, topic: GString) -> PackedStringArray {
        let mut arr = PackedStringArray::new();
        let topic_str = topic.to_string();
        for (_, _, _, t, c) in &self.propagation_sources {
            if t.to_string().contains(&topic_str) {
                arr.push(c);
            }
        }
        arr
    }

    #[func]
    fn get_propagation_sources(&self) -> PackedVector3Array {
        let mut arr = PackedVector3Array::new();
        for (x, y, z, _, _) in &self.propagation_sources {
            arr.push(Vector3::new(*x, *y, *z));
        }
        arr
    }

    #[func]
    fn get_active_signals(&self) -> i64 {
        self.active_signals
    }

    #[func]
    fn get_graph_stats(&self) -> Dictionary<Variant, Variant> {
        let avg_degree =
            if self.node_count > 0 { self.edge_count as f32 / self.node_count as f32 } else { 0.0 };
        dict! {
            "node_count" => self.node_count,
            "edge_count" => self.edge_count,
            "avg_degree" => avg_degree,
        }
    }

    #[func]
    fn get_related_topics(&self, topic: GString, max_depth: i64) -> PackedStringArray {
        let mut arr = PackedStringArray::new();
        let topic_str = topic.to_string();
        let depth = max_depth.clamp(1, 10) as usize;
        for (_, _, _, t, _) in &self.propagation_sources {
            let t_str = t.to_string();
            if t_str != topic_str && arr.len() < depth {
                arr.push(t);
            }
        }
        arr
    }

    #[func]
    fn add_knowledge_node(&mut self, id: GString, label: GString, category: GString) -> bool {
        let id_str = id.to_string();
        if id_str.is_empty() {
            return false;
        }
        if self.knowledge_nodes.iter().any(|(i, _, _)| i.to_string() == id_str) {
            return false;
        }
        self.knowledge_nodes.push((id, label, category));
        self.node_count = self.knowledge_nodes.len() as i64;
        true
    }

    #[func]
    fn add_knowledge_edge(
        &mut self,
        from_id: GString,
        to_id: GString,
        relation_type: GString,
        weight: f32,
    ) -> bool {
        let from_str = from_id.to_string();
        let to_str = to_id.to_string();
        if from_str.is_empty() || to_str.is_empty() {
            return false;
        }
        let from_exists = self.knowledge_nodes.iter().any(|(i, _, _)| i.to_string() == from_str);
        let to_exists = self.knowledge_nodes.iter().any(|(i, _, _)| i.to_string() == to_str);
        if !from_exists || !to_exists {
            return false;
        }
        self.knowledge_edges.push((from_id, to_id, relation_type, weight));
        self.edge_count = self.knowledge_edges.len() as i64;
        true
    }

    #[func]
    fn query_knowledge_graph(&self, topic: GString, max_hops: i64) -> Array<Variant> {
        let mut arr = Array::<Variant>::new();
        let topic_str = topic.to_string();
        let hops = max_hops.clamp(1, 5) as usize;
        let mut visited: Vec<String> = Vec::new();
        visited.push(topic_str.clone());
        for _hop in 0..hops {
            let mut next: Vec<String> = Vec::new();
            for v in &visited {
                for (from, to, rel, w) in &self.knowledge_edges {
                    let f = from.to_string();
                    let t = to.to_string();
                    if f == *v && !visited.contains(&t) && !next.contains(&t) {
                        let d: Dictionary<Variant, Variant> = dict! {
                            "from" => from,
                            "to" => to,
                            "relation" => rel,
                            "weight" => *w,
                        };
                        arr.push(&Variant::from(d));
                        next.push(t.clone());
                    }
                    if t == *v && !visited.contains(&f) && !next.contains(&f) {
                        let d: Dictionary<Variant, Variant> = dict! {
                            "from" => to,
                            "to" => from,
                            "relation" => rel,
                            "weight" => *w,
                        };
                        arr.push(&Variant::from(d));
                        next.push(f);
                    }
                }
            }
            for n in &next {
                visited.push(n.clone());
            }
        }
        arr
    }

    #[func]
    fn compute_centrality(&self, node_id: GString) -> f32 {
        let id_str = node_id.to_string();
        let n = self.knowledge_nodes.len();
        if n == 0 {
            return 0.0;
        }
        let mut degree = 0usize;
        for (from, to, _, _) in &self.knowledge_edges {
            if from.to_string() == id_str || to.to_string() == id_str {
                degree += 1;
            }
        }
        degree as f32 / (n as f32 * 2.0).max(1.0)
    }

    #[func]
    fn detect_communities(&self) -> PackedInt32Array {
        let mut arr = PackedInt32Array::new();
        let n = self.knowledge_nodes.len();
        if n == 0 {
            return arr;
        }
        let mut communities: Vec<i32> = (0..n as i32).collect();
        for _iter in 0..5 {
            for (from, to, _, _) in &self.knowledge_edges {
                let fi = self
                    .knowledge_nodes
                    .iter()
                    .position(|(i, _, _)| i.to_string() == from.to_string());
                let ti = self
                    .knowledge_nodes
                    .iter()
                    .position(|(i, _, _)| i.to_string() == to.to_string());
                if let (Some(fi), Some(ti)) = (fi, ti) {
                    let cf = communities[fi];
                    let ct = communities[ti];
                    let mut count: std::collections::HashMap<i32, i32> =
                        std::collections::HashMap::new();
                    for (f, t, _, _) in &self.knowledge_edges {
                        let fi2 = self
                            .knowledge_nodes
                            .iter()
                            .position(|(i, _, _)| i.to_string() == f.to_string());
                        let ti2 = self
                            .knowledge_nodes
                            .iter()
                            .position(|(i, _, _)| i.to_string() == t.to_string());
                        if let (Some(fi2), Some(ti2)) = (fi2, ti2) {
                            if communities[fi2] == ct || communities[ti2] == ct {
                                *count.entry(cf).or_insert(0) += 1;
                            }
                            if communities[fi2] == cf || communities[ti2] == cf {
                                *count.entry(ct).or_insert(0) += 1;
                            }
                        }
                    }
                    let best = count.iter().max_by_key(|(_, v)| *v);
                    if let Some((&new_comm, _)) = best {
                        communities[ti] = new_comm;
                    }
                }
            }
        }
        for c in &communities {
            arr.push(*c);
        }
        arr
    }

    #[func]
    fn propagate_rumor(
        &self,
        origin_x: f32,
        origin_y: f32,
        origin_z: f32,
        rumor_text: GString,
        max_radius: f32,
    ) -> Array<Variant> {
        let mut arr = Array::<Variant>::new();
        let rumor = rumor_text.to_string();
        let radius = max_radius.max(1.0);
        let steps = 8;
        for i in 0..steps {
            let dist = radius * (i as f32 / steps as f32);
            let angle = i as f32 * 0.785;
            let rx = origin_x + dist * angle.cos();
            let ry = origin_y + dist * angle.sin();
            let rz = origin_z + (angle * 0.5).sin() * dist * 0.2;
            let decay = 1.0 - (dist / radius).min(1.0);
            let reach = decay * (self.propagation_speed * self.signal_decay + 0.1).min(1.0);
            let d: Dictionary<Variant, Variant> = dict! {
                "x" => rx,
                "y" => ry,
                "z" => rz,
                "reach" => reach,
                "distance" => dist,
                "text" => &GString::from(rumor.as_str()),
            };
            arr.push(&Variant::from(d));
        }
        arr
    }
}
