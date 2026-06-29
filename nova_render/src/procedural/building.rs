//! 程序化建筑生成器（从 v1 wasteland_render/procedural/building.rs 移植）
//!
//! 突破性参数化建筑部件生成：
//! - `WindowParams`：窗框（frame_width/glass_ratio/mullion_count/depth）
//! - `GableRoofParams`：人字形屋顶（pitch_angle/overhang/ridge_length）
//! - `BalconyParams`：阳台（railing_height/depth/post_count/floor_thickness）
//! - `RebarParams`：钢筋（diameter/spacing/grid_pattern/corrosion）
//!
//! Nova 适配：
//! - 输出 `MeshData`（nova 资源类型）
//! - 通过 `ProceduralGenerator` trait 与 `GeneratorParams` 集成
//! - `MeshBuilder` 来自 `super::`

use crate::assets::MeshData;
use crate::procedural::{GeneratorParams, MeshBuilder, ProceduralGenerator, ProceduralStyle};

// ============================================================================
// 建筑类型 + 语义
// ============================================================================

/// 建筑类型（决定功能分区和承重结构识别）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildingType {
    Residential,
    Industrial,
    Military,
    Commercial,
    PublicFacility,
    Shelter,
}

/// 区域功能类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneType {
    LivingRoom,
    Bedroom,
    Kitchen,
    Workshop,
    Storage,
    GuardPost,
    Armory,
    Commercial,
    Classroom,
    OperatingRoom,
    GeneratorRoom,
    CommonArea,
}

/// 结构元素类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructuralType {
    LoadBearingWall,
    NonLoadBearingWall,
    Column,
    Beam,
    Slab,
    Foundation,
    Roof,
}

/// 老化状态
#[derive(Debug, Clone, Copy)]
pub struct DecayState {
    pub age_years: f32,
    pub structural_decay: f32,
    pub surface_weathering: f32,
    pub vegetation_coverage: f32,
    pub corrosion: f32,
}

impl Default for DecayState {
    fn default() -> Self {
        Self {
            age_years: 50.0,
            structural_decay: 0.3,
            surface_weathering: 0.5,
            vegetation_coverage: 0.2,
            corrosion: 0.4,
        }
    }
}

/// 功能分区
#[derive(Debug, Clone)]
pub struct FunctionZone {
    pub zone_type: ZoneType,
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
    pub floor: u32,
    pub access_point_count: u32,
    pub fixture_count: u32,
}

/// 结构元素
#[derive(Debug, Clone)]
pub struct StructuralElement {
    pub element_type: StructuralType,
    pub element_id: u32,
    pub stress_capacity_pa: f32,
    pub current_stress_pa: f32,
    pub damage: f32,
}

/// 承重图
#[derive(Debug, Clone)]
pub struct LoadBearingGraph {
    pub nodes: Vec<u32>,
    pub edges: Vec<(u32, u32)>,
    pub root: u32,
    pub critical_nodes: Vec<u32>,
}

/// 建筑语义
#[derive(Debug, Clone)]
pub struct BuildingSemantics {
    pub building_type: BuildingType,
    pub function_zones: Vec<FunctionZone>,
    pub structural_elements: Vec<StructuralElement>,
    pub load_bearing: LoadBearingGraph,
    pub age_and_decay: DecayState,
}

// ============================================================================
// 部件参数
// ============================================================================

/// 窗框参数
#[derive(Debug, Clone)]
pub struct WindowParams {
    pub width: f32,
    pub height: f32,
    pub depth: f32,
    pub frame_width: f32,
    pub glass_ratio: f32,
    pub mullion_h: u32,
    pub mullion_v: u32,
    pub frame_color: [f32; 4],
    pub glass_color: [f32; 4],
}

impl Default for WindowParams {
    fn default() -> Self {
        Self {
            width: 1.2,
            height: 1.5,
            depth: 0.1,
            frame_width: 0.05,
            glass_ratio: 0.85,
            mullion_h: 2,
            mullion_v: 1,
            frame_color: [0.4, 0.3, 0.2, 1.0],
            glass_color: [0.7, 0.85, 1.0, 0.6],
        }
    }
}

/// 人字形屋顶参数
#[derive(Debug, Clone)]
pub struct GableRoofParams {
    pub span: f32,
    pub length: f32,
    pub pitch_angle_deg: f32,
    pub overhang: f32,
    pub thickness: f32,
    pub roof_color: [f32; 4],
    pub gable_color: [f32; 4],
}

impl Default for GableRoofParams {
    fn default() -> Self {
        Self {
            span: 6.0,
            length: 8.0,
            pitch_angle_deg: 35.0,
            overhang: 0.5,
            thickness: 0.1,
            roof_color: [0.3, 0.15, 0.1, 1.0],
            gable_color: [0.6, 0.5, 0.4, 1.0],
        }
    }
}

/// 阳台参数
#[derive(Debug, Clone)]
pub struct BalconyParams {
    pub width: f32,
    pub depth: f32,
    pub railing_height: f32,
    pub floor_thickness: f32,
    pub post_count: u32,
    pub post_radius: f32,
    pub handrail_radius: f32,
    pub floor_color: [f32; 4],
    pub railing_color: [f32; 4],
}

impl Default for BalconyParams {
    fn default() -> Self {
        Self {
            width: 3.0,
            depth: 1.2,
            railing_height: 1.1,
            floor_thickness: 0.15,
            post_count: 4,
            post_radius: 0.03,
            handrail_radius: 0.04,
            floor_color: [0.5, 0.4, 0.3, 1.0],
            railing_color: [0.2, 0.2, 0.2, 1.0],
        }
    }
}

/// 钢筋参数
#[derive(Debug, Clone)]
pub struct RebarParams {
    pub grid_x: f32,
    pub grid_z: f32,
    pub diameter: f32,
    pub spacing_x: f32,
    pub spacing_z: f32,
    pub layer_height: f32,
    pub layers: u32,
    pub pattern: RebarPattern,
    pub corrosion: f32,
    pub color: [f32; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RebarPattern {
    OneWayX,
    OneWayZ,
    TwoWay,
    Diagonal,
}

impl Default for RebarParams {
    fn default() -> Self {
        Self {
            grid_x: 4.0,
            grid_z: 4.0,
            diameter: 0.016,
            spacing_x: 0.2,
            spacing_z: 0.2,
            layer_height: 0.3,
            layers: 2,
            pattern: RebarPattern::TwoWay,
            corrosion: 0.1,
            color: [0.35, 0.25, 0.15, 1.0],
        }
    }
}

/// 建筑参数
#[derive(Debug, Clone)]
pub struct BuildingParams {
    pub building_type: BuildingType,
    pub width: f32,
    pub depth: f32,
    pub floor_height: f32,
    pub floors: u32,
    pub wall_color: [f32; 4],
    pub windows_per_facade: u32,
    pub window: WindowParams,
    pub roof: GableRoofParams,
    pub balcony: Option<BalconyParams>,
    pub include_rebar: bool,
    pub rebar: RebarParams,
}

impl Default for BuildingParams {
    fn default() -> Self {
        Self {
            building_type: BuildingType::Residential,
            width: 8.0,
            depth: 6.0,
            floor_height: 3.0,
            floors: 2,
            wall_color: [0.7, 0.65, 0.55, 1.0],
            windows_per_facade: 3,
            window: WindowParams::default(),
            roof: GableRoofParams::default(),
            balcony: Some(BalconyParams::default()),
            include_rebar: true,
            rebar: RebarParams::default(),
        }
    }
}

/// 建筑生成输出（Mesh + 语义数据）
#[derive(Debug, Clone)]
pub struct BuildingOutput {
    pub mesh: MeshData,
    pub semantics: BuildingSemantics,
}

// ============================================================================
// 建筑生成器
// ============================================================================

/// 建筑生成器
pub struct BuildingGenerator {
    builder: MeshBuilder,
}

impl Default for BuildingGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl BuildingGenerator {
    pub fn new() -> Self {
        Self {
            builder: MeshBuilder::new(),
        }
    }

    /// 用专用参数生成完整建筑
    pub fn generate_with_params(mut self, params: &BuildingParams) -> BuildingOutput {
        let total_height = params.floor_height * params.floors as f32;
        self.push_walls(params.width, params.depth, total_height);

        // 楼板
        for floor in 0..params.floors {
            let y = floor as f32 * params.floor_height;
            self.builder.push_box([0.0, y, 0.0], [params.width, 0.1, params.depth]);
        }

        // 窗户
        for floor in 0..params.floors {
            let y = floor as f32 * params.floor_height + params.floor_height * 0.5;
            for facade in 0..4u32 {
                let (facade_width, is_x_axis) = match facade {
                    0 | 2 => (params.width, true),
                    _ => (params.depth, false),
                };
                let window_count = params.windows_per_facade;
                if window_count == 0 {
                    continue;
                }
                let spacing = facade_width / window_count as f32;
                for w in 0..window_count {
                    let offset = -facade_width * 0.5 + spacing * (w as f32 + 0.5);
                    let (x, z) = if is_x_axis {
                        let z_sign = if facade == 0 { 1.0 } else { -1.0 };
                        (offset, params.depth * 0.5 * z_sign)
                    } else {
                        let x_sign = if facade == 1 { 1.0 } else { -1.0 };
                        (params.width * 0.5 * x_sign, offset)
                    };
                    let mut window_builder = Self::build_window(&params.window);
                    let rot = if is_x_axis {
                        if facade == 2 {
                            [0.0, 1.0, 0.0, 0.0]
                        } else {
                            [0.0, 0.0, 0.0, 1.0]
                        }
                    } else if facade == 1 {
                        [0.7071, 0.0, 0.7071, 0.0]
                    } else {
                        [-0.7071, 0.0, -0.7071, 0.0]
                    };
                    window_builder.transform([x, y, z], rot, 1.0);
                    self.builder.append(&window_builder);
                }
            }
        }

        // 屋顶
        let roof_y = total_height;
        let roof_builder = Self::build_gable_roof(&params.roof, params.width, roof_y);
        self.builder.append(&roof_builder);

        // 阳台
        if let Some(balcony_params) = &params.balcony {
            for floor in 1..params.floors {
                let y = floor as f32 * params.floor_height;
                let balcony_builder = Self::build_balcony(balcony_params, y);
                self.builder.append(&balcony_builder);
            }
        }

        // 钢筋
        if params.include_rebar {
            for floor in 0..params.floors {
                let y = floor as f32 * params.floor_height + 0.05;
                let rebar_builder = Self::build_rebar(&params.rebar, y);
                self.builder.append(&rebar_builder);
            }
        }

        let semantics = Self::build_semantics(params);
        BuildingOutput {
            mesh: self.builder.into_mesh_data(),
            semantics,
        }
    }

    /// 生成窗框
    pub fn build_window(params: &WindowParams) -> MeshBuilder {
        let mut b = MeshBuilder::new();
        let w = params.width;
        let h = params.height;
        let d = params.depth;
        let fw = params.frame_width;
        let glass_w = w * params.glass_ratio;
        let glass_h = h * params.glass_ratio;
        let glass_offset_x = (w - glass_w) * 0.5;
        let glass_offset_y = (h - glass_h) * 0.5;

        // 外框 4 根
        b.push_box([w * 0.5, h - fw * 0.5, d * 0.5], [w, fw, d]);
        b.push_box([w * 0.5, fw * 0.5, d * 0.5], [w, fw, d]);
        b.push_box([fw * 0.5, h * 0.5, d * 0.5], [fw, h, d]);
        b.push_box([w - fw * 0.5, h * 0.5, d * 0.5], [fw, h, d]);

        // 玻璃
        if params.glass_ratio > 0.0 {
            b.push_box([w * 0.5, h * 0.5, d * 0.5], [glass_w, glass_h, d * 0.1]);
        }

        // mullion 横向
        if params.mullion_h > 0 {
            let inner_h = glass_h - fw;
            let section_h = inner_h / (params.mullion_h as f32 + 1.0);
            for i in 1..=params.mullion_h {
                let y = glass_offset_y + section_h * i as f32;
                b.push_box([w * 0.5, y, d * 0.5], [glass_w, fw * 0.5, d * 0.8]);
            }
        }
        // mullion 纵向
        if params.mullion_v > 0 {
            let inner_w = glass_w - fw;
            let section_w = inner_w / (params.mullion_v as f32 + 1.0);
            for i in 1..=params.mullion_v {
                let x = glass_offset_x + section_w * i as f32 + fw * 0.5;
                b.push_box([x, h * 0.5, d * 0.5], [fw * 0.5, glass_h, d * 0.8]);
            }
        }
        b
    }

    /// 生成人字形屋顶
    pub fn build_gable_roof(params: &GableRoofParams, building_width: f32, base_y: f32) -> MeshBuilder {
        let mut b = MeshBuilder::new();
        let span = params.span.max(building_width);
        let half_span = span * 0.5;
        let length = params.length;
        let half_length = length * 0.5;
        let pitch_rad = params.pitch_angle_deg.to_radians();
        let ridge_height = half_span * pitch_rad.tan();
        let overhang = params.overhang;
        let thickness = params.thickness;

        let eave_y = base_y;
        let ridge_y = base_y + ridge_height;
        let eave_z = half_length + overhang;
        let eave_x = half_span + overhang;

        // 前后坡面（退化四边形：屋脊为线）
        b.push_quad_face(
            [-eave_x, eave_y, eave_z],
            [eave_x, eave_y, eave_z],
            [0.0, ridge_y, half_length],
            [0.0, ridge_y, half_length],
            [0.0, 0.0, 1.0],
            [span, length],
        );
        b.push_quad_face(
            [eave_x, eave_y, -eave_z],
            [-eave_x, eave_y, -eave_z],
            [0.0, ridge_y, -half_length],
            [0.0, ridge_y, -half_length],
            [0.0, 0.0, -1.0],
            [span, length],
        );

        // 山墙（两端三角形）
        b.push_triangle_face(
            [eave_x, eave_y, -eave_z],
            [eave_x, eave_y, eave_z],
            [0.0, ridge_y, 0.0],
            [1.0, 0.0, 0.0],
        );
        b.push_triangle_face(
            [-eave_x, eave_y, eave_z],
            [-eave_x, eave_y, -eave_z],
            [0.0, ridge_y, 0.0],
            [-1.0, 0.0, 0.0],
        );

        // 屋脊
        b.push_box(
            [0.0, ridge_y, 0.0],
            [0.1, 0.05, length + overhang * 2.0],
        );

        // 屋檐封板
        b.push_box(
            [0.0, eave_y - thickness * 0.5, eave_z - overhang * 0.5],
            [span + overhang * 2.0, thickness, overhang],
        );
        b.push_box(
            [0.0, eave_y - thickness * 0.5, -eave_z + overhang * 0.5],
            [span + overhang * 2.0, thickness, overhang],
        );

        let _ = (params.gable_color, params.roof_color);
        b
    }

    /// 生成阳台
    pub fn build_balcony(params: &BalconyParams, base_y: f32) -> MeshBuilder {
        let mut b = MeshBuilder::new();
        let w = params.width;
        let d = params.depth;
        let h = params.railing_height;
        let ft = params.floor_thickness;

        // 楼板
        b.push_box([0.0, base_y, d * 0.5], [w, ft, d]);

        // 立柱（前侧）
        let post_r = params.post_radius;
        let post_count = params.post_count;
        if post_count > 0 {
            let spacing_x = w / post_count as f32;
            for i in 0..=post_count {
                let x = -w * 0.5 + spacing_x * i as f32;
                b.push_box([x, base_y + ft + h * 0.5, d], [post_r * 2.0, h, post_r * 2.0]);
            }
        }
        // 左右立柱
        b.push_box([-w * 0.5, base_y + ft + h * 0.5, d * 0.5], [post_r * 2.0, h, post_r * 2.0]);
        b.push_box([w * 0.5, base_y + ft + h * 0.5, d * 0.5], [post_r * 2.0, h, post_r * 2.0]);

        // 扶手
        let hr_r = params.handrail_radius;
        b.push_box([0.0, base_y + ft + h, d], [w, hr_r * 2.0, hr_r * 2.0]);
        b.push_box([-w * 0.5, base_y + ft + h, d * 0.5], [hr_r * 2.0, hr_r * 2.0, d]);
        b.push_box([w * 0.5, base_y + ft + h, d * 0.5], [hr_r * 2.0, hr_r * 2.0, d]);

        // 栏杆横档
        b.push_box([0.0, base_y + ft + h * 0.5, d], [w, hr_r, hr_r]);

        b
    }

    /// 生成钢筋网格
    pub fn build_rebar(params: &RebarParams, base_y: f32) -> MeshBuilder {
        let mut b = MeshBuilder::new();
        let r = params.diameter * 0.5;
        let gx = params.grid_x;
        let gz = params.grid_z;
        let sx = params.spacing_x;
        let sz = params.spacing_z;

        for layer in 0..params.layers {
            let y = base_y + layer as f32 * params.layer_height;
            match params.pattern {
                RebarPattern::OneWayX | RebarPattern::TwoWay => {
                    let count_z = (gz / sz).round() as u32 + 1;
                    for i in 0..count_z {
                        let z = -gz * 0.5 + i as f32 * sz;
                        b.push_box([0.0, y, z], [gx, r * 2.0, r * 2.0]);
                    }
                }
                _ => {}
            }
            match params.pattern {
                RebarPattern::OneWayZ | RebarPattern::TwoWay => {
                    let count_x = (gx / sx).round() as u32 + 1;
                    for i in 0..count_x {
                        let x = -gx * 0.5 + i as f32 * sx;
                        b.push_box([x, y, 0.0], [r * 2.0, r * 2.0, gz]);
                    }
                }
                _ => {}
            }
            if params.pattern == RebarPattern::Diagonal {
                let diag_len = (gx * gx + gz * gz).sqrt();
                let count_diag = (diag_len / sx).round() as u32 + 1;
                for i in 0..count_diag {
                    let t = i as f32 / count_diag as f32;
                    let x = -gx * 0.5 + t * gx;
                    let z = -gz * 0.5 + t * gz;
                    b.push_box([x, y, z], [diag_len, r * 2.0, r * 2.0]);
                    b.push_box([x, y, -z], [diag_len, r * 2.0, r * 2.0]);
                }
            }
        }
        b
    }

    fn push_walls(&mut self, width: f32, depth: f32, height: f32) {
        let hw = width * 0.5;
        let hd = depth * 0.5;
        // 4 面墙（quad_face）
        self.builder.push_quad_face(
            [-hw, 0.0, hd],
            [hw, 0.0, hd],
            [hw, height, hd],
            [-hw, height, hd],
            [0.0, 0.0, 1.0],
            [width, height],
        );
        self.builder.push_quad_face(
            [hw, 0.0, -hd],
            [-hw, 0.0, -hd],
            [-hw, height, -hd],
            [hw, height, -hd],
            [0.0, 0.0, -1.0],
            [width, height],
        );
        self.builder.push_quad_face(
            [hw, 0.0, -hd],
            [hw, 0.0, hd],
            [hw, height, hd],
            [hw, height, -hd],
            [1.0, 0.0, 0.0],
            [depth, height],
        );
        self.builder.push_quad_face(
            [-hw, 0.0, hd],
            [-hw, 0.0, -hd],
            [-hw, height, -hd],
            [-hw, height, hd],
            [-1.0, 0.0, 0.0],
            [depth, height],
        );
    }

    fn build_semantics(params: &BuildingParams) -> BuildingSemantics {
        let function_zones = Self::generate_function_zones(params);
        let structural_elements = Self::generate_structural_elements(params);
        let load_bearing = Self::build_load_bearing_graph(&structural_elements);
        BuildingSemantics {
            building_type: params.building_type,
            function_zones,
            structural_elements,
            load_bearing,
            age_and_decay: DecayState::default(),
        }
    }

    fn generate_function_zones(params: &BuildingParams) -> Vec<FunctionZone> {
        let mut zones = Vec::new();
        let hw = params.width * 0.5;
        let hd = params.depth * 0.5;
        let fh = params.floor_height;

        for floor in 0..params.floors {
            let y_min = floor as f32 * fh;
            let y_max = y_min + fh;
            match params.building_type {
                BuildingType::Residential => {
                    zones.push(FunctionZone {
                        zone_type: ZoneType::LivingRoom,
                        bounds_min: [-hw * 0.5, y_min, -hd * 0.5],
                        bounds_max: [hw * 0.5, y_max, hd * 0.5],
                        floor, access_point_count: 1, fixture_count: 4,
                    });
                    zones.push(FunctionZone {
                        zone_type: ZoneType::Bedroom,
                        bounds_min: [hw * 0.5, y_min, -hd * 0.5],
                        bounds_max: [hw, y_max, hd * 0.5],
                        floor, access_point_count: 1, fixture_count: 3,
                    });
                    zones.push(FunctionZone {
                        zone_type: ZoneType::Kitchen,
                        bounds_min: [-hw, y_min, hd * 0.5],
                        bounds_max: [-hw * 0.5, y_max, hd],
                        floor, access_point_count: 1, fixture_count: 5,
                    });
                }
                BuildingType::Industrial => {
                    zones.push(FunctionZone {
                        zone_type: ZoneType::Workshop,
                        bounds_min: [-hw, y_min, -hd], bounds_max: [hw, y_max, hd],
                        floor, access_point_count: 2, fixture_count: 8,
                    });
                    zones.push(FunctionZone {
                        zone_type: ZoneType::Storage,
                        bounds_min: [-hw, y_min, hd * 0.7], bounds_max: [hw, y_max, hd],
                        floor, access_point_count: 1, fixture_count: 6,
                    });
                }
                BuildingType::Military => {
                    zones.push(FunctionZone {
                        zone_type: ZoneType::GuardPost,
                        bounds_min: [-hw, y_min, -hd], bounds_max: [hw, y_max, -hd * 0.5],
                        floor, access_point_count: 2, fixture_count: 3,
                    });
                    zones.push(FunctionZone {
                        zone_type: ZoneType::Armory,
                        bounds_min: [-hw, y_min, hd * 0.5], bounds_max: [hw, y_max, hd],
                        floor, access_point_count: 1, fixture_count: 10,
                    });
                }
                BuildingType::Commercial => {
                    zones.push(FunctionZone {
                        zone_type: ZoneType::Commercial,
                        bounds_min: [-hw, y_min, -hd], bounds_max: [hw, y_max, hd],
                        floor, access_point_count: 3, fixture_count: 12,
                    });
                }
                BuildingType::PublicFacility => match floor % 3 {
                    0 => zones.push(FunctionZone {
                        zone_type: ZoneType::Classroom,
                        bounds_min: [-hw, y_min, -hd], bounds_max: [hw, y_max, hd],
                        floor, access_point_count: 2, fixture_count: 8,
                    }),
                    1 => zones.push(FunctionZone {
                        zone_type: ZoneType::OperatingRoom,
                        bounds_min: [-hw, y_min, -hd], bounds_max: [hw, y_max, hd],
                        floor, access_point_count: 1, fixture_count: 15,
                    }),
                    _ => zones.push(FunctionZone {
                        zone_type: ZoneType::GeneratorRoom,
                        bounds_min: [-hw, y_min, -hd], bounds_max: [hw, y_max, hd],
                        floor, access_point_count: 1, fixture_count: 6,
                    }),
                },
                BuildingType::Shelter => {
                    zones.push(FunctionZone {
                        zone_type: ZoneType::CommonArea,
                        bounds_min: [-hw, y_min, -hd], bounds_max: [hw, y_max, hd],
                        floor, access_point_count: 1, fixture_count: 20,
                    });
                }
            }
        }
        zones
    }

    fn generate_structural_elements(params: &BuildingParams) -> Vec<StructuralElement> {
        let mut elements = Vec::new();
        let mut id = 0u32;

        elements.push(StructuralElement {
            element_type: StructuralType::Foundation,
            element_id: id, stress_capacity_pa: 5_000_000.0,
            current_stress_pa: 0.0, damage: 0.0,
        });
        id += 1;

        for _ in 0..params.floors {
            elements.push(StructuralElement {
                element_type: StructuralType::Slab, element_id: id,
                stress_capacity_pa: 3_000_000.0, current_stress_pa: 0.0, damage: 0.0,
            });
            id += 1;
            for _ in 0..4 {
                elements.push(StructuralElement {
                    element_type: StructuralType::LoadBearingWall, element_id: id,
                    stress_capacity_pa: 2_500_000.0, current_stress_pa: 0.0, damage: 0.0,
                });
                id += 1;
            }
            for _ in 0..4 {
                elements.push(StructuralElement {
                    element_type: StructuralType::Column, element_id: id,
                    stress_capacity_pa: 4_000_000.0, current_stress_pa: 0.0, damage: 0.0,
                });
                id += 1;
            }
        }
        elements.push(StructuralElement {
            element_type: StructuralType::Roof, element_id: id,
            stress_capacity_pa: 1_500_000.0, current_stress_pa: 0.0, damage: 0.0,
        });
        elements
    }

    fn build_load_bearing_graph(elements: &[StructuralElement]) -> LoadBearingGraph {
        let nodes: Vec<u32> = elements.iter().map(|e| e.element_id).collect();
        let mut edges = Vec::new();
        let mut critical_nodes = Vec::new();

        let foundation_id = 0u32;
        let mut prev_floor_ids: Vec<u32> = vec![foundation_id];
        critical_nodes.push(foundation_id);

        let mut idx = 1usize;
        while idx < elements.len() {
            let slab_id = elements[idx].element_id;
            for &parent in &prev_floor_ids {
                edges.push((parent, slab_id));
            }
            critical_nodes.push(slab_id);
            idx += 1;
            let mut current_floor_ids = vec![slab_id];
            for _ in 0..8 {
                if idx < elements.len() {
                    let child_id = elements[idx].element_id;
                    edges.push((slab_id, child_id));
                    current_floor_ids.push(child_id);
                    idx += 1;
                }
            }
            prev_floor_ids = current_floor_ids;
        }

        LoadBearingGraph { nodes, edges, root: foundation_id, critical_nodes }
    }

    /// 查询承重路径：给定一个节点被破坏，返回所有受影响的下游节点
    pub fn query_collapse_chain(graph: &LoadBearingGraph, broken_node: u32) -> Vec<u32> {
        let mut affected = Vec::new();
        let mut frontier = vec![broken_node];
        let mut visited = std::collections::HashSet::new();
        visited.insert(broken_node);
        while let Some(node) = frontier.pop() {
            for &(parent, child) in &graph.edges {
                if parent == node && !visited.contains(&child) {
                    affected.push(child);
                    visited.insert(child);
                    frontier.push(child);
                }
            }
        }
        affected
    }

    /// 检查建筑是否会发生连锁坍塌
    pub fn check_collapse_risk(graph: &LoadBearingGraph, broken_nodes: &[u32]) -> bool {
        for &broken in broken_nodes {
            if graph.critical_nodes.contains(&broken) {
                return true;
            }
        }
        false
    }
}

// ============================================================================
// ProceduralGenerator trait 实现
// ============================================================================

impl ProceduralGenerator for BuildingGenerator {
    type Output = BuildingOutput;

    fn generate(&self, params: &GeneratorParams) -> Self::Output {
        let mut bp = BuildingParams::default();
        // 根据 style 调整默认参数
        match params.style {
            ProceduralStyle::OldWorldRuins => {
                bp.building_type = BuildingType::PublicFacility;
                bp.roof.pitch_angle_deg = 45.0;
            }
            ProceduralStyle::WastelandBuilding | _ => {
                bp.building_type = BuildingType::Residential;
            }
        }
        // seed 决定楼层数（1..5）
        let floors = (params.seed % 4) as u32 + 1;
        bp.floors = floors;
        // lod 影响窗户密度
        if params.lod >= 2 {
            bp.windows_per_facade = 1;
        }
        // clone self（生成器按值消费 builder）
        Self::new().generate_with_params(&bp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_generation() {
        let params = WindowParams::default();
        let b = BuildingGenerator::build_window(&params);
        let md = b.into_mesh_data();
        assert!(md.vertices.len() >= 96);
        assert!(!md.indices.is_empty());
    }

    #[test]
    fn test_gable_roof_generation() {
        let params = GableRoofParams::default();
        let b = BuildingGenerator::build_gable_roof(&params, 6.0, 6.0);
        let md = b.into_mesh_data();
        assert!(!md.vertices.is_empty());
        assert!(!md.indices.is_empty());
    }

    #[test]
    fn test_full_building_generation() {
        let params = BuildingParams::default();
        let out = BuildingGenerator::new().generate_with_params(&params);
        assert!(out.mesh.vertices.len() > 500);
        assert!(!out.mesh.indices.is_empty());
        assert_eq!(out.semantics.building_type, BuildingType::Residential);
    }

    #[test]
    fn test_structural_elements_generated() {
        let params = BuildingParams::default();
        let sem = BuildingGenerator::build_semantics(&params);
        // 默认 2 层：1 地基 + (1 楼板+4 墙+4 柱)*2 + 1 屋顶 = 20
        assert_eq!(sem.structural_elements.len(), 20);
    }

    #[test]
    fn test_collapse_chain_query() {
        let params = BuildingParams::default();
        let sem = BuildingGenerator::build_semantics(&params);
        let affected = BuildingGenerator::query_collapse_chain(&sem.load_bearing, 0);
        assert!(affected.len() >= 5);
    }

    #[test]
    fn test_procedural_generator_trait() {
        let gen = BuildingGenerator::new();
        let gp = GeneratorParams::default();
        let out = gen.generate(&gp);
        assert!(!out.mesh.vertices.is_empty());
    }
}
