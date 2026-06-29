use crate::WastelandWorld;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Node)]
struct WastelandModding {
    world_ref: Option<Gd<WastelandWorld>>,

    #[var]
    mods_directory: GString,

    #[var]
    sandbox_enabled: bool,

    loaded_mods: Vec<(GString, GString, GString, GString)>,
    mod_priorities: Vec<(GString, i64)>,
    api_versions: Vec<(GString, GString)>,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandModding {
    fn init(base: Base<Node>) -> Self {
        Self {
            world_ref: None,
            mods_directory: GString::from("mods/"),
            sandbox_enabled: true,
            loaded_mods: Vec::new(),
            mod_priorities: Vec::new(),
            api_versions: Vec::new(),
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
impl WastelandModding {
    fn sync_from_world(&mut self) {
        if let Some(ref _world) = self.world_ref {}
    }

    #[func]
    fn load_mod(&mut self, mod_name: GString) -> bool {
        let name_str = mod_name.to_string();
        if name_str.is_empty() {
            return false;
        }
        if self.loaded_mods.iter().any(|(n, _, _, _)| n.to_string() == name_str) {
            return false;
        }
        let version = GString::from("1.0.0");
        let author = GString::from("unknown");
        let deps = GString::from("");
        self.loaded_mods.push((mod_name, version, author, deps));
        true
    }

    #[func]
    fn unload_mod(&mut self, mod_name: GString) -> bool {
        let name_str = mod_name.to_string();
        let idx = self.loaded_mods.iter().position(|(n, _, _, _)| n.to_string() == name_str);
        if let Some(i) = idx {
            self.loaded_mods.remove(i);
            return true;
        }
        false
    }

    #[func]
    fn reload_all_mods(&mut self) -> bool {
        self.loaded_mods.clear();
        true
    }

    #[func]
    fn get_loaded_mods(&self) -> PackedStringArray {
        let mut arr = PackedStringArray::new();
        for (name, _, _, _) in &self.loaded_mods {
            arr.push(name);
        }
        arr
    }

    #[func]
    fn get_mod_count(&self) -> i64 {
        self.loaded_mods.len() as i64
    }

    #[func]
    fn is_mod_active(&self, mod_name: GString) -> bool {
        let name_str = mod_name.to_string();
        self.loaded_mods.iter().any(|(n, _, _, _)| n.to_string() == name_str)
    }

    #[func]
    fn get_mod_info(&self, mod_name: GString) -> Dictionary<Variant, Variant> {
        let name_str = mod_name.to_string();
        for (name, version, author, deps) in &self.loaded_mods {
            if name.to_string() == name_str {
                let version_clone: GString = version.clone();
                let author_clone: GString = author.clone();
                let deps_clone: GString = deps.clone();
                let status = GString::from("active");
                return dict! {
                    "name" => &version_clone,
                    "version" => name,
                    "author" => &author_clone,
                    "status" => &status,
                    "dependencies" => &deps_clone,
                };
            }
        }
        dict! {}
    }

    #[func]
    fn enable_sandbox(&mut self) {
        self.sandbox_enabled = true;
    }

    #[func]
    fn disable_sandbox(&mut self) {
        self.sandbox_enabled = false;
    }

    #[func]
    fn resolve_dependencies(&self, mod_name: GString) -> Dictionary<Variant, Variant> {
        let name_str = mod_name.to_string();
        let mut resolved = PackedStringArray::new();
        let mut missing = PackedStringArray::new();
        let mut found = false;
        for (name, _version, _author, deps) in &self.loaded_mods {
            if name.to_string() == name_str {
                found = true;
                let deps_str = deps.to_string();
                if !deps_str.is_empty() {
                    for dep in deps_str.split(',') {
                        let dep = dep.trim();
                        let dep_owned = dep.to_string();
                        if self.loaded_mods.iter().any(|(n, _, _, _)| n.to_string() == dep_owned) {
                            resolved.push(&GString::from(dep));
                        } else {
                            missing.push(&GString::from(dep));
                        }
                    }
                }
            }
        }
        dict! {
            "found" => found,
            "resolved" => &resolved,
            "missing" => &missing,
            "resolvable" => missing.is_empty(),
        }
    }

    #[func]
    fn get_load_order(&self) -> PackedStringArray {
        let mut arr = PackedStringArray::new();
        let n = self.loaded_mods.len();
        if n == 0 {
            return arr;
        }
        let mut in_degree: Vec<usize> = vec![0; n];
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (i, (_name, _ver, _auth, deps)) in self.loaded_mods.iter().enumerate() {
            let deps_str = deps.to_string();
            if deps_str.is_empty() {
                continue;
            }
            for dep in deps_str.split(',') {
                let dep = dep.trim();
                let dep_owned = dep.to_string();
                for (j, (n, _, _, _)) in self.loaded_mods.iter().enumerate() {
                    if n.to_string() == dep_owned {
                        adj[j].push(i);
                        in_degree[i] += 1;
                    }
                }
            }
        }
        let mut queue: Vec<usize> = Vec::new();
        for (i, &deg) in in_degree.iter().enumerate() {
            if deg == 0 {
                queue.push(i);
            }
        }
        while let Some(u) = queue.pop() {
            arr.push(&self.loaded_mods[u].0);
            for &v in &adj[u] {
                in_degree[v] -= 1;
                if in_degree[v] == 0 {
                    queue.push(v);
                }
            }
        }
        if arr.len() < n {
            for (name, _, _, _) in &self.loaded_mods {
                let name_str = name.to_string();
                let mut already = false;
                for j in 0..arr.len() {
                    if let Some(v) = arr.get(j) {
                        if v.to_string() == name_str {
                            already = true;
                            break;
                        }
                    }
                }
                if !already {
                    arr.push(name);
                }
            }
        }
        arr
    }

    #[func]
    fn detect_conflicts(&self) -> Array<Variant> {
        let mut arr = Array::<Variant>::new();
        let n = self.loaded_mods.len();
        for i in 0..n {
            for j in (i + 1)..n {
                let (name_a, _, _, _) = &self.loaded_mods[i];
                let (name_b, _, _, _) = &self.loaded_mods[j];
                let a_str = name_a.to_string();
                let b_str = name_b.to_string();
                let conflict = a_str.contains(&b_str) || b_str.contains(&a_str);
                if conflict {
                    let d: Dictionary<Variant, Variant> = dict! {
                        "mod_a" => name_a,
                        "mod_b" => name_b,
                        "reason" => &GString::from("name_overlap"),
                    };
                    arr.push(&Variant::from(d));
                }
            }
        }
        arr
    }

    #[func]
    fn validate_mod(&self, mod_name: GString) -> Dictionary<Variant, Variant> {
        let name_str = mod_name.to_string();
        let valid_name = !name_str.is_empty();
        let mut valid_version = false;
        let mut valid_deps = true;
        let mut found = false;
        for (name, version, _author, deps) in &self.loaded_mods {
            if name.to_string() == name_str {
                found = true;
                let ver_str = version.to_string();
                valid_version = ver_str.chars().filter(|c| *c == '.').count() == 2
                    && ver_str.split('.').all(|p| p.parse::<u32>().is_ok());
                let deps_str = deps.to_string();
                if !deps_str.is_empty() {
                    for dep in deps_str.split(',') {
                        let dep = dep.trim();
                        let dep_owned = dep.to_string();
                        if !dep.is_empty()
                            && !self.loaded_mods.iter().any(|(n, _, _, _)| n.to_string() == dep_owned)
                        {
                            valid_deps = false;
                        }
                    }
                }
                break;
            }
        }
        let valid = found && valid_name && valid_version && valid_deps;
        dict! {
            "valid" => valid,
            "found" => found,
            "valid_name" => valid_name,
            "valid_version" => valid_version,
            "valid_deps" => valid_deps,
        }
    }

    #[func]
    fn set_mod_priority(&mut self, mod_name: GString, priority: i64) -> bool {
        let name_str = mod_name.to_string();
        if name_str.is_empty() {
            return false;
        }
        let idx = self.mod_priorities.iter().position(|(n, _)| n.to_string() == name_str);
        if let Some(i) = idx {
            self.mod_priorities[i].1 = priority;
        } else {
            self.mod_priorities.push((mod_name, priority));
        }
        true
    }

    #[func]
    fn get_mod_api_version(&self, mod_name: GString) -> GString {
        let name_str = mod_name.to_string();
        for (n, ver) in &self.api_versions {
            if n.to_string() == name_str {
                return ver.clone();
            }
        }
        GString::from("0.0.0")
    }
}
