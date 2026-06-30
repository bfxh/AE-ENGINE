use std::collections::VecDeque;

pub const MAX_SAVED_STATES: usize = 32;

#[derive(Debug, Clone)]
pub struct RollbackState {
    pub frame: u64,
    pub state_data: Vec<u8>,
    pub checksum: u64,
}

impl RollbackState {
    pub fn new(frame: u64, state_data: Vec<u8>) -> Self {
        let checksum = RollbackState::compute_checksum(&state_data);
        RollbackState { frame, state_data, checksum }
    }

    fn compute_checksum(data: &[u8]) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for &byte in data {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }
}

pub struct RollbackBuffer {
    saved_states: VecDeque<RollbackState>,
    pub current_frame: u64,
    pub peer_frame: u64,
}

impl RollbackBuffer {
    pub fn new() -> Self {
        RollbackBuffer {
            saved_states: VecDeque::with_capacity(MAX_SAVED_STATES),
            current_frame: 0,
            peer_frame: 0,
        }
    }

    pub fn save_state(&mut self, frame: u64, state_data: Vec<u8>) {
        if self.saved_states.len() >= MAX_SAVED_STATES {
            self.saved_states.pop_front();
        }
        self.saved_states.push_back(RollbackState::new(frame, state_data));
        self.current_frame = frame;
    }

    pub fn rollback_to(&mut self, target_frame: u64) -> Option<RollbackState> {
        while self.saved_states.len() > 1
            && self.saved_states.back().is_some_and(|s| s.frame > target_frame)
        {
            self.saved_states.pop_back();
        }
        self.saved_states.back().cloned()
    }

    pub fn should_rollback(&self, remote_frame: u64, remote_checksum: u64) -> Option<u64> {
        for state in self.saved_states.iter().rev() {
            if state.frame == remote_frame && state.checksum != remote_checksum {
                return Some(remote_frame);
            }
        }
        None
    }

    pub fn resimulate_frames(&self, from_frame: u64) -> Vec<u64> {
        let mut frames: Vec<u64> = (from_frame..=self.current_frame).collect();
        frames.sort();
        frames
    }

    pub fn clear_old_states(&mut self, before_frame: u64) {
        while let Some(front) = self.saved_states.front() {
            if front.frame < before_frame {
                self.saved_states.pop_front();
            } else {
                break;
            }
        }
    }
}

impl Default for RollbackBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_and_rollback() {
        let mut buf = RollbackBuffer::new();
        buf.save_state(0, vec![0, 0, 0]);
        buf.save_state(1, vec![1, 1, 1]);
        buf.save_state(2, vec![2, 2, 2]);
        let state = buf.rollback_to(1).unwrap();
        assert_eq!(state.frame, 1);
        assert_eq!(state.state_data, vec![1, 1, 1]);
    }

    #[test]
    fn test_rollback_to_exact() {
        let mut buf = RollbackBuffer::new();
        buf.save_state(5, vec![5]);
        buf.save_state(10, vec![10]);
        let state = buf.rollback_to(5).unwrap();
        assert_eq!(state.frame, 5);
    }

    #[test]
    fn test_rollback_past_earliest() {
        let mut buf = RollbackBuffer::new();
        buf.save_state(5, vec![5]);
        let state = buf.rollback_to(0).unwrap();
        assert_eq!(state.frame, 5);
    }

    #[test]
    fn test_checksum_mismatch_detection() {
        let mut buf = RollbackBuffer::new();
        buf.save_state(0, vec![1, 2, 3]);
        let should = buf.should_rollback(0, 0xdeadbeef);
        assert!(should.is_some());
    }

    #[test]
    fn test_checksum_match_no_rollback() {
        let mut buf = RollbackBuffer::new();
        let state = RollbackState::new(0, vec![1, 2, 3]);
        let checksum = state.checksum;
        buf.saved_states.push_back(state);
        let should = buf.should_rollback(0, checksum);
        assert!(should.is_none());
    }

    #[test]
    fn test_clear_old_states() {
        let mut buf = RollbackBuffer::new();
        for i in 0..10 {
            buf.save_state(i, vec![i as u8]);
        }
        buf.clear_old_states(5);
        assert_eq!(buf.saved_states.front().unwrap().frame, 5);
    }
}
