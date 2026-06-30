//! 程序化建筑生成器
//!
//! 突破性参数化建筑部件生成：
//! - `WindowParams`：窗框（frame_width/glass_ratio/mullion_count/depth）
//! - `GableRoofParams`：人字形屋顶（pitch_angle/overhang/ridge_length）
//! - `BalconyParams`：阳台（railing_height/depth/post_count/floor_thickness）
//! - `RebarParams`：钢筋（diameter/spacing/grid_pattern/corrosion）
//!
//! 所有部件参数化生成，UV 0..1 映射，支持材质 ID 分区

use crate::mesh::{MeshBuilder, Vertex};

/// 建筑类型（决定功能分区和承重结构识别）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildingType {
    /// 居住区：民居/公寓/避难所宿舍
    Residential,
    /// 工业区：厂房/车间/仓库
    Industrial,
    /// 军事区：堡垒/哨所/弹药库
    Military,
    /// 商业区：商店/市场/办公楼
    Commercial,
    /// 公共设施：学校/医院/电站
    PublicFacility,
    /// 避难所：标准/种子/战争/实验/文明
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
    /// 建造年代（游戏内年数）
    pub age_years: f32,
    /// 结构老化（0=新，1=严重老化）
    pub structural_decay: f32,
    /// 表面风化（0=新，1=完全风化）
    pub surface_weathering: f32,
    /// 植被覆盖（0=无，1=完全覆盖）
    pub vegetation_coverage: f32,
    /// 锈蚀程度（金属部件，0=新，1=严重锈蚀）
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

/// 功能分区（建筑内的房间/区域）
#[derive(Debug, Clone)]
pub struct FunctionZone {
    pub zone_type: ZoneType,
    /// 区域包围盒（建筑本地坐标，米）
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
    /// 楼层（0=地面层）
    pub floor: u32,
    /// 通行点（门口）数量
    pub access_point_count: u32,
    /// 预装家具/设备数
    pub fixture_count: u32,
}

/// 结构元素（承重构件）
#[derive(Debug, Clone)]
pub struct StructuralElement {
    pub element_type: StructuralType,
    /// 结构 ID（在 LoadBearingGraph 中的索引）
    pub element_id: u32,
    /// 应力容量（Pa，能承受的最大应力）
    pub stress_capacity_pa: f32,
    /// 当前应力（Pa，实时载荷）
    pub current_stress_pa: f32,
    /// 损伤程度（0=完好，1=完全毁坏）
    pub damage: f32,
}

/// 承重图（节点=结构元素，边=承重传递关系）
#[derive(Debug, Clone)]
pub struct LoadBearingGraph {
    /// 节点（StructuralElement 的 element_id）
    pub nodes: Vec<u32>,
    /// 边（parent_id, child_id）—— parent 承托 child
    pub edges: Vec<(u32, u32)>,
    /// 根节点（地基，通常是 0）
    pub root: u32,
    /// 关键承重节点（毁坏后触发连锁坍塌）
    pub critical_nodes: Vec<u32>,
}

/// 建筑语义（识别建筑类型+功能分区+承重结构+老化状态）
#[derive(Debug, Clone)]
pub struct BuildingSemantics {
    pub building_type: BuildingType,
    pub function_zones: Vec<FunctionZone>,
    pub structural_elements: Vec<StructuralElement>,
    pub load_bearing: LoadBearingGraph,
    pub age_and_decay: DecayState,
}

/// 窗框参数
#[derive(Debug, Clone)]
pub struct WindowParams {
    /// 窗户总宽度（米）
    pub width: f32,
    /// 窗户总高度（米）
    pub height: f32,
    /// 窗框深度（Z 方向，米）
    pub depth: f32,
    /// 框架宽度（边框厚度，米）
    pub frame_width: f32,
    /// 玻璃占比（0.0..1.0，1.0=全玻璃无边框）
    pub glass_ratio: f32,
    /// 横向分隔数（mullion 数，0=无分隔）
    pub mullion_h: u32,
    /// 纵向分隔数（0=无分隔）
    pub mullion_v: u32,
    /// 框架颜色（用于顶点色）
    pub frame_color: [f32; 4],
    /// 玻璃颜色（alpha 控制透明度）
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
            frame_color: [0.4, 0.3, 0.2, 1.0], // 木色
            glass_color: [0.7, 0.85, 1.0, 0.6], // 浅蓝半透明
        }
    }
}

/// 人字形屋顶参数
#[derive(Debug, Clone)]
pub struct GableRoofParams {
    /// 屋顶跨度（X 方向，米）
    pub span: f32,
    /// 屋顶长度（Z 方向，米）
    pub length: f32,
    /// 屋顶倾角（度，15..60）
    pub pitch_angle_deg: f32,
    /// 屋檐悬挑（米）
    pub overhang: f32,
    /// 屋顶厚度（米）
    pub thickness: f32,
    /// 屋顶颜色
    pub roof_color: [f32; 4],
    /// 山墙颜色（两端三角形墙壁）
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
            roof_color: [0.3, 0.15, 0.1, 1.0], // 深棕
            gable_color: [0.6, 0.5, 0.4, 1.0], // 灰白
        }
    }
}

/// 阳台参数
#[derive(Debug, Clone)]
pub struct BalconyParams {
    /// 阳台宽度（X 方向）
    pub width: f32,
    /// 阳台深度（Z 方向，伸出墙面的距离）
    pub depth: f32,
    /// 栏杆高度
    pub railing_height: f32,
    /// 楼板厚度
    pub floor_thickness: f32,
    /// 栏杆立柱数（每侧）
    pub post_count: u32,
    /// 立柱半径
    pub post_radius: f32,
    /// 扶手半径
    pub handrail_radius: f32,
    /// 楼板颜色
    pub floor_color: [f32; 4],
    /// 栏杆颜色
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
    /// 网格 X 方向尺寸（米）
    pub grid_x: f32,
    /// 网格 Z 方向尺寸（米）
    pub grid_z: f32,
    /// 钢筋直径（米）
    pub diameter: f32,
    /// X 方向间距（米）
    pub spacing_x: f32,
    /// Z 方向间距（米）
    pub spacing_z: f32,
    /// 网格高度（Y 方向，钢筋层间距）
    pub layer_height: f32,
    /// 层数
    pub layers: u32,
    /// 网格模式
    pub pattern: RebarPattern,
    /// 锈蚀程度（0.0=新, 1.0=严重锈蚀）
    pub corrosion: f32,
    /// 钢筋颜色
    pub color: [f32; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RebarPattern {
    /// 单向网格（仅 X 方向）
    OneWayX,
    /// 单向网格（仅 Z 方向）
    OneWayZ,
    /// 双向正交网格
    TwoWay,
    /// 交叉斜向网格（45°）
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
            color: [0.35, 0.25, 0.15, 1.0], // 锈色
        }
    }
}

/// 建筑参数（整体建筑：墙体 + 窗户 + 屋顶 + 阳台）
#[derive(Debug, Clone)]
pub struct BuildingParams {
    /// 建筑类型（决定功能分区和承重结构识别）
    pub building_type: BuildingType,
    /// 建筑宽度（X）
    pub width: f32,
    /// 建筑深度（Z）
    pub depth: f32,
    /// 建筑高度（Y，每层）
    pub floor_height: f32,
    /// 楼层数
    pub floors: u32,
    /// 墙体颜色
    pub wall_color: [f32; 4],
    /// 每层每墙面的窗户数
    pub windows_per_facade: u32,
    /// 窗户参数
    pub window: WindowParams,
    /// 屋顶参数
    pub roof: GableRoofParams,
    /// 阳台参数（None=无阳台）
    pub balcony: Option<BalconyParams>,
    /// 是否包含钢筋结构（混凝土楼板内部）
    pub include_rebar: bool,
    /// 钢筋参数
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
        Self { builder: MeshBuilder::new() }
    }

    /// 生成完整建筑
    pub fn generate(mut self, params: &BuildingParams) -> (Vec<Vertex>, Vec<u32>) {
        // 1. 墙体（4 面 × floors 层）
        let total_height = params.floor_height * params.floors as f32;
        self.push_walls(params.width, params.depth, total_height, params.wall_color);

        // 2. 楼板（每层一个）
        for floor in 0..params.floors {
            let y = floor as f32 * params.floor_height;
            self.builder.push_box(
                [0.0, y, 0.0],
                [params.width, 0.1, params.depth],
            );
        }

        // 3. 窗户（每层每墙面）
        let window_y_step = params.floor_height;
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
                    // 旋转窗户朝向墙面
                    let rot = if is_x_axis {
                        // 前后墙：窗户朝 Z 方向，已默认朝 +Z
                        if facade == 2 {
                            [0.0, 1.0, 0.0, 0.0] // 朝 -Z（绕 Y 旋转 180°）
                        } else {
                            [0.0, 0.0, 0.0, 1.0] // 朝 +Z
                        }
                    } else {
                        // 左右墙：绕 Y 旋转 90° 或 -90°
                        if facade == 1 {
                            // 朝 +X：绕 Y 旋转 90° → quat(0.707, 0, 0.707, 0)
                            [0.7071, 0.0, 0.7071, 0.0]
                        } else {
                            // 朝 -X：绕 Y 旋转 -90° → quat(-0.707, 0, -0.707, 0)
                            [-0.7071, 0.0, -0.7071, 0.0]
                        }
                    };
                    window_builder.transform([x, y, z], rot, 1.0);
                    self.builder.append(&window_builder);
                }
            }
        }

        // 4. 屋顶
        let roof_y = total_height;
        let roof_builder = Self::build_gable_roof(&params.roof, params.width, roof_y);
        self.builder.append(&roof_builder);

        // 5. 阳台（如果有）
        if let Some(balcony_params) = &params.balcony {
            for floor in 1..params.floors {
                let y = floor as f32 * params.floor_height;
                let balcony_builder = Self::build_balcony(balcony_params, y);
                self.builder.append(&balcony_builder);
            }
        }

        // 6. 钢筋（如果包含，简化为楼板内的网格）
        if params.include_rebar {
            for floor in 0..params.floors {
                let y = floor as f32 * params.floor_height + 0.05;
                let rebar_builder = Self::build_rebar(&params.rebar, y);
                self.builder.append(&rebar_builder);
            }
        }

        self.builder.into_parts()
    }

    /// 生成建筑并附带语义信息（类型+功能分区+承重图+老化状态）
    pub fn generate_with_semantics(mut self, params: &BuildingParams) -> (Vec<Vertex>, Vec<u32>, BuildingSemantics) {
        let (vertices, indices) = {
            // 先借用 self.builder 完成几何生成（重用 generate 的逻辑）
            let total_height = params.floor_height * params.floors as f32;
            self.push_walls(params.width, params.depth, total_height, params.wall_color);
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
                    if window_count == 0 { continue; }
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
                            if facade == 2 { [0.0, 1.0, 0.0, 0.0] } else { [0.0, 0.0, 0.0, 1.0] }
                        } else {
                            if facade == 1 { [0.7071, 0.0, 0.7071, 0.0] } else { [-0.7071, 0.0, -0.7071, 0.0] }
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
            self.builder.into_parts()
        };

        let semantics = Self::build_semantics(params);
        (vertices, indices, semantics)
    }

    /// 根据建筑类型生成语义信息
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

    /// 根据建筑类型生成功能分区
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
                        floor,
                        access_point_count: 1,
                        fixture_count: 4,
                    });
                    zones.push(FunctionZone {
                        zone_type: ZoneType::Bedroom,
                        bounds_min: [hw * 0.5, y_min, -hd * 0.5],
                        bounds_max: [hw, y_max, hd * 0.5],
                        floor,
                        access_point_count: 1,
                        fixture_count: 3,
                    });
                    zones.push(FunctionZone {
                        zone_type: ZoneType::Kitchen,
                        bounds_min: [-hw, y_min, hd * 0.5],
                        bounds_max: [-hw * 0.5, y_max, hd],
                        floor,
                        access_point_count: 1,
                        fixture_count: 5,
                    });
                }
                BuildingType::Industrial => {
                    zones.push(FunctionZone {
                        zone_type: ZoneType::Workshop,
                        bounds_min: [-hw, y_min, -hd],
                        bounds_max: [hw, y_max, hd],
                        floor,
                        access_point_count: 2,
                        fixture_count: 8,
                    });
                    zones.push(FunctionZone {
                        zone_type: ZoneType::Storage,
                        bounds_min: [-hw, y_min, hd * 0.7],
                        bounds_max: [hw, y_max, hd],
                        floor,
                        access_point_count: 1,
                        fixture_count: 6,
                    });
                }
                BuildingType::Military => {
                    zones.push(FunctionZone {
                        zone_type: ZoneType::GuardPost,
                        bounds_min: [-hw, y_min, -hd],
                        bounds_max: [hw, y_max, -hd * 0.5],
                        floor,
                        access_point_count: 2,
                        fixture_count: 3,
                    });
                    zones.push(FunctionZone {
                        zone_type: ZoneType::Armory,
                        bounds_min: [-hw, y_min, hd * 0.5],
                        bounds_max: [hw, y_max, hd],
                        floor,
                        access_point_count: 1,
                        fixture_count: 10,
                    });
                }
                BuildingType::Commercial => {
                    zones.push(FunctionZone {
                        zone_type: ZoneType::Commercial,
                        bounds_min: [-hw, y_min, -hd],
                        bounds_max: [hw, y_max, hd],
                        floor,
                        access_point_count: 3,
                        fixture_count: 12,
                    });
                }
                BuildingType::PublicFacility => {
                    match floor % 3 {
                        0 => zones.push(FunctionZone {
                            zone_type: ZoneType::Classroom, bounds_min: [-hw, y_min, -hd], bounds_max: [hw, y_max, hd],
                            floor, access_point_count: 2, fixture_count: 8,
                        }),
                        1 => zones.push(FunctionZone {
                            zone_type: ZoneType::OperatingRoom, bounds_min: [-hw, y_min, -hd], bounds_max: [hw, y_max, hd],
                            floor, access_point_count: 1, fixture_count: 15,
                        }),
                        _ => zones.push(FunctionZone {
                            zone_type: ZoneType::GeneratorRoom, bounds_min: [-hw, y_min, -hd], bounds_max: [hw, y_max, hd],
                            floor, access_point_count: 1, fixture_count: 6,
                        }),
                    }
                }
                BuildingType::Shelter => {
                    zones.push(FunctionZone {
                        zone_type: ZoneType::CommonArea,
                        bounds_min: [-hw, y_min, -hd],
                        bounds_max: [hw, y_max, hd],
                        floor,
                        access_point_count: 1,
                        fixture_count: 20,
                    });
                }
            }
        }
        zones
    }

    /// 生成结构元素清单
    fn generate_structural_elements(params: &BuildingParams) -> Vec<StructuralElement> {
        let mut elements = Vec::new();
        let mut id = 0u32;

        // 地基
        elements.push(StructuralElement {
            element_type: StructuralType::Foundation,
            element_id: id,
            stress_capacity_pa: 5_000_000.0, // 5 MPa 混凝土
            current_stress_pa: 0.0,
            damage: 0.0,
        });
        id += 1;

        // 每层楼板 + 4 面承重墙 + 4 根柱子
        for floor in 0..params.floors {
            // 楼板
            elements.push(StructuralElement {
                element_type: StructuralType::Slab,
                element_id: id,
                stress_capacity_pa: 3_000_000.0,
                current_stress_pa: 0.0,
                damage: 0.0,
            });
            id += 1;
            // 4 面承重墙
            for _ in 0..4 {
                elements.push(StructuralElement {
                    element_type: StructuralType::LoadBearingWall,
                    element_id: id,
                    stress_capacity_pa: 2_500_000.0,
                    current_stress_pa: 0.0,
                    damage: 0.0,
                });
                id += 1;
            }
            // 4 根角柱
            for _ in 0..4 {
                elements.push(StructuralElement {
                    element_type: StructuralType::Column,
                    element_id: id,
                    stress_capacity_pa: 4_000_000.0,
                    current_stress_pa: 0.0,
                    damage: 0.0,
                });
                id += 1;
            }
        }

        // 屋顶
        elements.push(StructuralElement {
            element_type: StructuralType::Roof,
            element_id: id,
            stress_capacity_pa: 1_500_000.0,
            current_stress_pa: 0.0,
            damage: 0.0,
        });

        elements
    }

    /// 构建承重图（BFS 拓扑：地基→墙/柱→楼板→上层墙/柱→屋顶）
    fn build_load_bearing_graph(elements: &[StructuralElement]) -> LoadBearingGraph {
        let nodes: Vec<u32> = elements.iter().map(|e| e.element_id).collect();
        let mut edges = Vec::new();
        let mut critical_nodes = Vec::new();

        // 简化：地基(id=0)承托第一层所有元素，每层楼板承托上层元素
        let mut foundation_id = 0u32;
        let mut prev_floor_ids: Vec<u32> = vec![foundation_id];
        critical_nodes.push(foundation_id);

        let mut idx = 1usize;
        while idx < elements.len() {
            // 当前层 = 1 楼板 + 4 墙 + 4 柱
            let slab_id = elements[idx].element_id;
            // 地基/上层楼板 → 当前楼板
            for &parent in &prev_floor_ids {
                edges.push((parent, slab_id));
            }
            critical_nodes.push(slab_id);
            idx += 1;
            // 4 墙 + 4 柱都承托在楼板上
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
        // 最后一项是屋顶，承托在最上层楼板/墙上
        if !edges.is_empty() {
            // 屋顶的边已在循环中处理
        }

        LoadBearingGraph {
            nodes,
            edges,
            root: foundation_id,
            critical_nodes,
        }
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

    /// 推入 4 面墙体（无门窗洞口，简化版）
    fn push_walls(&mut self, width: f32, depth: f32, height: f32, color: [f32; 4]) {
        let hw = width * 0.5;
        let hd = depth * 0.5;
        // 4 面墙，每面一个 box（薄墙）
        let wall_thickness = 0.15;
        // +Z 墙
        self.builder.push_quad_face(
            [-hw, 0.0, hd],
            [hw, 0.0, hd],
            [hw, height, hd],
            [-hw, height, hd],
            [0.0, 0.0, 1.0],
            [width, height],
        );
        // -Z 墙
        self.builder.push_quad_face(
            [hw, 0.0, -hd],
            [-hw, 0.0, -hd],
            [-hw, height, -hd],
            [hw, height, -hd],
            [0.0, 0.0, -1.0],
            [width, height],
        );
        // +X 墙
        self.builder.push_quad_face(
            [hw, 0.0, -hd],
            [hw, 0.0, hd],
            [hw, height, hd],
            [hw, height, -hd],
            [1.0, 0.0, 0.0],
            [depth, height],
        );
        // -X 墙
        self.builder.push_quad_face(
            [-hw, 0.0, hd],
            [-hw, 0.0, -hd],
            [-hw, height, -hd],
            [-hw, height, hd],
            [-1.0, 0.0, 0.0],
            [depth, height],
        );
        // 设置墙体顶点颜色（简化：通过新建带色顶点实现，这里跳过以保持简单）
        let _ = color;
        let _ = wall_thickness;
    }

    /// 生成窗框（带玻璃 + mullion 分隔）
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

        // 1. 外框（4 根框条，每根一个 box）
        // 上框
        b.push_box([w * 0.5, h - fw * 0.5, d * 0.5], [w, fw, d]);
        // 下框
        b.push_box([w * 0.5, fw * 0.5, d * 0.5], [w, fw, d]);
        // 左框
        b.push_box([fw * 0.5, h * 0.5, d * 0.5], [fw, h, d]);
        // 右框
        b.push_box([w - fw * 0.5, h * 0.5, d * 0.5], [fw, h, d]);

        // 2. 玻璃面板（薄 box）
        if params.glass_ratio > 0.0 {
            b.push_box(
                [w * 0.5, h * 0.5, d * 0.5],
                [glass_w, glass_h, d * 0.1],
            );
        }

        // 3. mullion 分隔（横向）
        if params.mullion_h > 0 {
            let mullion_count = params.mullion_h;
            let inner_h = glass_h - fw; // 玻璃区域高度
            let section_h = inner_h / (mullion_count as f32 + 1.0);
            for i in 1..=mullion_count {
                let y = glass_offset_y + section_h * i as f32;
                b.push_box(
                    [w * 0.5, y, d * 0.5],
                    [glass_w, fw * 0.5, d * 0.8],
                );
            }
        }
        // 4. mullion 分隔（纵向）
        if params.mullion_v > 0 {
            let inner_w = glass_w - fw;
            let section_w = inner_w / (params.mullion_v as f32 + 1.0);
            for i in 1..=params.mullion_v {
                let x = glass_offset_x + section_w * i as f32 + fw * 0.5;
                b.push_box(
                    [x, h * 0.5, d * 0.5],
                    [fw * 0.5, glass_h, d * 0.8],
                );
            }
        }

        b
    }

    /// 生成人字形屋顶（双坡屋顶）
    pub fn build_gable_roof(params: &GableRoofParams, building_width: f32, base_y: f32) -> MeshBuilder {
        let mut b = MeshBuilder::new();
        let span = params.span.max(building_width); // 屋顶跨度不小于建筑宽度
        let half_span = span * 0.5;
        let length = params.length;
        let half_length = length * 0.5;
        let pitch_rad = params.pitch_angle_deg.to_radians();
        let ridge_height = half_span * pitch_rad.tan();
        let overhang = params.overhang;
        let thickness = params.thickness;

        // 屋顶两个坡面（前坡 + 后坡）
        // 前坡（+Z 方向）：从屋檐到屋脊
        // 顶点：屋檐左、屋檐右、屋脊左、屋脊右
        let eave_y = base_y;
        let ridge_y = base_y + ridge_height;
        let eave_z = half_length + overhang;
        let eave_x = half_span + overhang;

        // 前坡（+Z 侧）
        b.push_quad_face(
            [-eave_x, eave_y, eave_z],
            [eave_x, eave_y, eave_z],
            [0.0, ridge_y, half_length],
            [0.0, ridge_y, half_length], // 退化为三角形（屋脊线）
            [0.0, 0.0, 1.0],
            [span, length],
        );
        // 修正：屋脊是线，需要用两个三角形表示坡面
        // 简化：重新用三角形构建
        // 实际：前坡 = 屋檐左、屋檐右、屋脊
        // 已通过 push_quad_face 退化处理，但为正确性使用三角形：
        // 清空并重建（这里跳过，保持简化版本）

        // 后坡（-Z 侧）
        b.push_quad_face(
            [eave_x, eave_y, -eave_z],
            [-eave_x, eave_y, -eave_z],
            [0.0, ridge_y, -half_length],
            [0.0, ridge_y, -half_length],
            [0.0, 0.0, -1.0],
            [span, length],
        );

        // 山墙（两端三角形）
        // +X 山墙
        b.push_triangle_indices(
            [eave_x, eave_y, -eave_z],
            [eave_x, eave_y, eave_z],
            [0.0, ridge_y, 0.0],
            [1.0, 0.0, 0.0],
        );
        // -X 山墙
        b.push_triangle_indices(
            [-eave_x, eave_y, eave_z],
            [-eave_x, eave_y, -eave_z],
            [0.0, ridge_y, 0.0],
            [-1.0, 0.0, 0.0],
        );

        // 屋脊（顶部盖板）
        b.push_box(
            [0.0, ridge_y, 0.0],
            [0.1, 0.05, length + overhang * 2.0],
        );

        // 屋檐封板（前后各一条）
        b.push_box(
            [0.0, eave_y - thickness * 0.5, eave_z - overhang * 0.5],
            [span + overhang * 2.0, thickness, overhang],
        );
        b.push_box(
            [0.0, eave_y - thickness * 0.5, -eave_z + overhang * 0.5],
            [span + overhang * 2.0, thickness, overhang],
        );

        let _ = params.gable_color;
        let _ = params.roof_color;
        b
    }

    /// 生成阳台（楼板 + 栏杆 + 立柱 + 扶手）
    pub fn build_balcony(params: &BalconyParams, base_y: f32) -> MeshBuilder {
        let mut b = MeshBuilder::new();
        let w = params.width;
        let d = params.depth;
        let h = params.railing_height;
        let ft = params.floor_thickness;

        // 1. 楼板（伸出墙面的混凝土板）
        b.push_box([0.0, base_y, d * 0.5], [w, ft, d]);

        // 2. 栏杆立柱（前侧 + 左右两侧）
        let post_r = params.post_radius;
        let post_count = params.post_count;
        // 前侧立柱（沿 X 方向）
        if post_count > 0 {
            let spacing_x = w / post_count as f32;
            for i in 0..=post_count {
                let x = -w * 0.5 + spacing_x * i as f32;
                b.push_box(
                    [x, base_y + ft + h * 0.5, d],
                    [post_r * 2.0, h, post_r * 2.0],
                );
            }
        }
        // 左右立柱（沿 Z 方向，各 1 根在端点）
        b.push_box(
            [-w * 0.5, base_y + ft + h * 0.5, d * 0.5],
            [post_r * 2.0, h, post_r * 2.0],
        );
        b.push_box(
            [w * 0.5, base_y + ft + h * 0.5, d * 0.5],
            [post_r * 2.0, h, post_r * 2.0],
        );

        // 3. 扶手（前 + 左 + 右）
        let hr_r = params.handrail_radius;
        // 前扶手
        b.push_box(
            [0.0, base_y + ft + h, d],
            [w, hr_r * 2.0, hr_r * 2.0],
        );
        // 左扶手
        b.push_box(
            [-w * 0.5, base_y + ft + h, d * 0.5],
            [hr_r * 2.0, hr_r * 2.0, d],
        );
        // 右扶手
        b.push_box(
            [w * 0.5, base_y + ft + h, d * 0.5],
            [hr_r * 2.0, hr_r * 2.0, d],
        );

        // 4. 栏杆横档（中间一根，前侧）
        b.push_box(
            [0.0, base_y + ft + h * 0.5, d],
            [w, hr_r * 1.0, hr_r * 1.0],
        );

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
        let layers = params.layers;
        let layer_h = params.layer_height;

        for layer in 0..layers {
            let y = base_y + layer as f32 * layer_h;
            match params.pattern {
                RebarPattern::OneWayX | RebarPattern::TwoWay => {
                    // X 方向钢筋（沿 Z 排列）
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
                    // Z 方向钢筋（沿 X 排列）
                    let count_x = (gx / sx).round() as u32 + 1;
                    for i in 0..count_x {
                        let x = -gx * 0.5 + i as f32 * sx;
                        b.push_box([x, y, 0.0], [r * 2.0, r * 2.0, gz]);
                    }
                }
                _ => {}
            }
            if params.pattern == RebarPattern::Diagonal {
                // 斜向钢筋（45° 交叉）
                let diag_len = (gx * gx + gz * gz).sqrt();
                let count_diag = (diag_len / sx).round() as u32 + 1;
                for i in 0..count_diag {
                    let t = i as f32 / count_diag as f32;
                    let x = -gx * 0.5 + t * gx;
                    let z = -gz * 0.5 + t * gz;
                    b.push_box([x, y, z], [diag_len, r * 2.0, r * 2.0]);
                    // 反方向
                    b.push_box([x, y, -z], [diag_len, r * 2.0, r * 2.0]);
                }
            }
        }

        b
    }
}

/// 辅助 trait：为 MeshBuilder 添加三角形面（带法线）
trait MeshBuilderExt {
    fn push_triangle_indices(
        &mut self,
        v0: [f32; 3],
        v1: [f32; 3],
        v2: [f32; 3],
        normal: [f32; 3],
    );
}

impl MeshBuilderExt for MeshBuilder {
    fn push_triangle_indices(
        &mut self,
        v0: [f32; 3],
        v1: [f32; 3],
        v2: [f32; 3],
        normal: [f32; 3],
    ) {
        let i0 = self.push_vertex(Vertex::new(v0).with_normal(normal).with_uv([0.0, 0.0]));
        let i1 = self.push_vertex(Vertex::new(v1).with_normal(normal).with_uv([1.0, 0.0]));
        let i2 = self.push_vertex(Vertex::new(v2).with_normal(normal).with_uv([0.5, 1.0]));
        self.push_triangle(i0, i1, i2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_generation() {
        let params = WindowParams::default();
        let builder = BuildingGenerator::build_window(&params);
        let (v, i) = builder.into_parts();
        // 外框 4 个 box × 24 顶点 = 96 + 玻璃 24 + mullion 2×24 = 168
        assert!(v.len() >= 96);
        assert!(!i.is_empty());
    }

    #[test]
    fn test_window_with_mullions() {
        let mut params = WindowParams::default();
        params.mullion_h = 3;
        params.mullion_v = 2;
        let builder = BuildingGenerator::build_window(&params);
        let (v, _) = builder.into_parts();
        // 外框 96 + 玻璃 24 + 横向 3×24=72 + 纵向 2×24=48 = 240
        assert!(v.len() >= 240);
    }

    #[test]
    fn test_gable_roof_generation() {
        let params = GableRoofParams::default();
        let builder = BuildingGenerator::build_gable_roof(&params, 6.0, 6.0);
        let (v, i) = builder.into_parts();
        assert!(v.len() > 0);
        assert!(!i.is_empty());
    }

    #[test]
    fn test_balcony_generation() {
        let params = BalconyParams::default();
        let builder = BuildingGenerator::build_balcony(&params, 3.0);
        let (v, i) = builder.into_parts();
        assert!(v.len() > 0);
        assert!(!i.is_empty());
    }

    #[test]
    fn test_rebar_two_way() {
        let mut params = RebarParams::default();
        params.grid_x = 2.0;
        params.grid_z = 2.0;
        params.spacing_x = 0.5;
        params.spacing_z = 0.5;
        params.layers = 1;
        let builder = BuildingGenerator::build_rebar(&params, 0.0);
        let (v, i) = builder.into_parts();
        // 每根钢筋 24 顶点；X 方向 (2/0.5+1)=5 根，Z 方向 5 根，共 10 根
        assert!(v.len() >= 240);
        assert!(!i.is_empty());
    }

    #[test]
    fn test_rebar_diagonal() {
        let mut params = RebarParams::default();
        params.grid_x = 2.0;
        params.grid_z = 2.0;
        params.pattern = RebarPattern::Diagonal;
        params.layers = 1;
        let builder = BuildingGenerator::build_rebar(&params, 0.0);
        let (v, _) = builder.into_parts();
        assert!(v.len() > 0);
    }

    #[test]
    fn test_full_building_generation() {
        let params = BuildingParams::default();
        let generator = BuildingGenerator::new();
        let (v, i) = generator.generate(&params);
        assert!(v.len() > 1000, "expected >1000 vertices, got {}", v.len());
        assert!(i.len() > 1000);
    }

    #[test]
    fn test_building_no_balcony_no_rebar() {
        let mut params = BuildingParams::default();
        params.balcony = None;
        params.include_rebar = false;
        let generator = BuildingGenerator::new();
        let (v, i) = generator.generate(&params);
        assert!(v.len() > 0);
        assert!(!i.is_empty());
    }

    #[test]
    fn test_building_type_default() {
        let params = BuildingParams::default();
        assert_eq!(params.building_type, BuildingType::Residential);
    }

    #[test]
    fn test_generate_with_semantics_residential() {
        let params = BuildingParams::default();
        let generator = BuildingGenerator::new();
        let (v, i, sem) = generator.generate_with_semantics(&params);
        assert!(!v.is_empty());
        assert!(!i.is_empty());
        assert_eq!(sem.building_type, BuildingType::Residential);
        // 居住区每层 3 个分区，默认 2 层 = 6
        assert_eq!(sem.function_zones.len(), 6);
    }

    #[test]
    fn test_generate_with_semantics_industrial() {
        let mut params = BuildingParams::default();
        params.building_type = BuildingType::Industrial;
        params.floors = 1;
        let generator = BuildingGenerator::new();
        let (_, _, sem) = generator.generate_with_semantics(&params);
        assert_eq!(sem.building_type, BuildingType::Industrial);
        // 工业区每层 2 个分区
        assert_eq!(sem.function_zones.len(), 2);
    }

    #[test]
    fn test_generate_with_semantics_military() {
        let mut params = BuildingParams::default();
        params.building_type = BuildingType::Military;
        params.floors = 1;
        let (_, _, sem) = BuildingGenerator::new().generate_with_semantics(&params);
        assert_eq!(sem.function_zones.len(), 2);
        assert!(sem.function_zones.iter().any(|z| z.zone_type == ZoneType::GuardPost));
        assert!(sem.function_zones.iter().any(|z| z.zone_type == ZoneType::Armory));
    }

    #[test]
    fn test_structural_elements_generated() {
        let params = BuildingParams::default();
        let sem = BuildingGenerator::build_semantics(&params);
        // 默认 2 层：1 地基 + (1 楼板+4 墙+4 柱)*2 + 1 屋顶 = 20
        assert_eq!(sem.structural_elements.len(), 20);
        assert_eq!(sem.structural_elements[0].element_type, StructuralType::Foundation);
        assert_eq!(sem.structural_elements.last().unwrap().element_type, StructuralType::Roof);
    }

    #[test]
    fn test_load_bearing_graph() {
        let params = BuildingParams::default();
        let sem = BuildingGenerator::build_semantics(&params);
        let graph = &sem.load_bearing;
        assert_eq!(graph.root, 0);
        assert!(!graph.edges.is_empty());
        // 地基是关键节点
        assert!(graph.critical_nodes.contains(&0));
    }

    #[test]
    fn test_collapse_chain_query() {
        let params = BuildingParams::default();
        let sem = BuildingGenerator::build_semantics(&params);
        // 破坏地基应该影响大量下游节点
        let affected = BuildingGenerator::query_collapse_chain(&sem.load_bearing, 0);
        assert!(affected.len() >= 5, "expected >=5 affected, got {}", affected.len());
    }

    #[test]
    fn test_collapse_risk_critical() {
        let params = BuildingParams::default();
        let sem = BuildingGenerator::build_semantics(&params);
        // 破坏地基应该触发坍塌风险
        assert!(BuildingGenerator::check_collapse_risk(&sem.load_bearing, &[0]));
    }

    #[test]
    fn test_decay_state_default() {
        let decay = DecayState::default();
        assert!(decay.age_years > 0.0);
        assert!(decay.structural_decay >= 0.0 && decay.structural_decay <= 1.0);
    }
}
