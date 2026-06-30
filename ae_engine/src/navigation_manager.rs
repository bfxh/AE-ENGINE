use glam::Vec3;
use serde::{Deserialize, Serialize};
use ae_pathfinding::navmesh::NavMesh;
use ae_pathfinding::astar::AStarPathfinder;
use ae_pathfinding::flowfield::FlowField;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigationConfig {
    pub nav_mesh_resolution: f32,
    pub max_path_queries: usize,
    pub orca_neighbor_dist: f32,
    pub orca_time_horizon: f32,
    pub orca_agent_radius: f32,
    pub max_agents: usize,
}

impl Default for NavigationConfig {
    fn default() -> Self {
        NavigationConfig {
            nav_mesh_resolution: 0.5,
            max_path_queries: 500,
            orca_neighbor_dist: 10.0,
            orca_time_horizon: 3.0,
            orca_agent_radius: 0.5,
            max_agents: 300,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathRequest {
    pub agent_id: u64,
    pub start: Vec3,
    pub goal: Vec3,
    pub timestamp: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathResult {
    pub agent_id: u64,
    pub waypoints: Vec<Vec3>,
    pub status: PathStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PathStatus {
    Pending,
    Found,
    Partial,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrcaAgent {
    pub id: u64,
    pub position: Vec3,
    pub velocity: Vec3,
    pub preferred_velocity: Vec3,
    pub radius: f32,
    pub max_speed: f32,
    /// 当前寻路目标 waypoint 索引（None 表示无路径跟随）
    pub path_index: Option<usize>,
}

pub struct NavigationManager {
    pub config: NavigationConfig,
    pub path_requests: Vec<PathRequest>,
    pub path_results: Vec<PathResult>,
    pub orca_agents: Vec<OrcaAgent>,
    pub nav_mesh_initialized: bool,
    /// NavMesh 实例（A* 寻路的图结构）
    pub nav_mesh: NavMesh,
    /// A* 寻路器
    pub astar: AStarPathfinder,
    /// 流场（可选，构建后用于 ORCA 智能体的 preferred_velocity 注入）
    pub flow_field: Option<FlowField>,
    /// agent_id -> waypoints 映射（A* 寻路结果，ORCA 按 waypoint 跟随）
    pub agent_paths: std::collections::HashMap<u64, Vec<Vec3>>,
}

impl NavigationManager {
    pub fn new(config: NavigationConfig) -> Self {
        let max_agents = config.max_agents;
        NavigationManager {
            config,
            path_requests: Vec::new(),
            path_results: Vec::new(),
            orca_agents: Vec::with_capacity(max_agents),
            nav_mesh_initialized: false,
            nav_mesh: NavMesh::default(),
            astar: AStarPathfinder::default(),
            flow_field: None,
            agent_paths: std::collections::HashMap::new(),
        }
    }

    pub fn request_path(&mut self, agent_id: u64, start: Vec3, goal: Vec3) {
        if self.path_requests.len() >= self.config.max_path_queries {
            self.path_requests.remove(0);
        }
        self.path_requests.push(PathRequest { agent_id, start, goal, timestamp: 0.0 });
    }

    pub fn register_agent(&mut self, id: u64, position: Vec3, max_speed: f32) {
        self.orca_agents.push(OrcaAgent {
            id,
            position,
            velocity: Vec3::ZERO,
            preferred_velocity: Vec3::ZERO,
            radius: self.config.orca_agent_radius,
            max_speed,
            path_index: None,
        });
    }

    pub fn unregister_agent(&mut self, id: u64) {
        self.orca_agents.retain(|a| a.id != id);
        self.agent_paths.remove(&id);
    }

    pub fn update_agent(&mut self, id: u64, position: Vec3, preferred_velocity: Vec3) {
        if let Some(agent) = self.orca_agents.iter_mut().find(|a| a.id == id) {
            agent.position = position;
            agent.preferred_velocity = preferred_velocity;
        }
    }

    /// 消费 path_requests 队列，用 A* 在 NavMesh 上寻路。
    /// 结果写入 path_results 和 agent_paths（供 step_orca 跟随）。
    fn process_path_requests(&mut self) {
        let requests = std::mem::take(&mut self.path_requests);
        for req in requests {
            // NavMesh 为空时直接失败（避免 find_nearest_poly 永远返回 None 后 find_path 死循环）
            if self.nav_mesh.poly_count() == 0 {
                self.path_results.push(PathResult {
                    agent_id: req.agent_id,
                    waypoints: Vec::new(),
                    status: PathStatus::Failed,
                });
                continue;
            }
            let start_arr: [f32; 3] = req.start.into();
            let goal_arr: [f32; 3] = req.goal.into();
            let result = self.astar.find_path(&self.nav_mesh, &start_arr, &goal_arr);
            let waypoints: Vec<Vec3> = result
                .path
                .into_iter()
                .map(|p| Vec3::new(p[0], p[1], p[2]))
                .collect();
            let status = if result.success {
                PathStatus::Found
            } else if !waypoints.is_empty() {
                PathStatus::Partial
            } else {
                PathStatus::Failed
            };
            // 存入 agent_paths 供 ORCA 跟随（非空路径才存）
            if !waypoints.is_empty() {
                self.agent_paths.insert(req.agent_id, waypoints.clone());
                // 重置路径跟随索引
                if let Some(agent) = self.orca_agents.iter_mut().find(|a| a.id == req.agent_id) {
                    agent.path_index = Some(0);
                }
            }
            self.path_results.push(PathResult {
                agent_id: req.agent_id,
                waypoints,
                status,
            });
        }
        // 限制 path_results 长度，避免无限增长
        let max_results = self.config.max_path_queries;
        if self.path_results.len() > max_results {
            let drop_count = self.path_results.len() - max_results;
            self.path_results.drain(0..drop_count);
        }
    }

    /// 从 waypoint 路径或 FlowField 计算智能体的 preferred_velocity。
    /// 优先级：waypoint 路径 > FlowField > 原有 preferred_velocity
    fn compute_preferred_velocity(&self, agent: &OrcaAgent) -> Vec3 {
        // 1. 优先跟随 A* waypoint 路径
        if let Some(waypoints) = self.agent_paths.get(&agent.id) {
            if let Some(idx) = agent.path_index {
                if idx < waypoints.len() {
                    let target = waypoints[idx];
                    let to_target = target - agent.position;
                    let dist = to_target.length();
                    // 到达当前 waypoint（距离 < 1m）则前进到下一个
                    if dist < 1.0 {
                        return Vec3::ZERO; // 下一帧 path_index 会被 step_orca 推进
                    }
                    return (to_target / dist) * agent.max_speed;
                }
            }
        }
        // 2. 回退到 FlowField（如果存在）
        if let Some(ff) = &self.flow_field {
            let pos_arr: [f32; 3] = agent.position.into();
            let dir = ff.get_direction(&pos_arr);
            return Vec3::new(dir[0], 0.0, dir[1]) * agent.max_speed;
        }
        // 3. 最后回退到外部注入的 preferred_velocity
        agent.preferred_velocity
    }

    /// 推进 waypoint 跟随索引（到达当前 waypoint 时前进到下一个）。
    fn advance_path_index(&mut self) {
        // 先收集需要推进/完成的 (agent_id, new_index) 对，避免在借用 orca_agents 时修改 agent_paths
        let mut updates: Vec<(u64, Option<usize>)> = Vec::new();
        for agent in &self.orca_agents {
            if agent.path_index.is_none() {
                continue;
            }
            if let Some(waypoints) = self.agent_paths.get(&agent.id) {
                if let Some(idx) = agent.path_index {
                    if idx >= waypoints.len() {
                        updates.push((agent.id, None));
                        continue;
                    }
                    let target = waypoints[idx];
                    let dist = (target - agent.position).length();
                    if dist < 1.0 {
                        let next_idx = idx + 1;
                        if next_idx >= waypoints.len() {
                            updates.push((agent.id, None)); // 路径完成
                        } else {
                            updates.push((agent.id, Some(next_idx)));
                        }
                    }
                }
            }
        }
        // 应用 path_index 更新
        for (agent_id, new_idx) in &updates {
            if let Some(agent) = self.orca_agents.iter_mut().find(|a| &a.id == agent_id) {
                agent.path_index = *new_idx;
            }
            if new_idx.is_none() {
                self.agent_paths.remove(agent_id);
            }
        }
    }

    pub fn step_orca(&mut self, dt: f32) {
        // 1. 消费积压的寻路请求（A* on NavMesh）
        self.process_path_requests();

        // 2. 推进 waypoint 跟随索引（基于上一帧位置）
        self.advance_path_index();

        let neighbor_dist = self.config.orca_neighbor_dist;
        let time_horizon = self.config.orca_time_horizon;

        // 3. 计算每个智能体的 preferred_velocity（waypoint > FlowField > 外部注入）
        let mut preferred_velocities: Vec<Vec3> = Vec::with_capacity(self.orca_agents.len());
        for agent in &self.orca_agents {
            preferred_velocities.push(self.compute_preferred_velocity(agent));
        }

        // 4. ORCA 避障计算（原逻辑保留，但 new_velocity 初始值改为 computed preferred_velocity）
        let agents_clone = self.orca_agents.clone();

        for (i, agent) in self.orca_agents.iter_mut().enumerate() {
            let mut new_velocity = preferred_velocities[i];

            for other in &agents_clone {
                if other.id == agent.id {
                    continue;
                }
                let diff = agent.position - other.position;
                let dist = diff.length();
                if dist >= neighbor_dist || dist < 1e-6 {
                    continue;
                }

                let combined_radius = agent.radius + other.radius;
                if dist >= combined_radius {
                    let relative_position = diff / dist;
                    let relative_velocity = agent.velocity - other.velocity;
                    let dist_to_collision = relative_position.dot(relative_velocity);

                    if dist_to_collision < 0.0 {
                        continue;
                    }

                    let time_to_collision = dist_to_collision / relative_velocity.length_squared();
                    if time_to_collision > time_horizon {
                        continue;
                    }

                    let force = relative_position * (combined_radius - dist) / time_to_collision;
                    new_velocity += force * dt;
                } else {
                    let push_dir = diff.normalize_or_zero();
                    let push_force = (combined_radius - dist) / dt;
                    new_velocity += push_dir * push_force;
                }
            }

            if let Some(speed) = new_velocity.try_normalize() {
                let speed_val = new_velocity.length().min(agent.max_speed);
                agent.velocity = speed * speed_val;
            } else {
                agent.velocity = Vec3::ZERO;
            }

            agent.position += agent.velocity * dt;
        }
    }

    /// 构建流场（外部调用，target 为目标点）。
    /// 构建后 ORCA 智能体的 preferred_velocity 会自动从 FlowField 取方向（无 waypoint 路径时）。
    pub fn build_flow_field(&mut self, target_x: usize, target_z: usize) {
        if self.flow_field.is_none() {
            // 默认 64x64 网格，cell_size=1m，origin=[0,0]
            self.flow_field = Some(FlowField::new(64, 64, 1.0, [0.0, 0.0]));
        }
        if let Some(ff) = &mut self.flow_field {
            ff.build_from_navmesh(&self.nav_mesh, target_x, target_z);
        }
    }

    pub fn get_agent_position(&self, id: u64) -> Option<Vec3> {
        self.orca_agents.iter().find(|a| a.id == id).map(|a| a.position)
    }

    pub fn active_agent_count(&self) -> usize {
        self.orca_agents.len()
    }

    pub fn pending_path_requests(&self) -> usize {
        self.path_requests.len()
    }

    pub fn completed_path_results(&self) -> usize {
        self.path_results.len()
    }
}
