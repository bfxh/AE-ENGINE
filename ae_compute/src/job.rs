use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum JobPriority {
    Critical = 0,
    High = 1,
    Normal = 2,
    Low = 3,
    Background = 4,
}

pub struct Job {
    pub id: u64,
    pub name: String,
    pub priority: JobPriority,
    pub dependencies: Vec<u64>,
    pub work: Option<Box<dyn FnOnce() + Send>>,
}

impl std::fmt::Debug for Job {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Job")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("priority", &self.priority)
            .field("dependencies", &self.dependencies)
            .field("work", &"<closure>")
            .finish()
    }
}

impl Job {
    pub fn new<F>(id: u64, name: &str, priority: JobPriority, work: F) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        Self {
            id,
            name: name.to_string(),
            priority,
            dependencies: Vec::new(),
            work: Some(Box::new(work)),
        }
    }

    pub fn with_dependencies(mut self, deps: Vec<u64>) -> Self {
        self.dependencies = deps;
        self
    }
}

struct Worker {
    thread: Option<JoinHandle<()>>,
    #[allow(dead_code)]
    running: Arc<AtomicBool>,
    #[allow(dead_code)]
    worker_id: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobState {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug)]
struct JobEntry {
    job: Option<Job>,
    state: JobState,
}

pub struct JobSystem {
    workers: Vec<Worker>,
    jobs: Arc<Mutex<Vec<Arc<Mutex<JobEntry>>>>>,
    #[allow(dead_code)]
    next_job_id: AtomicU64,
    running: Arc<AtomicBool>,
    completed_count: Arc<AtomicU64>,
    failed_count: Arc<AtomicU64>,
    steal_queue: Arc<Mutex<Vec<usize>>>,
}

impl JobSystem {
    pub fn new(num_workers: usize) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let completed = Arc::new(AtomicU64::new(0));
        let failed = Arc::new(AtomicU64::new(0));
        let steal_queue: Arc<Mutex<Vec<usize>>> = Arc::new(Mutex::new(Vec::new()));
        let jobs: Arc<Mutex<Vec<Arc<Mutex<JobEntry>>>>> = Arc::new(Mutex::new(Vec::new()));

        let mut workers = Vec::with_capacity(num_workers);
        for i in 0..num_workers {
            let r = running.clone();
            let c = completed.clone();
            let _f = failed.clone();
            let sq = steal_queue.clone();
            let worker_jobs = jobs.clone();

            let thread = thread::spawn(move || {
                while r.load(Ordering::Relaxed) {
                    let stolen = {
                        let mut sq = sq.lock().unwrap();
                        sq.pop()
                    };

                    if let Some(idx) = stolen {
                        let job_list = worker_jobs.lock().unwrap();
                        if let Some(entry_arc) = job_list.get(idx).cloned() {
                            drop(job_list);
                            let mut entry = entry_arc.lock().unwrap();
                            if entry.state == JobState::Pending {
                                entry.state = JobState::Running;
                                drop(entry);

                                let mut job_entry = entry_arc.lock().unwrap();
                                if let Some(ref mut job) = job_entry.job {
                                    if let Some(work) = job.work.take() {
                                        drop(job_entry);
                                        work();
                                        c.fetch_add(1, Ordering::Relaxed);
                                        let mut entry = entry_arc.lock().unwrap();
                                        entry.state = JobState::Completed;
                                        entry.job = None;
                                    }
                                }
                            }
                        }
                    } else {
                        thread::sleep(std::time::Duration::from_micros(100));
                    }
                }
            });

            workers.push(Worker { thread: Some(thread), running: running.clone(), worker_id: i });
        }

        Self {
            workers,
            jobs,
            next_job_id: AtomicU64::new(1),
            running,
            completed_count: completed,
            failed_count: failed,
            steal_queue,
        }
    }

    pub fn submit(&mut self, job: Job) -> u64 {
        let id = job.id;
        let mut jobs = self.jobs.lock().unwrap();
        jobs.push(Arc::new(Mutex::new(JobEntry { job: Some(job), state: JobState::Pending })));
        let idx = jobs.len() - 1;
        drop(jobs);
        self.steal_queue.lock().unwrap().push(idx);
        id
    }

    pub fn submit_batch(&mut self, jobs: Vec<Job>) -> Vec<u64> {
        let mut ids = Vec::with_capacity(jobs.len());
        for job in jobs {
            ids.push(self.submit(job));
        }
        ids
    }

    pub fn wait_all(&self) {
        while self.pending_count() > 0 {
            thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    pub fn pending_count(&self) -> usize {
        self.jobs
            .lock()
            .unwrap()
            .iter()
            .filter(|j| {
                let entry = j.lock().unwrap();
                entry.state == JobState::Pending || entry.state == JobState::Running
            })
            .count()
    }

    pub fn completed_count(&self) -> u64 {
        self.completed_count.load(Ordering::Relaxed)
    }

    pub fn failed_count(&self) -> u64 {
        self.failed_count.load(Ordering::Relaxed)
    }

    pub fn shutdown(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                let _ = thread.join();
            }
        }
    }
}

impl Drop for JobSystem {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

#[derive(Debug)]
pub struct JobGraph {
    jobs: Vec<Job>,
    adjacency: Vec<Vec<usize>>,
    in_degree: Vec<usize>,
}

impl JobGraph {
    pub fn new() -> Self {
        Self { jobs: Vec::new(), adjacency: Vec::new(), in_degree: Vec::new() }
    }

    pub fn add_job(&mut self, job: Job) -> usize {
        let idx = self.jobs.len();
        self.jobs.push(job);
        self.adjacency.push(Vec::new());
        self.in_degree.push(0);
        idx
    }

    pub fn add_dependency(&mut self, from_idx: usize, to_idx: usize) {
        if from_idx < self.adjacency.len() && to_idx < self.in_degree.len() {
            self.adjacency[from_idx].push(to_idx);
            self.in_degree[to_idx] += 1;
        }
    }

    pub fn topological_sort(&self) -> Vec<Vec<usize>> {
        let mut result = Vec::new();
        let mut in_deg = self.in_degree.clone();
        let n = self.jobs.len();

        while result.iter().flatten().count() < n {
            let mut current_level = Vec::new();
            for (i, &deg) in in_deg.iter().enumerate() {
                if deg == 0 {
                    current_level.push(i);
                }
            }

            if current_level.is_empty() {
                break;
            }

            for &node in &current_level {
                in_deg[node] = usize::MAX;
                for &neighbor in &self.adjacency[node] {
                    if in_deg[neighbor] != usize::MAX {
                        in_deg[neighbor] -= 1;
                    }
                }
            }

            result.push(current_level);
        }

        result
    }

    pub fn execute_parallel(&mut self, system: &mut JobSystem) -> Vec<u64> {
        let levels = self.topological_sort();
        let mut all_ids = Vec::new();

        for level in levels {
            let level_jobs: Vec<Job> = level
                .iter()
                .map(|&idx| {
                    let mut placeholder =
                        Job::new(idx as u64, "placeholder", JobPriority::Normal, || {});
                    std::mem::swap(&mut placeholder, &mut self.jobs[idx]);
                    placeholder
                })
                .collect();
            let ids = system.submit_batch(level_jobs);
            system.wait_all();
            all_ids.extend(ids);
        }

        all_ids
    }

    pub fn len(&self) -> usize {
        self.jobs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.jobs.is_empty()
    }
}

impl Default for JobGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn test_job_system_submit() {
        let mut system = JobSystem::new(2);
        let counter = Arc::new(AtomicUsize::new(0));
        let c = counter.clone();
        system.submit(Job::new(1, "test", JobPriority::Normal, move || {
            c.fetch_add(1, Ordering::Relaxed);
        }));
        system.wait_all();
        assert_eq!(counter.load(Ordering::Relaxed), 1);
        system.shutdown();
    }

    #[test]
    fn test_job_graph_topo_sort() {
        let mut graph = JobGraph::new();
        let a = graph.add_job(Job::new(1, "a", JobPriority::Normal, || {}));
        let b = graph.add_job(Job::new(2, "b", JobPriority::Normal, || {}));
        let c = graph.add_job(Job::new(3, "c", JobPriority::Normal, || {}));
        graph.add_dependency(a, b);
        graph.add_dependency(b, c);

        let levels = graph.topological_sort();
        assert_eq!(levels.len(), 3);
        assert_eq!(levels[0], vec![a]);
        assert_eq!(levels[1], vec![b]);
        assert_eq!(levels[2], vec![c]);
    }

    #[test]
    fn test_job_graph_parallel_levels() {
        let mut graph = JobGraph::new();
        graph.add_job(Job::new(1, "a", JobPriority::Normal, || {}));
        graph.add_job(Job::new(2, "b", JobPriority::Normal, || {}));

        let levels = graph.topological_sort();
        assert_eq!(levels.len(), 1);
        assert_eq!(levels[0].len(), 2);
    }

    #[test]
    fn test_multiple_jobs() {
        let mut system = JobSystem::new(4);
        let counter = Arc::new(AtomicUsize::new(0));

        for i in 0..100 {
            let c = counter.clone();
            system.submit(Job::new(i, "job", JobPriority::Normal, move || {
                c.fetch_add(1, Ordering::Relaxed);
            }));
        }
        system.wait_all();
        assert_eq!(counter.load(Ordering::Relaxed), 100);
        system.shutdown();
    }
}
