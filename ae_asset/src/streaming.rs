use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamPriority {
    Critical,
    High,
    Normal,
    Low,
    Background,
}

impl StreamPriority {
    pub fn order(&self) -> u8 {
        match self {
            StreamPriority::Critical => 0,
            StreamPriority::High => 1,
            StreamPriority::Normal => 2,
            StreamPriority::Low => 3,
            StreamPriority::Background => 4,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StreamRequest {
    pub asset_id: u64,
    pub priority: StreamPriority,
    pub offset: u64,
    pub size: u64,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct StreamScheduler {
    queue: VecDeque<StreamRequest>,
    max_bandwidth: u64,
    #[allow(dead_code)]
    current_bandwidth: u64,
    frame_budget: u64,
    active_streams: usize,
    max_streams: usize,
    total_bytes_streamed: u64,
    frame_bytes: u64,
}

impl StreamScheduler {
    pub fn new(max_bandwidth_per_sec: u64, max_streams: usize) -> Self {
        StreamScheduler {
            queue: VecDeque::new(),
            max_bandwidth: max_bandwidth_per_sec,
            current_bandwidth: 0,
            frame_budget: max_bandwidth_per_sec / 60,
            active_streams: 0,
            max_streams,
            total_bytes_streamed: 0,
            frame_bytes: 0,
        }
    }

    pub fn request(&mut self, request: StreamRequest) {
        let pos = self
            .queue
            .iter()
            .position(|r| r.priority.order() > request.priority.order())
            .unwrap_or(self.queue.len());
        self.queue.insert(pos, request);
    }

    pub fn process_frame(&mut self, delta_secs: f32) -> Vec<StreamRequest> {
        self.frame_bytes = 0;
        self.frame_budget = (self.max_bandwidth as f32 * delta_secs) as u64;
        let mut dispatched = Vec::new();
        while self.active_streams < self.max_streams && self.frame_bytes < self.frame_budget {
            if let Some(req) = self.queue.pop_front() {
                let bytes = req.size.min(self.frame_budget - self.frame_bytes);
                self.frame_bytes += bytes;
                self.total_bytes_streamed += bytes;
                dispatched.push(StreamRequest {
                    asset_id: req.asset_id,
                    priority: req.priority,
                    offset: req.offset,
                    size: bytes,
                    timestamp: req.timestamp,
                });
                if bytes < req.size {
                    self.queue.push_front(StreamRequest {
                        size: req.size - bytes,
                        offset: req.offset + bytes,
                        ..req
                    });
                }
                self.active_streams += 1;
            } else {
                break;
            }
        }
        dispatched
    }

    pub fn complete_stream(&mut self) {
        self.active_streams = self.active_streams.saturating_sub(1);
    }

    pub fn cancel_asset(&mut self, asset_id: u64) {
        self.queue.retain(|r| r.asset_id != asset_id);
    }

    pub fn pending_count(&self) -> usize {
        self.queue.len()
    }

    pub fn pending_bytes(&self) -> u64 {
        self.queue.iter().map(|r| r.size).sum()
    }

    pub fn bandwidth_usage(&self) -> f32 {
        if self.max_bandwidth == 0 {
            return 0.0;
        }
        self.frame_bytes as f32 / self.max_bandwidth as f32
    }

    pub fn set_max_bandwidth(&mut self, bytes_per_sec: u64) {
        self.max_bandwidth = bytes_per_sec;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_request(id: u64, prio: StreamPriority, size: u64) -> StreamRequest {
        StreamRequest { asset_id: id, priority: prio, offset: 0, size, timestamp: 0 }
    }

    #[test]
    fn test_priority_ordering() {
        let mut sched = StreamScheduler::new(1024 * 1024, 4);
        sched.request(make_request(1, StreamPriority::Low, 100));
        sched.request(make_request(2, StreamPriority::Critical, 100));
        sched.request(make_request(3, StreamPriority::Normal, 100));
        let dispatched = sched.process_frame(1.0 / 60.0);
        assert_eq!(dispatched[0].asset_id, 2);
        assert_eq!(dispatched[1].asset_id, 3);
        assert_eq!(dispatched[2].asset_id, 1);
    }

    #[test]
    fn test_bandwidth_limit() {
        let mut sched = StreamScheduler::new(1000, 4);
        sched.request(make_request(1, StreamPriority::Normal, 500));
        sched.request(make_request(2, StreamPriority::Normal, 500));
        let dispatched = sched.process_frame(1.0 / 60.0);
        let total: u64 = dispatched.iter().map(|r| r.size).sum();
        assert!(total <= 1000 / 60);
    }

    #[test]
    fn test_cancel_asset() {
        let mut sched = StreamScheduler::new(1024 * 1024, 4);
        sched.request(make_request(1, StreamPriority::Normal, 100));
        sched.request(make_request(2, StreamPriority::Normal, 100));
        sched.cancel_asset(1);
        assert_eq!(sched.pending_count(), 1);
    }

    #[test]
    fn test_complete_stream() {
        let mut sched = StreamScheduler::new(1024 * 1024, 2);
        sched.request(make_request(1, StreamPriority::Normal, 100));
        sched.request(make_request(2, StreamPriority::Normal, 100));
        sched.request(make_request(3, StreamPriority::Normal, 100));
        let dispatched = sched.process_frame(1.0 / 60.0);
        assert_eq!(dispatched.len(), 2);
        sched.complete_stream();
        let dispatched2 = sched.process_frame(1.0 / 60.0);
        assert_eq!(dispatched2.len(), 1);
    }

    #[test]
    fn test_bandwidth_usage() {
        let mut sched = StreamScheduler::new(10000, 4);
        sched.request(make_request(1, StreamPriority::Normal, 1000));
        sched.process_frame(1.0);
        assert!(sched.bandwidth_usage() > 0.0);
    }
}
