use glam::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FloodType {
    Riverine,
    Flash,
    Coastal,
    Pluvial,
    DamBreak,
    Groundwater,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FloodSeverity {
    Minor,
    Moderate,
    Major,
    Catastrophic,
}

impl FloodSeverity {
    pub fn from_depth(depth: f32) -> Self {
        if depth < 0.5 {
            FloodSeverity::Minor
        } else if depth < 1.5 {
            FloodSeverity::Moderate
        } else if depth < 3.0 {
            FloodSeverity::Major
        } else {
            FloodSeverity::Catastrophic
        }
    }

    pub fn damage_factor(&self) -> f32 {
        match self {
            FloodSeverity::Minor => 0.05,
            FloodSeverity::Moderate => 0.2,
            FloodSeverity::Major => 0.5,
            FloodSeverity::Catastrophic => 0.9,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloodCell {
    pub position: Vec3,
    pub water_depth: f32,
    pub flow_velocity: Vec3,
    pub inundation_time: f32,
    pub sediment_deposit: f32,
    pub is_inundated: bool,
}

impl FloodCell {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            water_depth: 0.0,
            flow_velocity: Vec3::ZERO,
            inundation_time: 0.0,
            sediment_deposit: 0.0,
            is_inundated: false,
        }
    }

    pub fn severity(&self) -> FloodSeverity {
        FloodSeverity::from_depth(self.water_depth)
    }

    pub fn hazard_index(&self) -> f32 {
        let depth_factor = (self.water_depth / 3.0).min(1.0);
        let velocity_factor = (self.flow_velocity.length() / 3.0).min(1.0);
        let time_factor = (self.inundation_time / 3600.0).min(1.0);
        depth_factor * 0.5 + velocity_factor * 0.3 + time_factor * 0.2
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloodModel {
    pub cells: Vec<FloodCell>,
    pub grid_width: usize,
    pub grid_height: usize,
    pub cell_size: f32,
    pub flood_type: FloodType,
    pub manning_n: f32,
    pub total_water_volume: f32,
    pub source_points: Vec<FloodSource>,
    pub water_surface_elevation: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloodSource {
    pub position: Vec3,
    pub discharge: f32,
    pub duration: f32,
    pub remaining_time: f32,
}

impl FloodSource {
    pub fn new(position: Vec3, discharge: f32, duration: f32) -> Self {
        Self { position, discharge, duration, remaining_time: duration }
    }

    pub fn is_active(&self) -> bool {
        self.remaining_time > 0.0
    }
}

impl FloodModel {
    pub fn new(
        grid_width: usize,
        grid_height: usize,
        cell_size: f32,
        flood_type: FloodType,
    ) -> Self {
        let cells = (0..grid_height)
            .flat_map(|y| {
                (0..grid_width).map(move |x| {
                    FloodCell::new(Vec3::new(x as f32 * cell_size, 0.0, y as f32 * cell_size))
                })
            })
            .collect();

        let n = grid_width * grid_height;
        Self {
            cells,
            grid_width,
            grid_height,
            cell_size,
            flood_type,
            manning_n: 0.04,
            total_water_volume: 0.0,
            source_points: Vec::new(),
            water_surface_elevation: vec![0.0; n],
        }
    }

    pub fn index(&self, x: usize, y: usize) -> usize {
        y * self.grid_width + x
    }

    pub fn add_source(&mut self, source: FloodSource) {
        self.source_points.push(source);
    }

    pub fn topographic_gradient(&self, elevations: &[f32], idx: usize, x: usize, y: usize) -> Vec3 {
        let cx = elevations[idx];
        let left = if x > 0 { elevations[self.index(x - 1, y)] } else { cx };
        let right = if x < self.grid_width - 1 { elevations[self.index(x + 1, y)] } else { cx };
        let up = if y > 0 { elevations[self.index(x, y - 1)] } else { cx };
        let down = if y < self.grid_height - 1 { elevations[self.index(x, y + 1)] } else { cx };

        let dx = (left - right) / (2.0 * self.cell_size);
        let dz = (up - down) / (2.0 * self.cell_size);
        Vec3::new(dx, 0.0, dz)
    }

    pub fn flow_accumulation(&self, elevations: &[f32]) -> Vec<f32> {
        let n = self.grid_width * self.grid_height;
        let mut accumulation = vec![0.0; n];

        for y in 0..self.grid_height {
            for x in 0..self.grid_width {
                let idx = self.index(x, y);
                let grad = self.topographic_gradient(elevations, idx, x, y);
                let slope = grad.length();

                if slope > 0.0001 {
                    let dir = grad / slope;
                    let nx = x as f32 + dir.x;
                    let nz = y as f32 + dir.z;
                    let nx = nx.round().clamp(0.0, (self.grid_width - 1) as f32) as usize;
                    let nz = nz.round().clamp(0.0, (self.grid_height - 1) as f32) as usize;
                    let neighbor_idx = self.index(nx, nz);
                    if neighbor_idx != idx {
                        accumulation[neighbor_idx] += 1.0;
                    }
                }
            }
        }

        accumulation
    }

    pub fn step(&mut self, elevations: &[f32], dt: f32) {
        let n = self.grid_width * self.grid_height;
        let mut new_depth = vec![0.0; n];
        let mut new_velocity = vec![Vec3::ZERO; n];

        let cell_size = self.cell_size;
        let grid_w = self.grid_width;
        let grid_h = self.grid_height;

        let mut source_data: Vec<(usize, f32)> = Vec::new();
        for source in &mut self.source_points {
            if source.is_active() {
                let sx = (source.position.x / cell_size).round().clamp(0.0, (grid_w - 1) as f32)
                    as usize;
                let sy = (source.position.z / cell_size).round().clamp(0.0, (grid_h - 1) as f32)
                    as usize;
                let s_idx = sy * grid_w + sx;
                let vol = source.discharge * dt;
                source.remaining_time -= dt;
                source_data.push((s_idx, vol));
            }
        }

        for (s_idx, vol) in source_data {
            self.cells[s_idx].water_depth += vol / (cell_size * cell_size);
        }

        for y in 0..self.grid_height {
            for x in 0..self.grid_width {
                let idx = self.index(x, y);
                let depth = self.cells[idx].water_depth;

                if depth < 0.001 {
                    continue;
                }

                let _ws_elev = elevations[idx] + depth;
                let grad = self.topographic_gradient(&self.water_surface_elevation, idx, x, y);
                let slope = grad.length();

                if slope < 0.0001 {
                    continue;
                }

                let h_radius = depth.max(0.001);
                let v = (1.0 / self.manning_n) * h_radius.powf(2.0 / 3.0) * slope.sqrt();
                let flow_dir = grad / slope;

                let outflow = depth * v * self.cell_size * dt;
                let nx = (x as f32 + flow_dir.x).round().clamp(0.0, (self.grid_width - 1) as f32)
                    as usize;
                let nz = (y as f32 + flow_dir.z).round().clamp(0.0, (self.grid_height - 1) as f32)
                    as usize;
                let neighbor_idx = self.index(nx, nz);

                let transfer = outflow.min(depth * self.cell_size * self.cell_size * 0.5);
                let transfer_depth = transfer / (self.cell_size * self.cell_size);

                new_depth[idx] -= transfer_depth;
                new_depth[neighbor_idx] += transfer_depth;
                new_velocity[idx] = flow_dir * v;
            }
        }

        for i in 0..n {
            self.cells[i].water_depth = (self.cells[i].water_depth + new_depth[i]).max(0.0);
            self.cells[i].flow_velocity = new_velocity[i];
            self.cells[i].is_inundated = self.cells[i].water_depth > 0.01;

            if self.cells[i].is_inundated {
                self.cells[i].inundation_time += dt;
            }
        }

        self.total_water_volume =
            self.cells.iter().map(|c| c.water_depth * self.cell_size * self.cell_size).sum();

        let water_surface = &mut self.water_surface_elevation;
        let cells = &self.cells;
        for ((ws, &e), cell) in water_surface.iter_mut().zip(elevations.iter()).zip(cells.iter()) {
            *ws = e + cell.water_depth;
        }
    }

    pub fn inundation_area(&self) -> f32 {
        let count = self.cells.iter().filter(|c| c.is_inundated).count();
        count as f32 * self.cell_size * self.cell_size
    }

    pub fn average_depth(&self) -> f32 {
        let inundated: Vec<&FloodCell> = self.cells.iter().filter(|c| c.is_inundated).collect();
        if inundated.is_empty() {
            return 0.0;
        }
        inundated.iter().map(|c| c.water_depth).sum::<f32>() / inundated.len() as f32
    }

    pub fn max_depth(&self) -> f32 {
        self.cells.iter().map(|c| c.water_depth).fold(0.0f32, f32::max)
    }

    pub fn severity_distribution(&self) -> Vec<(FloodSeverity, usize)> {
        let mut counts = vec![
            (FloodSeverity::Minor, 0),
            (FloodSeverity::Moderate, 0),
            (FloodSeverity::Major, 0),
            (FloodSeverity::Catastrophic, 0),
        ];
        for cell in &self.cells {
            if cell.is_inundated {
                let sev = cell.severity();
                for (s, count) in &mut counts {
                    if *s == sev {
                        *count += 1;
                        break;
                    }
                }
            }
        }
        counts
    }

    pub fn erosion_potential(&self, idx: usize, soil_erodibility: f32) -> f32 {
        let cell = &self.cells[idx];
        if !cell.is_inundated {
            return 0.0;
        }
        let shear = 1000.0 * 9.81 * cell.water_depth * cell.flow_velocity.length().powi(2) * 0.001;
        shear * soil_erodibility
    }

    pub fn compute_erosion(
        &mut self,
        elevations: &[f32],
        soil_erodibility: f32,
        dt: f32,
    ) -> Vec<f32> {
        let mut new_elevations = elevations.to_vec();

        for (i, new_elev) in new_elevations.iter_mut().enumerate() {
            let erosion = self.erosion_potential(i, soil_erodibility) * dt;
            *new_elev -= erosion * 0.001;
            self.cells[i].sediment_deposit += erosion * 0.5;
        }

        new_elevations
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloodWave {
    pub position: Vec3,
    pub amplitude: f32,
    pub speed: f32,
    pub wavelength: f32,
    pub direction: Vec3,
    pub age: f32,
}

impl FloodWave {
    pub fn new(position: Vec3, amplitude: f32, direction: Vec3) -> Self {
        Self {
            position,
            amplitude,
            speed: (9.81 * amplitude).sqrt(),
            wavelength: amplitude * 20.0,
            direction: direction.normalize(),
            age: 0.0,
        }
    }

    pub fn propagate(&mut self, dt: f32) {
        self.position += self.direction * self.speed * dt;
        self.age += dt;
        self.amplitude *= (-0.001 * self.age).exp();
    }

    pub fn water_surface_height(&self, pos: Vec3, time: f32) -> f32 {
        let dist = (pos - self.position).dot(self.direction);
        let phase = 2.0
            * std::f32::consts::PI
            * (dist / self.wavelength - time * self.speed / self.wavelength);
        self.amplitude * phase.sin() * (-0.001 * self.age).exp()
    }

    pub fn is_dissipated(&self) -> bool {
        self.amplitude < 0.01
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloodDamage {
    pub affected_area: f32,
    pub structural_damage: f32,
    pub agricultural_loss: f32,
    pub infrastructure_damage: f32,
    pub economic_cost: f32,
    pub casualties_risk: f32,
}

impl FloodDamage {
    pub fn assess(
        flood_model: &FloodModel,
        population_density: f32,
        infrastructure_value: f32,
    ) -> Self {
        let mut affected = 0.0;
        let mut structural = 0.0;
        let mut agricultural = 0.0;
        let mut infra = 0.0;

        for cell in &flood_model.cells {
            if !cell.is_inundated {
                continue;
            }
            let sev = cell.severity();
            let factor = sev.damage_factor();
            let area = flood_model.cell_size * flood_model.cell_size;

            affected += area;
            structural += factor * area * 0.3;
            agricultural += factor * area * 0.2;
            infra += factor * area * infrastructure_value * 0.01;
        }

        let economic = structural * 1000.0 + agricultural * 500.0 + infra * 2000.0;
        let risk = affected * population_density * flood_model.average_depth() * 0.001;

        Self {
            affected_area: affected,
            structural_damage: structural,
            agricultural_loss: agricultural,
            infrastructure_damage: infra,
            economic_cost: economic,
            casualties_risk: risk,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flood_model_creation() {
        let model = FloodModel::new(10, 10, 1.0, FloodType::Riverine);
        assert_eq!(model.cells.len(), 100);
        assert_eq!(model.grid_width, 10);
        assert_eq!(model.grid_height, 10);
        assert_eq!(model.total_water_volume, 0.0);
    }

    #[test]
    fn test_flood_model_with_source() {
        let mut model = FloodModel::new(10, 10, 10.0, FloodType::Flash);
        let source = FloodSource::new(Vec3::new(50.0, 0.0, 50.0), 100.0, 10.0);
        model.add_source(source);

        let elevations = vec![0.0; 100];
        model.step(&elevations, 1.0);
        assert!(model.total_water_volume > 0.0);
    }

    #[test]
    fn test_flood_severity() {
        assert_eq!(FloodSeverity::from_depth(0.3), FloodSeverity::Minor);
        assert_eq!(FloodSeverity::from_depth(1.0), FloodSeverity::Moderate);
        assert_eq!(FloodSeverity::from_depth(2.0), FloodSeverity::Major);
        assert_eq!(FloodSeverity::from_depth(5.0), FloodSeverity::Catastrophic);
    }

    #[test]
    fn test_flood_wave_propagation() {
        let mut wave = FloodWave::new(Vec3::ZERO, 2.0, Vec3::X);
        let initial_pos = wave.position;
        wave.propagate(1.0);
        assert!(wave.position.x > initial_pos.x);
        assert!(wave.age > 0.0);
    }

    #[test]
    fn test_flood_cell_hazard() {
        let mut cell = FloodCell::new(Vec3::ZERO);
        cell.water_depth = 2.0;
        cell.flow_velocity = Vec3::new(1.0, 0.0, 0.0);
        cell.inundation_time = 1800.0;
        let hazard = cell.hazard_index();
        assert!(hazard > 0.0);
        assert!(hazard <= 1.0);
    }

    #[test]
    fn test_flood_damage_assessment() {
        let mut model = FloodModel::new(5, 5, 10.0, FloodType::Riverine);
        for i in 0..model.cells.len() {
            model.cells[i].water_depth = 1.5;
            model.cells[i].is_inundated = true;
        }
        let damage = FloodDamage::assess(&model, 0.01, 100.0);
        assert!(damage.affected_area > 0.0);
        assert!(damage.structural_damage > 0.0);
        assert!(damage.economic_cost > 0.0);
    }

    #[test]
    fn test_flood_flow_accumulation() {
        let model = FloodModel::new(5, 5, 10.0, FloodType::Pluvial);
        let mut elevations = vec![0.0; 25];
        elevations[model.index(2, 2)] = 10.0;
        let acc = model.flow_accumulation(&elevations);
        assert!(acc.iter().any(|&v| v > 0.0));
    }

    #[test]
    fn test_flood_source_lifetime() {
        let mut source = FloodSource::new(Vec3::ZERO, 100.0, 5.0);
        assert!(source.is_active());
        source.remaining_time = 0.0;
        assert!(!source.is_active());
    }
}
