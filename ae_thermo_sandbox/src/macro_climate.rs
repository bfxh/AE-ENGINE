//! V8 沙盒：Macro 层城市/区域级气候模型
//!
//! 城市/区域级连续介质气候模型，处理 km 级网格、100s 时间步的大气物理。
//! 上接 Meso 层（房间级 1Hz），下驱动天气系统与城市热岛。
//!
//! 设计要点：
//! - 2D 水平网格（nx × nz），每个 cell 边长典型 1km
//! - 时间步 100s（比 Meso 层 1Hz 慢 100 倍）
//! - 太阳辐射模型（纬度 + 季节 + 时间）
//! - 地表能量平衡（Stefan-Boltzmann + 显热 + 潜热）
//! - 风场（压力梯度驱动 + 摩擦阻尼）
//! - 温度/CO2/污染平流（上风差分，CFL 稳定）
//! - 水体高热容 + 蒸发增湿
//! - 从 Meso 层注入 CO2/污染物

use serde::{Deserialize, Serialize};

// ─── ClimateCell：宏观网格单元 ───────────────────────────────
/// 宏观网格单元（km 级）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClimateCell {
    /// K 地表温度
    pub temperature: f32,
    /// K 空气温度
    pub air_temp: f32,
    /// 0..1 相对湿度
    pub humidity: f32,
    /// Pa
    pub pressure: f32,
    /// kg/m³ CO2 浓度
    pub co2: f32,
    /// kg/m³ 污染物
    pub pollution: f32,
    /// m/s 风速 [x, z]（2D 水平风场）
    pub wind: [f32; 2],
    /// 0..1 地表反照率
    pub albedo: f32,
    /// J/(m³·K) 地表热容
    pub heat_capacity: f32,
    /// 是否水体（海洋/湖泊）
    pub is_water: bool,
    /// m 海拔
    pub elevation: f32,
}

impl ClimateCell {
    /// 地表能量平衡：太阳辐射吸收、红外损失、显热/潜热通量
    fn step_surface_energy(&mut self, solar: f32, dt: f32) {
        // 吸收太阳辐射（考虑反照率）
        let absorbed_solar = solar * (1.0 - self.albedo);

        // 红外辐射损失（Stefan-Boltzmann 简化）
        let emissivity = 0.95;
        let ir_loss = emissivity * 5.67e-8 * self.temperature.powi(4);

        // 显热通量（地表向空气传热）
        let sensible_heat = 10.0 * (self.temperature - self.air_temp);

        // 潜热通量（蒸发冷却）
        let latent_heat = if self.is_water {
            100.0 * self.humidity
        } else {
            20.0 * self.humidity
        };

        // 净能量
        let net_energy = absorbed_solar - ir_loss - sensible_heat - latent_heat;

        // 温度变化
        let d_t = net_energy * dt / self.heat_capacity.max(1.0);
        self.temperature += d_t;

        // 空气温度向地表温度松弛
        self.air_temp += (self.temperature - self.air_temp) * 0.01 * dt;
    }
}

// ─── ClimateGrid：宏观网格 ───────────────────────────────────
/// 宏观气候网格（2D 水平）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClimateGrid {
    /// 所有网格单元
    pub cells: Vec<ClimateCell>,
    /// x 方向 cell 数
    pub nx: usize,
    /// z 方向 cell 数（2D 网格水平）
    pub nz: usize,
    /// m 每个 cell 边长（典型 1000m = 1km）
    pub cell_size: f32,
    /// 当前模拟时间 s
    pub time: f32,
    /// 时间步（典型 100s）
    pub dt: f32,
    /// W/m² 太阳辐射
    pub solar_radiation: f32,
    /// 纬度（影响太阳角度）
    pub latitude: f32,
    /// 1..365（影响季节）
    pub day_of_year: u32,
    /// 0..24 小时
    pub time_of_day: f32,
}

impl ClimateGrid {
    /// 创建默认气候网格（10×10 km 区域）
    pub fn new_default() -> Self {
        Self::new(10, 10, 1000.0, 100.0)
    }

    /// 创建均匀初始场
    pub fn new(nx: usize, nz: usize, cell_size: f32, dt: f32) -> Self {
        let mut cells = Vec::with_capacity(nx * nz);
        for _ in 0..(nx * nz) {
            cells.push(ClimateCell {
                temperature: 288.15, // 15°C
                air_temp: 288.15,
                humidity: 0.5,
                pressure: 101_325.0,
                co2: 0.0006, // 约 400ppm
                pollution: 0.0,
                wind: [0.0, 0.0],
                albedo: 0.3,
                heat_capacity: 1.5e6, // J/(m³·K) 混合地表
                is_water: false,
                elevation: 0.0,
            });
        }
        Self {
            cells,
            nx,
            nz,
            cell_size,
            time: 0.0,
            dt,
            solar_radiation: 0.0,
            latitude: 30.0,
            day_of_year: 180,
            time_of_day: 12.0,
        }
    }

    /// 设置海洋区域
    pub fn set_ocean(&mut self, i: usize, k: usize) {
        if let Some(cell) = self.cells.get_mut(k * self.nx + i) {
            cell.is_water = true;
            cell.albedo = 0.06;
            cell.heat_capacity = 4.0e6; // 水的高热容
            cell.humidity = 0.8;
        }
    }

    /// 设置火源（CO2/污染源）
    pub fn set_pollution_source(&mut self, i: usize, k: usize, intensity: f32) {
        if let Some(cell) = self.cells.get_mut(k * self.nx + i) {
            cell.co2 += intensity * 0.001;
            cell.pollution += intensity * 0.0005;
        }
    }

    /// 推进一步气候模拟
    pub fn step(&mut self) {
        let dt = self.dt;

        // 1. 更新时间
        self.time += dt;
        self.time_of_day += dt / 3600.0;
        if self.time_of_day >= 24.0 {
            self.time_of_day -= 24.0;
            self.day_of_year += 1;
            if self.day_of_year > 365 {
                self.day_of_year = 1;
            }
        }

        // 2. 计算太阳辐射
        let solar = solar_radiation(self.latitude, self.day_of_year, self.time_of_day);
        self.solar_radiation = solar;

        // 3. 地表能量平衡
        for cell in &mut self.cells {
            cell.step_surface_energy(solar, dt);
        }

        // 4. 风场更新
        self.step_wind();

        // 5. 平流扩散
        self.step_advection();

        // 6. 湿度更新（水体蒸发增加湿度）
        for cell in &mut self.cells {
            if cell.is_water {
                cell.humidity = (cell.humidity + 0.001 * dt).min(1.0);
            } else {
                cell.humidity = (cell.humidity - 0.0005 * dt).max(0.0);
            }
        }
    }

    /// 温度/CO2/污染物平流（上风差分，CFL 稳定）
    fn step_advection(&mut self) {
        let dt = self.dt;
        let dx = self.cell_size;

        let mut new_temp = vec![0.0f32; self.cells.len()];
        let mut new_co2 = vec![0.0f32; self.cells.len()];
        let mut new_pollution = vec![0.0f32; self.cells.len()];

        for k in 0..self.nz {
            for i in 0..self.nx {
                let idx = k * self.nx + i;
                let wind = self.cells[idx].wind;

                // 上风差分（简化）
                let source_i = if wind[0] > 0.0 {
                    i.saturating_sub(1)
                } else {
                    (i + 1).min(self.nx - 1)
                };
                let source_k = if wind[1] > 0.0 {
                    k.saturating_sub(1)
                } else {
                    (k + 1).min(self.nz - 1)
                };
                let source_idx = source_k * self.nx + source_i;

                let flux = wind[0].abs() * dt / dx;
                let flux_z = wind[1].abs() * dt / dx;
                let total_flux = (flux + flux_z).min(0.5); // CFL 稳定性

                new_temp[idx] = self.cells[idx].air_temp * (1.0 - total_flux)
                    + self.cells[source_idx].air_temp * total_flux;
                new_co2[idx] = self.cells[idx].co2 * (1.0 - total_flux)
                    + self.cells[source_idx].co2 * total_flux;
                new_pollution[idx] = self.cells[idx].pollution * (1.0 - total_flux)
                    + self.cells[source_idx].pollution * total_flux;
            }
        }

        for i in 0..self.cells.len() {
            self.cells[i].air_temp = new_temp[i];
            self.cells[i].co2 = new_co2[i];
            self.cells[i].pollution = new_pollution[i];
        }
    }

    /// 风场更新（简化：压力梯度驱动 + 摩擦阻尼）
    fn step_wind(&mut self) {
        let dt = self.dt;
        let dx = self.cell_size;

        let mut new_wind = vec![[0.0f32; 2]; self.cells.len()];

        for k in 0..self.nz {
            for i in 0..self.nx {
                let idx = k * self.nx + i;

                // 压力梯度（温度差驱动）
                let mut grad_x = 0.0;
                let mut grad_z = 0.0;

                if i > 0 && i < self.nx - 1 {
                    grad_x =
                        (self.cells[idx + 1].pressure - self.cells[idx - 1].pressure) / (2.0 * dx);
                }
                if k > 0 && k < self.nz - 1 {
                    grad_z = (self.cells[(k + 1) * self.nx + i].pressure
                        - self.cells[(k - 1) * self.nx + i].pressure)
                        / (2.0 * dx);
                }

                // 风加速 = -梯度/密度
                let density = 1.225; // kg/m³ 海平面空气
                let damping = 0.95; // 摩擦阻尼

                new_wind[idx][0] = (self.cells[idx].wind[0] - grad_x / density * dt) * damping;
                new_wind[idx][1] = (self.cells[idx].wind[1] - grad_z / density * dt) * damping;
            }
        }

        for i in 0..self.cells.len() {
            self.cells[i].wind = new_wind[i];
        }

        // 更新压力（理想气体，温度变化驱动）
        for cell in &mut self.cells {
            cell.pressure = 101_325.0 * (cell.air_temp / 288.15).max(0.1);
        }
    }

    /// 从 Meso 层聚合创建气候网格
    /// 将多个 MesoNode 的热量/CO2 注入到气候网格
    pub fn inject_from_meso(
        &mut self,
        meso_nodes: &[crate::meso_layer::MesoNode],
        grid_origin: [f32; 2],
        _grid_size: [f32; 2],
    ) {
        for node in meso_nodes {
            // 计算 Meso 节点在气候网格中的位置
            let gi = ((node.position[0] - grid_origin[0]) / self.cell_size).floor() as isize;
            let gk = ((node.position[2] - grid_origin[1]) / self.cell_size).floor() as isize;

            if gi < 0 || gk < 0 {
                continue;
            }
            let (gi, gk) = (gi as usize, gk as usize);
            if gi >= self.nx || gk >= self.nz {
                continue;
            }

            let idx = gk * self.nx + gi;
            // 注入 CO2 和污染
            self.cells[idx].co2 += node.co2_mass * 0.001; // 简化系数
            self.cells[idx].pollution += node.toxic_gas * 0.001;
        }
    }
}

// ─── 太阳辐射模型 ────────────────────────────────────────────
/// 计算太阳辐射 W/m²
/// 简化模型：考虑纬度、季节、时间
fn solar_radiation(latitude: f32, day_of_year: u32, time_of_day: f32) -> f32 {
    // 太阳赤纬
    let declination = 23.45
        * (2.0 * std::f32::consts::PI * (day_of_year as f32 - 81.0) / 365.0)
            .sin()
            .to_radians();
    let lat_rad = latitude.to_radians();

    // 时角
    let hour_angle = (time_of_day - 12.0) * 15.0 * std::f32::consts::PI / 180.0;

    // 太阳高度角
    let sin_altitude = (lat_rad.sin() * declination.sin()
        + lat_rad.cos() * declination.cos() * hour_angle.cos())
        .max(0.0);

    if sin_altitude <= 0.0 {
        return 0.0; // 夜晚
    }

    // 大气层外辐射 1367 W/m²，考虑大气衰减
    let solar_constant = 1367.0;
    let altitude = sin_altitude.asin();
    let air_mass = 1.0
        / (altitude.sin() + 0.50572 * (altitude + 6.07995).to_radians().powf(-1.6364)).max(0.0);
    let atmospheric_transmission = 0.7_f32.powf(air_mass);

    solar_constant * sin_altitude * atmospheric_transmission
}

// ─── 单元测试 ──────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::materials;
    use crate::meso_layer::MesoNode;

    /// 构造测试用 MesoNode（位于指定位置，co2/toxic_gas 非零）
    fn make_test_meso_node(id: u32, position: [f32; 3]) -> MesoNode {
        let size = [4.0, 4.0, 3.0];
        let volume = size[0] * size[1] * size[2];
        let air_mass = 1.225 * volume;
        MesoNode {
            id,
            position,
            size,
            volume,
            temperature: 300.0,
            pressure: 101_325.0,
            air_mass,
            o2_mass: air_mass * 0.232,
            co2_mass: 1.0,
            h2o_vapor: 0.0,
            toxic_gas: 0.5,
            wall_material: materials::MaterialKind::Brick,
            wall_thickness: 0.2,
            openings_area: 1.0,
            has_fire: false,
            fire_power: 0.0,
            fuel_mass: 0.0,
            npc_count: 0,
            biomass_mass: 0.0,
            connections: Vec::new(),
        }
    }

    #[test]
    fn test_solar_radiation_noon_positive() {
        // 正午太阳辐射应 > 0
        let s = solar_radiation(30.0, 180, 12.0);
        assert!(s > 0.0, "正午太阳辐射应 > 0: {}", s);
    }

    #[test]
    fn test_solar_radiation_night_zero() {
        // 夜晚太阳辐射应为 0
        let s = solar_radiation(30.0, 180, 0.0);
        assert_eq!(s, 0.0, "夜晚太阳辐射应为 0");
    }

    #[test]
    fn test_solar_radiation_seasonal_variation() {
        // 北半球纬度 30°，夏季（day 180）正午太阳辐射应 > 冬季（day 1）正午
        let summer = solar_radiation(30.0, 180, 12.0);
        let winter = solar_radiation(30.0, 1, 12.0);
        assert!(
            summer > winter,
            "夏季太阳辐射应 > 冬季: summer={} winter={}",
            summer,
            winter
        );
    }

    #[test]
    fn test_surface_energy_solar_heating() {
        // 有太阳辐射时地表温度应上升
        let mut cell = ClimateCell {
            temperature: 288.15,
            air_temp: 288.15,
            humidity: 0.5,
            pressure: 101_325.0,
            co2: 0.0006,
            pollution: 0.0,
            wind: [0.0, 0.0],
            albedo: 0.3,
            heat_capacity: 1.5e6,
            is_water: false,
            elevation: 0.0,
        };
        let t0 = cell.temperature;
        cell.step_surface_energy(1000.0, 100.0);
        assert!(
            cell.temperature > t0,
            "有太阳辐射时温度应上升: before={} after={}",
            t0,
            cell.temperature
        );
    }

    #[test]
    fn test_surface_energy_ir_loss_at_night() {
        // 夜晚（无太阳辐射）高温地表应通过红外辐射损失降温
        let mut cell = ClimateCell {
            temperature: 320.0,
            air_temp: 320.0,
            humidity: 0.5,
            pressure: 101_325.0,
            co2: 0.0006,
            pollution: 0.0,
            wind: [0.0, 0.0],
            albedo: 0.3,
            heat_capacity: 1.5e6,
            is_water: false,
            elevation: 0.0,
        };
        let t0 = cell.temperature;
        cell.step_surface_energy(0.0, 100.0);
        assert!(
            cell.temperature < t0,
            "夜晚高温地表应降温: before={} after={}",
            t0,
            cell.temperature
        );
    }

    #[test]
    fn test_grid_creation_default() {
        let grid = ClimateGrid::new_default();
        assert_eq!(grid.nx, 10);
        assert_eq!(grid.nz, 10);
        assert_eq!(grid.cells.len(), 100);
        assert!((grid.cell_size - 1000.0).abs() < 1e-3);
        assert!((grid.dt - 100.0).abs() < 1e-3);
        // 默认初始场
        for cell in &grid.cells {
            assert!((cell.temperature - 288.15).abs() < 1e-3);
            assert!((cell.air_temp - 288.15).abs() < 1e-3);
            assert!((cell.humidity - 0.5).abs() < 1e-3);
            assert!((cell.pressure - 101_325.0).abs() < 1.0);
            assert!((cell.co2 - 0.0006).abs() < 1e-6);
            assert!((cell.albedo - 0.3).abs() < 1e-3);
            assert!((cell.heat_capacity - 1.5e6).abs() < 1.0);
            assert!(!cell.is_water);
        }
    }

    #[test]
    fn test_grid_wind_initialized_zero() {
        // 风场初始化应为 0
        let grid = ClimateGrid::new_default();
        for cell in &grid.cells {
            assert_eq!(cell.wind, [0.0, 0.0]);
        }
    }

    #[test]
    fn test_step_wind_pressure_gradient_creates_wind() {
        // 温差/压差应产生风
        let mut grid = ClimateGrid::new(3, 1, 1000.0, 100.0);
        // 设置不对称温度和压力（cell 0 高温高压，cell 2 低温低压）
        grid.cells[0].air_temp = 300.0;
        grid.cells[0].pressure = 101_325.0 * (300.0_f32 / 288.15).max(0.1);
        grid.cells[1].air_temp = 290.0;
        grid.cells[1].pressure = 101_325.0 * (290.0_f32 / 288.15).max(0.1);
        grid.cells[2].air_temp = 280.0;
        grid.cells[2].pressure = 101_325.0 * (280.0_f32 / 288.15).max(0.1);

        grid.step_wind();
        // 中间 cell 应产生非零风（压力梯度驱动）
        assert!(
            grid.cells[1].wind[0].abs() > 0.0,
            "压力梯度应产生风: wind={:?}",
            grid.cells[1].wind
        );
    }

    #[test]
    fn test_advection_transports_heat() {
        // 风应将热空气吹到冷区域
        let mut grid = ClimateGrid::new(2, 1, 1000.0, 100.0);
        // cell 0 热，cell 1 冷
        grid.cells[0].air_temp = 300.0;
        grid.cells[1].air_temp = 280.0;
        // cell 1 风向 +x（即从 cell 0 吹向 cell 1）
        grid.cells[1].wind = [1.0, 0.0];

        let temp_before = grid.cells[1].air_temp;
        grid.step_advection();
        // cell 1 应从 cell 0 接收热量，温度上升
        assert!(
            grid.cells[1].air_temp > temp_before,
            "风应将热空气吹到冷区域: before={} after={}",
            temp_before,
            grid.cells[1].air_temp
        );
    }

    #[test]
    fn test_advection_transports_co2() {
        // 风应将 CO2 从源吹到下游
        let mut grid = ClimateGrid::new(2, 1, 1000.0, 100.0);
        grid.cells[0].co2 = 0.01; // 高 CO2
        grid.cells[1].co2 = 0.0006; // 背景 CO2
        grid.cells[1].wind = [1.0, 0.0]; // 从 cell 0 吹向 cell 1

        let co2_before = grid.cells[1].co2;
        grid.step_advection();
        assert!(
            grid.cells[1].co2 > co2_before,
            "风应将 CO2 从源吹到下游: before={} after={}",
            co2_before,
            grid.cells[1].co2
        );
    }

    #[test]
    fn test_humidity_increase_over_water() {
        // 水体蒸发应增加湿度
        let mut grid = ClimateGrid::new(1, 1, 1000.0, 100.0);
        grid.set_ocean(0, 0);
        let h_before = grid.cells[0].humidity;
        // 多步推进以使湿度变化可见
        for _ in 0..10 {
            grid.step();
        }
        assert!(
            grid.cells[0].humidity >= h_before,
            "水体应增加或维持湿度: before={} after={}",
            h_before,
            grid.cells[0].humidity
        );
    }

    #[test]
    fn test_water_high_heat_capacity() {
        // 水体温度变化应小于陆地（高热容）
        let mut grid_water = ClimateGrid::new(1, 1, 1000.0, 100.0);
        grid_water.set_ocean(0, 0);

        let mut grid_land = ClimateGrid::new(1, 1, 1000.0, 100.0);
        // 陆地为默认配置

        let t_water_0 = grid_water.cells[0].temperature;
        let t_land_0 = grid_land.cells[0].temperature;

        // 推进多步（白天）
        for _ in 0..100 {
            grid_water.step();
            grid_land.step();
        }

        let d_t_water = (grid_water.cells[0].temperature - t_water_0).abs();
        let d_t_land = (grid_land.cells[0].temperature - t_land_0).abs();
        assert!(
            d_t_water <= d_t_land + 1e-3,
            "水体温度变化应 <= 陆地: water_dT={} land_dT={}",
            d_t_water,
            d_t_land
        );
    }

    #[test]
    fn test_set_ocean_modifies_cell() {
        let mut grid = ClimateGrid::new_default();
        let i = 2;
        let k = 3;
        grid.set_ocean(i, k);
        let cell = &grid.cells[k * grid.nx + i];
        assert!(cell.is_water);
        assert!((cell.albedo - 0.06).abs() < 1e-3);
        assert!((cell.heat_capacity - 4.0e6).abs() < 1.0);
        assert!((cell.humidity - 0.8).abs() < 1e-3);
    }

    #[test]
    fn test_set_pollution_source() {
        let mut grid = ClimateGrid::new_default();
        let i = 5;
        let k = 5;
        let co2_before = grid.cells[k * grid.nx + i].co2;
        let pollution_before = grid.cells[k * grid.nx + i].pollution;
        grid.set_pollution_source(i, k, 100.0);
        assert!(
            grid.cells[k * grid.nx + i].co2 > co2_before,
            "set_pollution_source 应增加 CO2"
        );
        assert!(
            grid.cells[k * grid.nx + i].pollution > pollution_before,
            "set_pollution_source 应增加污染物"
        );
    }

    #[test]
    fn test_inject_from_meso() {
        // Meso 层节点的 CO2/污染物应注入到对应气候 cell
        let mut grid = ClimateGrid::new(10, 10, 1000.0, 100.0);
        let origin = [0.0, 0.0];
        let size = [10000.0, 10000.0];

        // Meso 节点位置 x=1500, z=2500 → cell (i=1, k=2)
        let node = make_test_meso_node(0, [1500.0, 0.0, 2500.0]);

        let target_idx = 2 * 10 + 1;
        let co2_before = grid.cells[target_idx].co2;
        let pollution_before = grid.cells[target_idx].pollution;

        grid.inject_from_meso(&[node], origin, size);

        assert!(
            grid.cells[target_idx].co2 > co2_before,
            "inject_from_meso 应增加目标 cell 的 CO2: before={} after={}",
            co2_before,
            grid.cells[target_idx].co2
        );
        assert!(
            grid.cells[target_idx].pollution > pollution_before,
            "inject_from_meso 应增加目标 cell 的污染物: before={} after={}",
            pollution_before,
            grid.cells[target_idx].pollution
        );
    }

    #[test]
    fn test_inject_from_meso_out_of_bounds_ignored() {
        // 超出网格范围的 Meso 节点应被忽略
        let mut grid = ClimateGrid::new(10, 10, 1000.0, 100.0);
        let origin = [0.0, 0.0];
        let size = [10000.0, 10000.0];

        // 节点位置在网格外（x = 20000）
        let node = make_test_meso_node(0, [20000.0, 0.0, 5000.0]);

        let total_co2_before: f32 = grid.cells.iter().map(|c| c.co2).sum();
        grid.inject_from_meso(&[node], origin, size);
        let total_co2_after: f32 = grid.cells.iter().map(|c| c.co2).sum();

        // 超出范围的节点不应改变网格
        assert!(
            (total_co2_after - total_co2_before).abs() < 1e-6,
            "超出范围的节点应被忽略"
        );
    }

    #[test]
    fn test_day_night_cycle_wrap() {
        // time_of_day 超过 24 应自动归零并增加 day_of_year
        let mut grid = ClimateGrid::new_default();
        grid.time_of_day = 23.99;
        let doy_before = grid.day_of_year;
        grid.step(); // 单步应触发 wrap（23.99 + 100/3600 > 24）
        assert!(
            grid.time_of_day < 24.0,
            "time_of_day 应 < 24: {}",
            grid.time_of_day
        );
        assert!(
            grid.day_of_year == doy_before + 1 || grid.day_of_year == 1,
            "day_of_year 应推进或归 1: before={} after={}",
            doy_before,
            grid.day_of_year
        );
    }

    #[test]
    fn test_step_advances_time() {
        let mut grid = ClimateGrid::new_default();
        let t0 = grid.time;
        grid.step();
        assert!(grid.time > t0, "step 应推进时间");
    }

    #[test]
    fn test_step_no_panic() {
        // 多步推进不应 panic
        let mut grid = ClimateGrid::new_default();
        for _ in 0..100 {
            grid.step();
        }
        assert!(grid.time > 0.0);
    }

    #[test]
    fn test_day_of_year_wrap() {
        // day_of_year 超过 365 应归 1
        let mut grid = ClimateGrid::new_default();
        grid.day_of_year = 365;
        grid.time_of_day = 23.99;
        // 单步即跨日（23.99 + 100/3600 > 24）
        grid.step();
        assert_eq!(grid.day_of_year, 1, "day_of_year 超过 365 应归 1");
    }
}
