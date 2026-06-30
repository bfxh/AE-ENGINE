use serde::{Deserialize, Serialize};

use crate::properties::ThermalProperties;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConductionSolver {
    pub ambient_temperature: f32,
    pub time_step: f32,
    pub convergence_threshold: f32,
    pub max_iterations: u32,
}

impl Default for ConductionSolver {
    fn default() -> Self {
        Self {
            ambient_temperature: 293.15,
            time_step: 1.0 / 60.0,
            convergence_threshold: 0.01,
            max_iterations: 100,
        }
    }
}

impl ConductionSolver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn solve_1d(
        &self,
        temp_a: f32,
        temp_b: f32,
        distance: f32,
        cross_section: f32,
        props: &ThermalProperties,
    ) -> f32 {
        if distance < 1e-6 {
            return 0.0;
        }
        let gradient = (temp_b - temp_a) / distance;
        props.thermal_conductivity * cross_section * gradient
    }

    #[allow(clippy::too_many_arguments)]
    pub fn solve_pair(
        &self,
        temp_a: f32,
        temp_b: f32,
        mass_a: f32,
        mass_b: f32,
        distance: f32,
        cross_section: f32,
        props_a: &ThermalProperties,
        props_b: &ThermalProperties,
    ) -> (f32, f32) {
        let heat_flow = self.solve_1d(temp_a, temp_b, distance, cross_section, props_a);
        let heat_flow_clamped = heat_flow.clamp(
            -(temp_a - temp_b).abs() * props_a.heat_capacity(mass_a / props_a.density),
            (temp_a - temp_b).abs() * props_b.heat_capacity(mass_b / props_b.density),
        );

        let delta_a =
            -heat_flow_clamped * self.time_step / props_a.heat_capacity(mass_a / props_a.density);
        let delta_b =
            heat_flow_clamped * self.time_step / props_b.heat_capacity(mass_b / props_b.density);

        (delta_a, delta_b)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn solve_contact(
        &self,
        temp_a: f32,
        temp_b: f32,
        mass_a: f32,
        mass_b: f32,
        contact_area: f32,
        conductivity_a: f32,
        conductivity_b: f32,
        props_a: &ThermalProperties,
        props_b: &ThermalProperties,
    ) -> (f32, f32) {
        let contact_conductance =
            2.0 * conductivity_a * conductivity_b / (conductivity_a + conductivity_b).max(1e-6);

        let heat_flow = contact_conductance * contact_area * (temp_b - temp_a);
        let heat_flow_clamped = heat_flow.clamp(
            -(temp_a - temp_b).abs() * props_a.heat_capacity(mass_a / props_a.density),
            (temp_a - temp_b).abs() * props_b.heat_capacity(mass_b / props_b.density),
        );

        let delta_a =
            heat_flow_clamped * self.time_step / props_a.heat_capacity(mass_a / props_a.density);
        let delta_b =
            -heat_flow_clamped * self.time_step / props_b.heat_capacity(mass_b / props_b.density);

        (delta_a, delta_b)
    }

    pub fn solve_ambient(
        &self,
        temp: f32,
        surface_area: f32,
        mass: f32,
        props: &ThermalProperties,
    ) -> f32 {
        let heat_flow =
            props.thermal_conductivity * surface_area * (self.ambient_temperature - temp);
        let heat_flow_clamped = heat_flow.clamp(
            -(temp - self.ambient_temperature).abs() * props.heat_capacity(mass / props.density),
            (temp - self.ambient_temperature).abs() * props.heat_capacity(mass / props.density),
        );

        heat_flow_clamped * self.time_step / props.heat_capacity(mass / props.density)
    }

    pub fn steady_state_temperature(
        &self,
        hot_temp: f32,
        cold_temp: f32,
        distance: f32,
        x: f32,
    ) -> f32 {
        if distance < 1e-6 {
            return hot_temp;
        }
        let t = (x / distance).clamp(0.0, 1.0);
        hot_temp + (cold_temp - hot_temp) * t
    }

    pub fn thermal_resistance(
        &self,
        length: f32,
        cross_section: f32,
        props: &ThermalProperties,
    ) -> f32 {
        if cross_section < 1e-6 {
            return f32::MAX;
        }
        length / (props.thermal_conductivity * cross_section)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThermalGrid {
    pub temperatures: Vec<f32>,
    pub dimensions: (usize, usize, usize),
    pub cell_size: f32,
    pub solver: ConductionSolver,
}

impl ThermalGrid {
    pub fn new(dimensions: (usize, usize, usize), cell_size: f32, initial_temp: f32) -> Self {
        let (nx, ny, nz) = dimensions;
        let size = nx * ny * nz;
        Self {
            temperatures: vec![initial_temp; size],
            dimensions,
            cell_size,
            solver: ConductionSolver::default(),
        }
    }

    pub fn step(&mut self, props: &ThermalProperties) {
        let (nx, ny, nz) = self.dimensions;
        let dx = self.cell_size;
        let alpha = props.thermal_diffusivity();
        let dt = self.solver.time_step;
        let r = alpha * dt / (dx * dx);

        if r > 0.5 {
            log::warn!("ThermalGrid: stability condition violated, r={} > 0.5", r);
        }

        let mut new_temps = self.temperatures.clone();

        for k in 1..nz - 1 {
            for j in 1..ny - 1 {
                for i in 1..nx - 1 {
                    let idx = self.index(i, j, k);
                    let current = self.temperatures[idx];

                    let laplacian = (self.temperatures[self.index(i + 1, j, k)]
                        + self.temperatures[self.index(i - 1, j, k)]
                        + self.temperatures[self.index(i, j + 1, k)]
                        + self.temperatures[self.index(i, j - 1, k)]
                        + self.temperatures[self.index(i, j, k + 1)]
                        + self.temperatures[self.index(i, j, k - 1)]
                        - 6.0 * current)
                        / (dx * dx);

                    new_temps[idx] = current + alpha * dt * laplacian;
                }
            }
        }

        self.temperatures = new_temps;
    }

    fn index(&self, i: usize, j: usize, k: usize) -> usize {
        let (nx, ny, _) = self.dimensions;
        k * nx * ny + j * nx + i
    }

    pub fn get_temperature(&self, (x, y, z): (usize, usize, usize)) -> f32 {
        self.temperatures[self.index(x, y, z)]
    }

    pub fn set_boundary(&mut self, face: BoundaryFace, temp: f32) {
        let (nx, ny, nz) = self.dimensions;
        match face {
            BoundaryFace::XMin => {
                for k in 0..nz {
                    for j in 0..ny {
                        let idx = self.index(0, j, k);
                        self.temperatures[idx] = temp;
                    }
                }
            },
            BoundaryFace::XMax => {
                for k in 0..nz {
                    for j in 0..ny {
                        let idx = self.index(nx - 1, j, k);
                        self.temperatures[idx] = temp;
                    }
                }
            },
            BoundaryFace::YMin => {
                for k in 0..nz {
                    for i in 0..nx {
                        let idx = self.index(i, 0, k);
                        self.temperatures[idx] = temp;
                    }
                }
            },
            BoundaryFace::YMax => {
                for k in 0..nz {
                    for i in 0..nx {
                        let idx = self.index(i, ny - 1, k);
                        self.temperatures[idx] = temp;
                    }
                }
            },
            BoundaryFace::ZMin => {
                for j in 0..ny {
                    for i in 0..nx {
                        let idx = self.index(i, j, 0);
                        self.temperatures[idx] = temp;
                    }
                }
            },
            BoundaryFace::ZMax => {
                for j in 0..ny {
                    for i in 0..nx {
                        let idx = self.index(i, j, nz - 1);
                        self.temperatures[idx] = temp;
                    }
                }
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryFace {
    XMin,
    XMax,
    YMin,
    YMax,
    ZMin,
    ZMax,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fourier_heat_flow() {
        let solver = ConductionSolver::default();
        let props = crate::properties::THERMAL_COPPER;
        let heat_flow = solver.solve_1d(400.0, 300.0, 0.1, 0.01, &props);
        assert!(heat_flow < 0.0);
    }

    #[test]
    fn test_no_heat_flow_equal_temp() {
        let solver = ConductionSolver::default();
        let props = crate::properties::THERMAL_COPPER;
        let heat_flow = solver.solve_1d(300.0, 300.0, 0.1, 0.01, &props);
        assert!((heat_flow).abs() < 1e-6);
    }

    #[test]
    fn test_pair_energy_conservation() {
        let solver = ConductionSolver::default();
        let props = crate::properties::THERMAL_COPPER;
        let (da, db) = solver.solve_pair(400.0, 300.0, 1.0, 1.0, 0.1, 0.01, &props, &props);
        let energy_a = da * props.heat_capacity(1.0 / props.density);
        let energy_b = db * props.heat_capacity(1.0 / props.density);
        assert!((energy_a + energy_b).abs() < 0.1);
    }

    #[test]
    fn test_thermal_grid_initialization() {
        let grid = ThermalGrid::new((10, 10, 10), 0.1, 300.0);
        assert_eq!(grid.temperatures.len(), 1000);
        assert_eq!(grid.temperatures[0], 300.0);
    }

    #[test]
    fn test_thermal_grid_boundary() {
        let mut grid = ThermalGrid::new((10, 10, 10), 0.1, 300.0);
        grid.set_boundary(BoundaryFace::XMin, 500.0);
        assert_eq!(grid.get_temperature((0, 5, 5)), 500.0);
    }

    #[test]
    fn test_steady_state_temperature() {
        let solver = ConductionSolver::default();
        let t = solver.steady_state_temperature(500.0, 300.0, 1.0, 0.5);
        assert!((t - 400.0).abs() < 1.0);
    }
}
