use std::collections::HashMap;

pub const MAX_ROLLBACK_FRAMES: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputBits(pub u32);

impl InputBits {
    pub fn new() -> Self {
        InputBits(0)
    }

    pub fn set(&mut self, bit: u8, value: bool) {
        if bit < 32 {
            if value {
                self.0 |= 1 << bit;
            } else {
                self.0 &= !(1 << bit);
            }
        }
    }

    pub fn get(&self, bit: u8) -> bool {
        if bit < 32 { (self.0 >> bit) & 1 == 1 } else { false }
    }

    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

impl Default for InputBits {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct LockstepState {
    pub current_frame: u64,
    pub confirmed_frame: u64,
    pub local_inputs: HashMap<u64, InputBits>,
    pub remote_inputs: HashMap<u32, HashMap<u64, InputBits>>,
    pub input_delay: u64,
    pub max_players: u32,
}

impl LockstepState {
    pub fn new(input_delay: u64, max_players: u32) -> Self {
        LockstepState {
            current_frame: 0,
            confirmed_frame: 0,
            local_inputs: HashMap::new(),
            remote_inputs: HashMap::new(),
            input_delay,
            max_players,
        }
    }

    pub fn advance_frame(&mut self, local_input: InputBits) {
        self.local_inputs.insert(self.current_frame, local_input);
        self.current_frame += 1;
    }

    pub fn add_remote_input(&mut self, player_id: u32, frame: u64, input: InputBits) {
        self.remote_inputs.entry(player_id).or_default().insert(frame, input);
    }

    pub fn is_frame_ready(&self, frame: u64) -> bool {
        if frame > self.current_frame {
            return false;
        }
        if !self.local_inputs.contains_key(&frame) {
            return false;
        }
        for player_id in 0..self.max_players {
            if let Some(inputs) = self.remote_inputs.get(&player_id) {
                if !inputs.contains_key(&frame) {
                    return false;
                }
            }
        }
        true
    }

    pub fn get_frame_inputs(&self, frame: u64) -> Vec<InputBits> {
        let mut inputs = Vec::new();
        if let Some(&local) = self.local_inputs.get(&frame) {
            inputs.push(local);
        }
        for player_id in 0..self.max_players {
            if let Some(remote) = self.remote_inputs.get(&player_id) {
                if let Some(&input) = remote.get(&frame) {
                    inputs.push(input);
                }
            }
        }
        inputs
    }

    pub fn cleanup_old_frames(&mut self, before_frame: u64) {
        self.local_inputs.retain(|&f, _| f >= before_frame);
        for inputs in self.remote_inputs.values_mut() {
            inputs.retain(|&f, _| f >= before_frame);
        }
    }

    pub fn ready_frames_after(&self, after_frame: u64) -> Vec<u64> {
        let mut frames: Vec<u64> = self
            .local_inputs
            .keys()
            .filter(|&&f| f > after_frame && self.is_frame_ready(f))
            .copied()
            .collect();
        frames.sort();
        frames
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_bits() {
        let mut bits = InputBits::new();
        bits.set(0, true);
        bits.set(5, true);
        bits.set(10, true);
        assert!(bits.get(0));
        assert!(bits.get(5));
        assert!(bits.get(10));
        assert!(!bits.get(1));
        bits.set(0, false);
        assert!(!bits.get(0));
    }

    #[test]
    fn test_lockstep_basic() {
        let mut state = LockstepState::new(2, 2);
        let input = InputBits::new();
        state.advance_frame(input);
        state.add_remote_input(1, 0, input);
        assert!(state.is_frame_ready(0));
    }

    #[test]
    fn test_lockstep_frame_not_ready() {
        let state = LockstepState::new(2, 2);
        assert!(!state.is_frame_ready(0));
    }

    #[test]
    fn test_lockstep_missing_remote() {
        let mut state = LockstepState::new(2, 2);
        state.add_remote_input(1, 0, InputBits::new());
        state.advance_frame(InputBits::new());
        state.advance_frame(InputBits::new());
        assert!(!state.is_frame_ready(1));
    }

    #[test]
    fn test_cleanup_old_frames() {
        let mut state = LockstepState::new(2, 2);
        for i in 0..10 {
            state.advance_frame(InputBits::new());
            state.add_remote_input(1, i, InputBits::new());
        }
        state.cleanup_old_frames(5);
        assert!(!state.local_inputs.contains_key(&4));
        assert!(state.local_inputs.contains_key(&5));
    }

    #[test]
    fn test_ready_frames_after() {
        let mut state = LockstepState::new(2, 1);
        for _ in 0..5 {
            state.advance_frame(InputBits::new());
        }
        let ready = state.ready_frames_after(0);
        assert_eq!(ready.len(), 4);
    }
}
