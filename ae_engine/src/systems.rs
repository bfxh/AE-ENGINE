use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VisualUnitId(pub Uuid);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualUnit {
    pub id: VisualUnitId,
    pub name: String,
    pub mesh_path: String,
    pub material_path: String,
    pub transform: Transform,
    pub visibility: bool,
    pub layer: u32,
    pub tags: Vec<String>,
    pub cached: bool,
    pub last_used: f64,
    pub usage_count: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: glam::Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self { position: Vec3::ZERO, rotation: glam::Quat::IDENTITY, scale: Vec3::ONE }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Material {
    pub name: String,
    pub shader: String,
    pub properties: HashMap<String, MaterialProperty>,
    pub texture_paths: Vec<String>,
    pub cached: bool,
    pub hit_count: u64,
}

#[derive(Debug, Clone)]
pub enum MaterialProperty {
    Float(f32),
    Vec3(Vec3),
    Color([f32; 4]),
    String(String),
    Bool(bool),
}

impl Serialize for MaterialProperty {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            MaterialProperty::Float(v) => v.serialize(serializer),
            MaterialProperty::Vec3(v) => v.serialize(serializer),
            MaterialProperty::Color(v) => v.serialize(serializer),
            MaterialProperty::String(v) => v.serialize(serializer),
            MaterialProperty::Bool(v) => v.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for MaterialProperty {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value: serde_json::Value = serde::Deserialize::deserialize(deserializer)?;
        if let Some(v) = value.as_f64() {
            Ok(MaterialProperty::Float(v as f32))
        } else if let Some(v) = value.as_str() {
            Ok(MaterialProperty::String(v.to_string()))
        } else if let Some(v) = value.as_bool() {
            Ok(MaterialProperty::Bool(v))
        } else {
            Ok(MaterialProperty::String(value.to_string()))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shader {
    pub name: String,
    pub vertex_source: String,
    pub fragment_source: String,
    pub defines: Vec<String>,
    pub cached: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderLayer {
    pub id: u32,
    pub name: String,
    pub enabled: bool,
    pub priority: i32,
    pub units: Vec<VisualUnitId>,
}

#[derive(Debug)]
pub struct VisualCacheSystem {
    units: RwLock<HashMap<VisualUnitId, VisualUnit>>,
    materials: RwLock<HashMap<String, Material>>,
    shaders: RwLock<HashMap<String, Shader>>,
    layers: RwLock<HashMap<u32, RenderLayer>>,

    lru_cache: Mutex<Vec<VisualUnitId>>,
    cache_size: usize,
    hit_count: Mutex<u64>,
    miss_count: Mutex<u64>,

    recent_access: Mutex<HashMap<VisualUnitId, f64>>,
    usage_patterns: Mutex<HashMap<String, u64>>,
}

impl VisualCacheSystem {
    pub fn new(cache_size: usize) -> Self {
        Self {
            units: RwLock::new(HashMap::new()),
            materials: RwLock::new(HashMap::new()),
            shaders: RwLock::new(HashMap::new()),
            layers: RwLock::new(HashMap::new()),
            lru_cache: Mutex::new(Vec::with_capacity(cache_size)),
            cache_size,
            hit_count: Mutex::new(0),
            miss_count: Mutex::new(0),
            recent_access: Mutex::new(HashMap::new()),
            usage_patterns: Mutex::new(HashMap::new()),
        }
    }

    pub fn add_visual_unit(&self, unit: VisualUnit) {
        let id = unit.id;
        let mut units = self.units.write().unwrap();
        units.insert(id, unit);
        self.update_lru(id);
    }

    pub fn get_visual_unit(&self, id: VisualUnitId) -> Option<VisualUnit> {
        let units = self.units.read().unwrap();
        let unit = units.get(&id).cloned();

        if unit.is_some() {
            *self.hit_count.lock().unwrap() += 1;
            self.update_lru(id);
            self.record_usage(id);
        } else {
            *self.miss_count.lock().unwrap() += 1;
        }

        unit
    }

    pub fn remove_visual_unit(&self, id: VisualUnitId) {
        let mut units = self.units.write().unwrap();
        units.remove(&id);
        self.remove_from_lru(id);
    }

    pub fn add_material(&self, material: Material) {
        let mut materials = self.materials.write().unwrap();
        materials.insert(material.name.clone(), material);
    }

    pub fn get_material(&self, name: &str) -> Option<Material> {
        let mat = {
            let materials = self.materials.read().unwrap();
            materials.get(name).cloned()
        };
        if mat.is_some() {
            let mut mats = self.materials.write().unwrap();
            if let Some(mat_ref) = mats.get_mut(name) {
                mat_ref.hit_count += 1;
            }
        }
        mat
    }

    pub fn add_shader(&self, shader: Shader) {
        let mut shaders = self.shaders.write().unwrap();
        shaders.insert(shader.name.clone(), shader);
    }

    pub fn add_layer(&self, layer: RenderLayer) {
        let mut layers = self.layers.write().unwrap();
        layers.insert(layer.id, layer);
    }

    pub fn update_lru(&self, id: VisualUnitId) {
        let mut lru = self.lru_cache.lock().unwrap();

        if let Some(idx) = lru.iter().position(|&x| x == id) {
            lru.remove(idx);
        }

        lru.insert(0, id);

        if lru.len() > self.cache_size {
            let removed = lru.pop().unwrap();
            self.remove_cached_unit(removed);
        }
    }

    fn remove_from_lru(&self, id: VisualUnitId) {
        let mut lru = self.lru_cache.lock().unwrap();
        if let Some(idx) = lru.iter().position(|&x| x == id) {
            lru.remove(idx);
        }
    }

    fn remove_cached_unit(&self, id: VisualUnitId) {
        let mut units = self.units.write().unwrap();
        if let Some(unit) = units.get_mut(&id) {
            unit.cached = false;
        }
    }

    fn record_usage(&self, id: VisualUnitId) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();

        let mut recent = self.recent_access.lock().unwrap();
        recent.insert(id, now);

        let units = self.units.read().unwrap();
        if let Some(unit) = units.get(&id) {
            let mut patterns = self.usage_patterns.lock().unwrap();
            for tag in &unit.tags {
                *patterns.entry(tag.clone()).or_insert(0) += 1;
            }
        }
    }

    pub fn cache_stats(&self) -> CacheStats {
        let hit = *self.hit_count.lock().unwrap();
        let miss = *self.miss_count.lock().unwrap();
        let total = hit + miss;
        let hit_rate = if total > 0 { hit as f64 / total as f64 } else { 0.0 };

        let lru_len = self.lru_cache.lock().unwrap().len();
        let units_len = self.units.read().unwrap().len();

        let material_hits: u64 = self.materials.read().unwrap().values().map(|m| m.hit_count).sum();

        CacheStats {
            hit_count: hit,
            miss_count: miss,
            hit_rate,
            cached_units: lru_len,
            total_units: units_len,
            material_hits,
        }
    }

    pub fn preload_resources(&self, paths: Vec<String>) {
        for path in paths {
            let name = path.split('/').next_back().unwrap_or("unknown");
            let mat = Material {
                name: name.to_string(),
                shader: "default".to_string(),
                properties: HashMap::new(),
                texture_paths: vec![path],
                cached: true,
                hit_count: 0,
            };
            self.add_material(mat);
        }
    }

    pub fn warm_cache(&self, unit_ids: Vec<VisualUnitId>) {
        for id in unit_ids {
            self.get_visual_unit(id);
        }
    }

    pub fn evict_unused(&self, threshold_seconds: f64) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();

        let recent = self.recent_access.lock().unwrap();
        let mut to_remove = Vec::new();

        for (&id, &access_time) in recent.iter() {
            if now - access_time > threshold_seconds {
                to_remove.push(id);
            }
        }

        drop(recent);

        for id in to_remove {
            self.remove_visual_unit(id);
        }
    }

    pub fn predictive_cache(&self, camera_position: Vec3, radius: f32) {
        // Fix: Collect IDs to cache while holding read lock, then drop read lock
        // before acquiring write lock. Previous code held read lock while trying
        // to acquire write lock on the same RwLock, causing deadlock on std RwLock
        // (non-reentrant).
        let to_cache: Vec<_> = {
            let units = self.units.read().unwrap();
            units
                .iter()
                .filter(|(_, u)| {
                    (u.transform.position - camera_position).length() < radius && !u.cached
                })
                .map(|(_, u)| u.id)
                .collect()
        };

        if !to_cache.is_empty() {
            let mut units = self.units.write().unwrap();
            for id in to_cache {
                if let Some(u) = units.get_mut(&id) {
                    u.cached = true;
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub hit_count: u64,
    pub miss_count: u64,
    pub hit_rate: f64,
    pub cached_units: usize,
    pub total_units: usize,
    pub material_hits: u64,
}

#[derive(Debug, Clone)]
pub struct RenderState {
    pub active_layer: u32,
    pub camera_position: Vec3,
    pub camera_target: Vec3,
    pub fov: f32,
    pub resolution: (u32, u32),
    pub ambient_color: [f32; 4],
    pub sun_direction: Vec3,
    pub sun_intensity: f32,
}

impl Default for RenderState {
    fn default() -> Self {
        Self {
            active_layer: 0,
            camera_position: Vec3::new(0.0, 10.0, 10.0),
            camera_target: Vec3::ZERO,
            fov: 60.0,
            resolution: (1920, 1080),
            ambient_color: [0.1, 0.1, 0.1, 1.0],
            sun_direction: Vec3::new(1.0, -1.0, 1.0).normalize(),
            sun_intensity: 1.0,
        }
    }
}

#[derive(Debug)]
pub struct MemorySystem {
    memories: RwLock<HashMap<Uuid, MemoryEntry>>,
    tags: RwLock<HashMap<String, Vec<Uuid>>>,
    recent_memory: Mutex<Vec<Uuid>>,
    max_memories: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: Uuid,
    pub timestamp: f64,
    pub content: MemoryContent,
    pub tags: Vec<String>,
    pub importance: f32,
    pub accessed_count: u64,
    pub last_accessed: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryContent {
    Event { event_type: String, location: Vec3, description: String, participants: Vec<String> },
    Knowledge { key: String, value: String, source: String },
    StateSnapshot { world_time: f64, entities: Vec<String>, state: serde_json::Value },
    Relationship { subject: String, predicate: String, object: String, confidence: f32 },
}

impl MemorySystem {
    pub fn new(max_memories: usize) -> Self {
        Self {
            memories: RwLock::new(HashMap::new()),
            tags: RwLock::new(HashMap::new()),
            recent_memory: Mutex::new(Vec::new()),
            max_memories,
        }
    }

    pub fn add_memory(&self, content: MemoryContent, tags: Vec<String>) -> Uuid {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();

        let memory = MemoryEntry {
            id: Uuid::new_v4(),
            timestamp: now,
            content,
            tags: tags.clone(),
            importance: 0.5,
            accessed_count: 0,
            last_accessed: now,
        };

        let memory_id = memory.id;

        // Fix: Check capacity and evict BEFORE acquiring write lock.
        // Previous code held write lock on self.memories while calling
        // remove_oldest_memory(), which also tries to acquire write lock = deadlock.
        {
            let memories = self.memories.read().unwrap();
            if memories.len() >= self.max_memories {
                drop(memories);
                self.remove_oldest_memory();
            }
        }

        let mut memories = self.memories.write().unwrap();
        memories.insert(memory_id, memory);

        let mut tag_map = self.tags.write().unwrap();
        for tag in tags {
            tag_map.entry(tag).or_default().push(memory_id);
        }

        memory_id
    }

    fn remove_oldest_memory(&self) {
        // Fix: Clone needed data (id, tags) and drop read lock BEFORE acquiring
        // write lock. Previous code held read lock via `oldest` borrow while
        // trying to acquire write lock, causing deadlock on std RwLock.
        // Additionally, add_memory() may already hold a write lock on self.memories
        // when calling this, so we must not take a read lock either — use write
        // lock directly for both find and remove.
        let (oldest_id, oldest_tags) = {
            let memories = self.memories.write().unwrap();
            memories
                .values()
                .min_by(|a, b| {
                    a.timestamp.partial_cmp(&b.timestamp).unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|o| (o.id, o.tags.clone()))
                .unwrap_or((Uuid::nil(), Vec::new()))
        };

        if oldest_id != Uuid::nil() {
            {
                let mut mems = self.memories.write().unwrap();
                mems.remove(&oldest_id);
            }
            let mut tags = self.tags.write().unwrap();
            for tag in &oldest_tags {
                if let Some(ids) = tags.get_mut(tag) {
                    ids.retain(|&id| id != oldest_id);
                }
            }
        }
    }

    pub fn get_memory(&self, id: Uuid) -> Option<MemoryEntry> {
        let mut memories = self.memories.write().unwrap();
        if let Some(memory) = memories.get_mut(&id) {
            memory.accessed_count += 1;
            memory.last_accessed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64();

            let mut recent = self.recent_memory.lock().unwrap();
            if let Some(idx) = recent.iter().position(|&x| x == id) {
                recent.remove(idx);
            }
            recent.insert(0, id);

            if recent.len() > 100 {
                recent.pop();
            }

            return Some(memory.clone());
        }
        None
    }

    pub fn search_by_tag(&self, tag: &str) -> Vec<MemoryEntry> {
        let tags = self.tags.read().unwrap();
        let memories = self.memories.read().unwrap();

        if let Some(ids) = tags.get(tag) {
            ids.iter().filter_map(|id| memories.get(id).cloned()).collect()
        } else {
            Vec::new()
        }
    }

    pub fn search_by_type(&self, memory_type: &str) -> Vec<MemoryEntry> {
        let memories = self.memories.read().unwrap();
        memories
            .values()
            .filter(|m| match &m.content {
                MemoryContent::Event { .. } => memory_type == "event",
                MemoryContent::Knowledge { .. } => memory_type == "knowledge",
                MemoryContent::StateSnapshot { .. } => memory_type == "snapshot",
                MemoryContent::Relationship { .. } => memory_type == "relationship",
            })
            .cloned()
            .collect()
    }

    pub fn recall_recent(&self, count: usize) -> Vec<MemoryEntry> {
        let recent = self.recent_memory.lock().unwrap();
        let memories = self.memories.read().unwrap();

        recent.iter().take(count).filter_map(|id| memories.get(id).cloned()).collect()
    }

    pub fn update_importance(&self, id: Uuid, importance: f32) {
        let mut memories = self.memories.write().unwrap();
        if let Some(memory) = memories.get_mut(&id) {
            memory.importance = importance.clamp(0.0, 1.0);
        }
    }

    pub fn forget_memory(&self, id: Uuid) {
        let memories = self.memories.read().unwrap();
        let memory = memories.get(&id).cloned();

        if let Some(memory) = memory {
            let mut mems = self.memories.write().unwrap();
            mems.remove(&id);

            let mut tags = self.tags.write().unwrap();
            for tag in &memory.tags {
                if let Some(ids) = tags.get_mut(tag) {
                    *ids = ids.iter().filter(|&&i| i != id).cloned().collect();
                }
            }
        }
    }

    pub fn get_memories_sorted(&self, sort_by: MemorySort) -> Vec<MemoryEntry> {
        let memories = self.memories.read().unwrap();
        let mut result: Vec<_> = memories.values().cloned().collect();

        match sort_by {
            MemorySort::Time => result.sort_by(|a, b| {
                b.timestamp.partial_cmp(&a.timestamp).unwrap_or(std::cmp::Ordering::Equal)
            }),
            MemorySort::Importance => result.sort_by(|a, b| {
                b.importance.partial_cmp(&a.importance).unwrap_or(std::cmp::Ordering::Equal)
            }),
            MemorySort::Access => result.sort_by_key(|b| std::cmp::Reverse(b.accessed_count)),
        }

        result
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemorySort {
    Time,
    Importance,
    Access,
}

#[derive(Debug)]
pub struct SupervisionSystem {
    monitors: RwLock<HashMap<String, Monitor>>,
    alerts: RwLock<Vec<Alert>>,
    metrics: RwLock<HashMap<String, Metric>>,
    alert_callbacks: Mutex<Vec<fn(&Alert)>>,
}

#[derive(Debug, Clone)]
pub struct Monitor {
    pub name: String,
    pub check_interval: f64,
    pub last_check: f64,
    pub threshold: MonitorThreshold,
    pub status: MonitorStatus,
    pub consecutive_failures: u32,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub enum MonitorThreshold {
    Value { min: f64, max: f64 },
    Rate { max_per_second: f64 },
    Count { min: usize, max: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitorStatus {
    Healthy,
    Warning,
    Critical,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct Alert {
    pub id: Uuid,
    pub timestamp: f64,
    pub severity: AlertSeverity,
    pub source: String,
    pub message: String,
    pub resolved: bool,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone)]
pub struct Metric {
    pub name: String,
    pub values: Vec<(f64, f64)>,
    pub max_samples: usize,
    pub unit: String,
}

impl Default for SupervisionSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl SupervisionSystem {
    pub fn new() -> Self {
        Self {
            monitors: RwLock::new(HashMap::new()),
            alerts: RwLock::new(Vec::new()),
            metrics: RwLock::new(HashMap::new()),
            alert_callbacks: Mutex::new(Vec::new()),
        }
    }

    pub fn add_monitor(&self, monitor: Monitor) {
        let mut monitors = self.monitors.write().unwrap();
        monitors.insert(monitor.name.clone(), monitor);
    }

    pub fn remove_monitor(&self, name: &str) {
        let mut monitors = self.monitors.write().unwrap();
        monitors.remove(name);
    }

    pub fn add_alert_callback(&self, callback: fn(&Alert)) {
        let mut callbacks = self.alert_callbacks.lock().unwrap();
        callbacks.push(callback);
    }

    pub fn trigger_alert(
        &self,
        severity: AlertSeverity,
        source: String,
        message: String,
        metadata: HashMap<String, String>,
    ) {
        let alert = Alert {
            id: Uuid::new_v4(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
            severity,
            source,
            message,
            resolved: false,
            metadata,
        };

        let mut alerts = self.alerts.write().unwrap();
        alerts.insert(0, alert.clone());

        if alerts.len() > 1000 {
            alerts.pop();
        }

        let callbacks = self.alert_callbacks.lock().unwrap();
        for callback in callbacks.iter() {
            callback(&alert);
        }
    }

    pub fn resolve_alert(&self, id: Uuid) {
        let mut alerts = self.alerts.write().unwrap();
        for alert in alerts.iter_mut() {
            if alert.id == id {
                alert.resolved = true;
                break;
            }
        }
    }

    pub fn add_metric_sample(&self, name: &str, value: f64, unit: &str) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();

        let mut metrics = self.metrics.write().unwrap();
        let metric = metrics.entry(name.to_string()).or_insert(Metric {
            name: name.to_string(),
            values: Vec::new(),
            max_samples: 600,
            unit: unit.to_string(),
        });

        metric.values.insert(0, (now, value));
        if metric.values.len() > metric.max_samples {
            metric.values.pop();
        }
    }

    pub fn get_metric(&self, name: &str) -> Option<Metric> {
        let metrics = self.metrics.read().unwrap();
        metrics.get(name).cloned()
    }

    pub fn run_monitors(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();

        let mut monitors = self.monitors.write().unwrap();

        for (_, monitor) in monitors.iter_mut() {
            if !monitor.enabled {
                continue;
            }

            if now - monitor.last_check < monitor.check_interval {
                continue;
            }

            monitor.last_check = now;
            let status = self.check_monitor(monitor);

            if status != MonitorStatus::Healthy {
                monitor.consecutive_failures += 1;

                if monitor.consecutive_failures >= 3 {
                    self.trigger_alert(
                        if status == MonitorStatus::Critical {
                            AlertSeverity::Critical
                        } else {
                            AlertSeverity::Warning
                        },
                        monitor.name.clone(),
                        format!("Monitor {} failed", monitor.name),
                        HashMap::new(),
                    );
                }
            } else {
                monitor.consecutive_failures = 0;
            }

            monitor.status = status;
        }
    }

    fn check_monitor(&self, monitor: &Monitor) -> MonitorStatus {
        match &monitor.threshold {
            MonitorThreshold::Value { min, max } => {
                let value = self.get_metric_value(monitor.name.as_str());
                if value < *min || value > *max {
                    if value < *min * 0.5 || value > *max * 1.5 {
                        MonitorStatus::Critical
                    } else {
                        MonitorStatus::Warning
                    }
                } else {
                    MonitorStatus::Healthy
                }
            },
            MonitorThreshold::Count { min, max } => {
                let count = self.get_count_value(monitor.name.as_str());
                if count < *min || count > *max {
                    MonitorStatus::Warning
                } else {
                    MonitorStatus::Healthy
                }
            },
            MonitorThreshold::Rate { max_per_second } => {
                let rate = self.get_rate_value(monitor.name.as_str());
                if rate > *max_per_second { MonitorStatus::Warning } else { MonitorStatus::Healthy }
            },
        }
    }

    fn get_metric_value(&self, _name: &str) -> f64 {
        0.0
    }

    fn get_count_value(&self, _name: &str) -> usize {
        0
    }

    fn get_rate_value(&self, _name: &str) -> f64 {
        0.0
    }

    pub fn get_alerts(&self, severity: Option<AlertSeverity>, resolved: bool) -> Vec<Alert> {
        let alerts = self.alerts.read().unwrap();
        alerts
            .iter()
            .filter(|a| {
                if let Some(s) = severity {
                    a.severity == s && a.resolved == resolved
                } else {
                    a.resolved == resolved
                }
            })
            .cloned()
            .collect()
    }

    pub fn health_report(&self) -> HealthReport {
        let monitors = self.monitors.read().unwrap();
        let alerts = self.alerts.read().unwrap();

        let mut healthy = 0;
        let mut warning = 0;
        let mut critical = 0;

        for monitor in monitors.values() {
            match monitor.status {
                MonitorStatus::Healthy => healthy += 1,
                MonitorStatus::Warning => warning += 1,
                MonitorStatus::Critical => critical += 1,
                _ => {},
            }
        }

        let active_alerts: usize = alerts.iter().filter(|a| !a.resolved).count();

        HealthReport {
            total_monitors: monitors.len(),
            healthy,
            warning,
            critical,
            active_alerts,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct HealthReport {
    pub total_monitors: usize,
    pub healthy: usize,
    pub warning: usize,
    pub critical: usize,
    pub active_alerts: usize,
    pub timestamp: f64,
}

#[derive(Debug)]
pub struct WorkflowSystem {
    workflows: RwLock<HashMap<String, Workflow>>,
    running_tasks: Mutex<HashMap<Uuid, TaskInstance>>,
    scheduler: Arc<TaskScheduler>,
}

#[derive(Debug, Clone)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub tasks: Vec<TaskDefinition>,
    pub triggers: Vec<Trigger>,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct TaskDefinition {
    pub id: String,
    pub name: String,
    pub task_type: TaskType,
    pub parameters: HashMap<String, String>,
    pub dependencies: Vec<String>,
    pub retries: u32,
    pub timeout: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskType {
    LuaScript,
    PythonScript,
    ShellCommand,
    HTTPRequest,
    Delay,
    Conditional,
    Parallel,
    Sequence,
}

#[derive(Debug, Clone)]
pub struct Trigger {
    pub trigger_type: TriggerType,
    pub parameters: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerType {
    TimeInterval,
    WorldTime,
    Event,
    Alert,
    Manual,
}

#[derive(Debug, Clone)]
pub struct TaskInstance {
    pub id: Uuid,
    pub workflow_id: String,
    pub task_def_id: String,
    pub status: TaskStatus,
    pub progress: f32,
    pub started_at: f64,
    pub completed_at: Option<f64>,
    pub error: Option<String>,
    pub retry_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Retrying,
    Cancelled,
}

#[derive(Debug)]
struct TaskScheduler {
    task_queue: Mutex<Vec<TaskInstance>>,
    workers: usize,
    running: Arc<Mutex<bool>>,
}

impl TaskScheduler {
    fn new(workers: usize) -> Self {
        Self { task_queue: Mutex::new(Vec::new()), workers, running: Arc::new(Mutex::new(false)) }
    }

    fn start(self: &Arc<Self>) {
        *self.running.lock().unwrap() = true;
        for _ in 0..self.workers {
            let scheduler = Arc::clone(self);
            std::thread::spawn(move || {
                scheduler.worker_loop();
            });
        }
    }

    fn stop(&self) {
        *self.running.lock().unwrap() = false;
    }

    fn enqueue(&self, task: TaskInstance) {
        let mut queue = self.task_queue.lock().unwrap();
        queue.push(task);
    }

    fn worker_loop(&self) {
        while *self.running.lock().unwrap() {
            let mut queue = self.task_queue.lock().unwrap();
            if queue.is_empty() {
                drop(queue);
                std::thread::sleep(std::time::Duration::from_millis(100));
                continue;
            }

            let task = queue.remove(0);
            drop(queue);

            self.execute_task(task);
        }
    }

    fn execute_task(&self, mut task: TaskInstance) {
        task.status = TaskStatus::Running;
        task.started_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();

        std::thread::sleep(std::time::Duration::from_millis(100));

        task.status = TaskStatus::Completed;
        task.completed_at = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
        );
    }
}

impl WorkflowSystem {
    pub fn new(worker_count: usize) -> Self {
        Self {
            workflows: RwLock::new(HashMap::new()),
            running_tasks: Mutex::new(HashMap::new()),
            scheduler: Arc::new(TaskScheduler::new(worker_count)),
        }
    }

    pub fn add_workflow(&self, workflow: Workflow) {
        let mut workflows = self.workflows.write().unwrap();
        workflows.insert(workflow.id.clone(), workflow);
    }

    pub fn remove_workflow(&self, id: &str) {
        let mut workflows = self.workflows.write().unwrap();
        workflows.remove(id);
    }

    pub fn start_workflow(&self, workflow_id: &str) -> Option<Uuid> {
        let workflows = self.workflows.read().unwrap();
        let workflow = workflows.get(workflow_id)?;

        if !workflow.enabled {
            return None;
        }

        for task_def in &workflow.tasks {
            let instance = TaskInstance {
                id: Uuid::new_v4(),
                workflow_id: workflow_id.to_string(),
                task_def_id: task_def.id.clone(),
                status: TaskStatus::Queued,
                progress: 0.0,
                started_at: 0.0,
                completed_at: None,
                error: None,
                retry_count: 0,
            };

            self.running_tasks.lock().unwrap().insert(instance.id, instance.clone());
            self.scheduler.enqueue(instance);
        }

        Some(Uuid::new_v4())
    }

    pub fn start(&self) {
        self.scheduler.start();
    }

    pub fn stop(&self) {
        self.scheduler.stop();
    }

    pub fn get_task_status(&self, task_id: Uuid) -> Option<TaskStatus> {
        let tasks = self.running_tasks.lock().unwrap();
        tasks.get(&task_id).map(|t| t.status)
    }

    pub fn cancel_task(&self, task_id: Uuid) {
        let mut tasks = self.running_tasks.lock().unwrap();
        if let Some(task) = tasks.get_mut(&task_id) {
            task.status = TaskStatus::Cancelled;
        }
    }

    pub fn schedule_repeating(&self, interval_seconds: f64, workflow_id: String) {
        let scheduler = Arc::clone(&self.scheduler);
        std::thread::spawn(move || {
            let interval = std::time::Duration::from_secs_f64(interval_seconds);
            loop {
                std::thread::sleep(interval);
                let task = TaskInstance {
                    id: Uuid::new_v4(),
                    workflow_id: workflow_id.clone(),
                    task_def_id: String::new(),
                    status: TaskStatus::Queued,
                    progress: 0.0,
                    started_at: 0.0,
                    completed_at: None,
                    error: None,
                    retry_count: 0,
                };
                scheduler.enqueue(task);
            }
        });
    }

    pub fn get_workflow_status(&self, workflow_id: &str) -> WorkflowStatus {
        let tasks = self.running_tasks.lock().unwrap();
        let workflows = self.workflows.read().unwrap();
        let workflow = workflows.get(workflow_id);

        let task_count = tasks.values().filter(|t| t.workflow_id == workflow_id).count();

        let completed = tasks
            .values()
            .filter(|t| t.workflow_id == workflow_id && t.status == TaskStatus::Completed)
            .count();

        let failed = tasks
            .values()
            .filter(|t| t.workflow_id == workflow_id && t.status == TaskStatus::Failed)
            .count();

        WorkflowStatus {
            workflow_id: workflow_id.to_string(),
            total_tasks: workflow.map(|w| w.tasks.len()).unwrap_or(0),
            running_tasks: task_count,
            completed_tasks: completed,
            failed_tasks: failed,
            enabled: workflow.map(|w| w.enabled).unwrap_or(false),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WorkflowStatus {
    pub workflow_id: String,
    pub total_tasks: usize,
    pub running_tasks: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
    pub enabled: bool,
}

pub type SystemArc<T> = Arc<T>;
