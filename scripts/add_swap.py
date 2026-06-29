p = r'D:\rj\wasteland_project\wasteland_particle\src\mpss.rs'
with open(p, 'r', encoding='utf-8') as f:
    src = f.read()

# Add swap_particles method after kill method
old = '''    /// Mark a particle as dead (slot will be recycled)
    pub fn kill(&mut self, idx: usize) {
        if idx < self.capacity && self.active[idx] {
            self.active[idx] = false;
            self.free_list.push(idx);
            self.count -= 1;
        }
    }'''

new = '''    /// Mark a particle as dead (slot will be recycled)
    pub fn kill(&mut self, idx: usize) {
        if idx < self.capacity && self.active[idx] {
            self.active[idx] = false;
            self.free_list.push(idx);
            self.count -= 1;
        }
    }

    /// Swap two particles by index (used for LOD compaction)
    pub fn swap_particles(&mut self, a: usize, b: usize) {
        if a == b || a >= self.capacity || b >= self.capacity {
            return;
        }
        self.pos.swap(a, b);
        self.vel.swap(a, b);
        self.strain.swap(a, b);
        self.jacobian.swap(a, b);
        self.c.swap(a, b);
        self.force.swap(a, b);
        self.grid_vel.swap(a, b);
        self.mass.swap(a, b);
        self.kind.swap(a, b);
        self.chemical_id.swap(a, b);
        self.biomass.swap(a, b);
        self.temperature.swap(a, b);
        self.charge.swap(a, b);
        self.parent_id.swap(a, b);
        self.lifetime.swap(a, b);
        self.age.swap(a, b);
        self.subcell_strain.swap(a, b);
        self.material_idx.swap(a, b);
        self.active.swap(a, b);
    }'''

if old not in src:
    print('OLD NOT FOUND')
    raise SystemExit(1)
src = src.replace(old, new, 1)
with open(p, 'w', encoding='utf-8', newline='\n') as f:
    f.write(src)
print('swap_particles method added')
