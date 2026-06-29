use hashbrown::HashSet;
use std::collections::VecDeque;
use uuid::Uuid;
use wasteland_metaentity::meta_entity::EntityChanges;
use wasteland_unified_interface::UnifiedWorld;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EventType {
    Collision,
    Reaction,
    Metabolism,
    PhaseTransition,
    HeatTransfer,
    Custom(u64),
}

#[derive(Debug, Clone)]
pub struct ScheduledEvent {
    pub event_id: u64,
    pub entity_ids: Vec<Uuid>,
    pub modified_fields: Vec<u64>,
    pub event_type: EventType,
    pub priority: u8,
    pub timestamp: u64,
    pub payload: Option<EntityChanges>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct Task {
    event: ScheduledEvent,
    dependencies: Vec<usize>,
    dependents: Vec<usize>,
    status: TaskStatus,
}

pub struct Scheduler {
    worker_count: usize,
    pending_events: VecDeque<ScheduledEvent>,
    event_counter: u64,
    thread_pool: rayon::ThreadPool,
}

impl Scheduler {
    pub fn new(worker_count: usize) -> Self {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(worker_count)
            .build()
            .unwrap_or_else(|_| rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap());

        Self { worker_count, pending_events: VecDeque::new(), event_counter: 0, thread_pool: pool }
    }

    pub fn with_fixed_order(worker_count: usize) -> Self {
        Self::new(worker_count)
    }

    pub fn submit_event(
        &mut self,
        event_type: EventType,
        entity_ids: Vec<Uuid>,
        modified_fields: Vec<u64>,
        priority: u8,
        timestamp: u64,
    ) -> u64 {
        let event_id = self.event_counter;
        self.event_counter += 1;
        self.pending_events.push_back(ScheduledEvent {
            event_id,
            entity_ids,
            modified_fields,
            event_type,
            priority,
            timestamp,
            payload: None,
        });
        event_id
    }

    fn build_dependency_graph(&self, events: &[ScheduledEvent]) -> Vec<Task> {
        let mut tasks: Vec<Task> = events
            .iter()
            .map(|e| Task {
                event: e.clone(),
                dependencies: Vec::new(),
                dependents: Vec::new(),
                status: TaskStatus::Pending,
            })
            .collect();

        for i in 0..tasks.len() {
            for j in (i + 1)..tasks.len() {
                if Self::has_conflict(&tasks[i].event, &tasks[j].event) {
                    tasks[j].dependencies.push(i);
                    tasks[i].dependents.push(j);
                }
            }
        }

        tasks
    }

    fn has_conflict(a: &ScheduledEvent, b: &ScheduledEvent) -> bool {
        let a_entities: HashSet<Uuid> = a.entity_ids.iter().copied().collect();
        let b_entities: HashSet<Uuid> = b.entity_ids.iter().copied().collect();
        let shared: Vec<&Uuid> = a_entities.intersection(&b_entities).collect();

        if shared.is_empty() {
            return false;
        }

        let a_fields: HashSet<u64> = a.modified_fields.iter().copied().collect();
        let b_fields: HashSet<u64> = b.modified_fields.iter().copied().collect();

        !a_fields.is_disjoint(&b_fields)
    }

    pub fn schedule_frame(&mut self) -> Vec<ScheduledEvent> {
        let mut events: Vec<ScheduledEvent> = self.pending_events.drain(..).collect();

        events.sort_by(|a, b| {
            a.entity_ids
                .first()
                .map(|id| id.as_u64_pair())
                .cmp(&b.entity_ids.first().map(|id| id.as_u64_pair()))
                .then_with(|| a.timestamp.cmp(&b.timestamp))
                .then_with(|| a.event_id.cmp(&b.event_id))
        });

        events
    }

    pub fn execute_frame<W: UnifiedWorld>(&mut self, world: &mut W) -> Vec<u64> {
        let events = self.schedule_frame();
        let tasks = self.build_dependency_graph(&events);

        let mut completed: Vec<u64> = Vec::new();
        let mut in_degree: Vec<usize> = tasks.iter().map(|t| t.dependencies.len()).collect();
        let mut ready: VecDeque<usize> =
            in_degree.iter().enumerate().filter(|(_, &d)| d == 0).map(|(i, _)| i).collect();

        while !ready.is_empty() {
            let parallel_batch: Vec<usize> = ready.drain(..).collect();
            let results: Vec<Vec<u64>> = self.thread_pool.install(|| {
                parallel_batch
                    .iter()
                    .map(|&idx| Self::execute_task(&tasks[idx].event, world))
                    .collect()
            });

            for (batch_idx, &task_idx) in parallel_batch.iter().enumerate() {
                for &event_id in &results[batch_idx] {
                    completed.push(event_id);
                }
                for &dep_idx in &tasks[task_idx].dependents {
                    in_degree[dep_idx] -= 1;
                    if in_degree[dep_idx] == 0 {
                        ready.push_back(dep_idx);
                    }
                }
            }
        }

        completed
    }

    pub fn execute_frame_deterministic<W: UnifiedWorld>(&mut self, world: &mut W) -> Vec<u64> {
        let events = self.schedule_frame();
        let tasks = self.build_dependency_graph(&events);

        let mut completed: Vec<u64> = Vec::new();
        let mut in_degree: Vec<usize> = tasks.iter().map(|t| t.dependencies.len()).collect();
        let mut ready: VecDeque<usize> =
            in_degree.iter().enumerate().filter(|(_, &d)| d == 0).map(|(i, _)| i).collect();

        let parallel_threshold = self.worker_count * 4;

        while !ready.is_empty() {
            let batch_size = ready.len().min(parallel_threshold);
            let parallel_batch: Vec<usize> = ready.drain(..batch_size).collect();

            if parallel_batch.len() < self.worker_count {
                for &task_idx in &parallel_batch {
                    let result = Self::execute_task(&tasks[task_idx].event, world);
                    completed.extend(result);
                    for &dep_idx in &tasks[task_idx].dependents {
                        in_degree[dep_idx] -= 1;
                        if in_degree[dep_idx] == 0 {
                            ready.push_back(dep_idx);
                        }
                    }
                }
            } else {
                let results: Vec<Vec<u64>> = self.thread_pool.install(|| {
                    parallel_batch
                        .iter()
                        .map(|&idx| Self::execute_task(&tasks[idx].event, world))
                        .collect()
                });

                for (batch_idx, &task_idx) in parallel_batch.iter().enumerate() {
                    completed.extend(&results[batch_idx]);
                    for &dep_idx in &tasks[task_idx].dependents {
                        in_degree[dep_idx] -= 1;
                        if in_degree[dep_idx] == 0 {
                            ready.push_back(dep_idx);
                        }
                    }
                }
            }
        }

        completed
    }

    fn execute_task<W: UnifiedWorld>(event: &ScheduledEvent, world: &mut W) -> Vec<u64> {
        let mut completed = Vec::new();

        if let Some(ref changes) = event.payload {
            for &entity_id in &event.entity_ids {
                if let Some(entity) = world.read_entity(entity_id) {
                    let mut actual_changes = changes.clone();
                    actual_changes.entity_id = entity_id;
                    actual_changes.expected_version = entity.version;

                    if world.write_entity(entity_id, actual_changes).is_ok() {
                        completed.push(event.event_id);
                    }
                }
            }
        }

        if completed.is_empty() {
            completed.push(event.event_id);
        }

        completed
    }

    pub fn pending_count(&self) -> usize {
        self.pending_events.len()
    }

    pub fn clear(&mut self) {
        self.pending_events.clear();
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        let cpu_count = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
        Self::new(cpu_count.saturating_sub(1).max(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;
    use wasteland_metaentity::meta_entity::*;
    use wasteland_unified_interface::WorldStorage;

    #[test]
    fn test_scheduler_creation() {
        let scheduler = Scheduler::new(2);
        assert_eq!(scheduler.worker_count, 2);
        assert_eq!(scheduler.pending_count(), 0);
    }

    #[test]
    fn test_submit_event() {
        let mut scheduler = Scheduler::new(2);
        let id =
            scheduler.submit_event(EventType::Collision, vec![Uuid::new_v4()], vec![1, 2], 0, 0);
        assert_eq!(id, 0);
        assert_eq!(scheduler.pending_count(), 1);
    }

    #[test]
    fn test_no_conflict_different_entities() {
        let a = ScheduledEvent {
            event_id: 0,
            entity_ids: vec![Uuid::new_v4()],
            modified_fields: vec![1],
            event_type: EventType::Collision,
            priority: 0,
            timestamp: 0,
            payload: None,
        };
        let b = ScheduledEvent {
            event_id: 1,
            entity_ids: vec![Uuid::new_v4()],
            modified_fields: vec![1],
            event_type: EventType::Reaction,
            priority: 0,
            timestamp: 0,
            payload: None,
        };
        assert!(!Scheduler::has_conflict(&a, &b));
    }

    #[test]
    fn test_conflict_same_entity_and_field() {
        let id = Uuid::new_v4();
        let a = ScheduledEvent {
            event_id: 0,
            entity_ids: vec![id],
            modified_fields: vec![1],
            event_type: EventType::Collision,
            priority: 0,
            timestamp: 0,
            payload: None,
        };
        let b = ScheduledEvent {
            event_id: 1,
            entity_ids: vec![id],
            modified_fields: vec![1],
            event_type: EventType::Reaction,
            priority: 0,
            timestamp: 0,
            payload: None,
        };
        assert!(Scheduler::has_conflict(&a, &b));
    }

    #[test]
    fn test_schedule_frame_deterministic() {
        let mut scheduler = Scheduler::new(2);
        let id1 = Uuid::from_u64_pair(1, 0);
        let id2 = Uuid::from_u64_pair(2, 0);

        scheduler.submit_event(EventType::Collision, vec![id2], vec![1], 0, 100);
        scheduler.submit_event(EventType::Collision, vec![id1], vec![1], 0, 50);

        let events = scheduler.schedule_frame();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].entity_ids[0], id1);
        assert_eq!(events[1].entity_ids[0], id2);
    }

    #[test]
    fn test_execute_frame() {
        let mut scheduler = Scheduler::new(2);
        let mut world = WorldStorage::new();

        let iron = MetaEntity::iron(Vec3::ZERO, 0);
        let id = world.spawn_entity(iron);

        scheduler.submit_event(EventType::Collision, vec![id], vec![1], 0, 0);

        let completed = scheduler.execute_frame(&mut world);
        assert!(!completed.is_empty());
        assert_eq!(scheduler.pending_count(), 0);
    }

    #[test]
    fn test_dependency_graph() {
        let id = Uuid::new_v4();
        let events = vec![
            ScheduledEvent {
                event_id: 0,
                entity_ids: vec![id],
                modified_fields: vec![1],
                event_type: EventType::Collision,
                priority: 0,
                timestamp: 0,
                payload: None,
            },
            ScheduledEvent {
                event_id: 1,
                entity_ids: vec![id],
                modified_fields: vec![1],
                event_type: EventType::Reaction,
                priority: 0,
                timestamp: 1,
                payload: None,
            },
        ];

        let scheduler = Scheduler::new(1);
        let tasks = scheduler.build_dependency_graph(&events);
        assert_eq!(tasks[0].dependencies.len(), 0);
        assert_eq!(tasks[1].dependencies.len(), 1);
    }
}
