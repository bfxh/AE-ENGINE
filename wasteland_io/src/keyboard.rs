use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Key {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
    Escape,
    Tab,
    CapsLock,
    ShiftL,
    ShiftR,
    CtrlL,
    CtrlR,
    AltL,
    AltR,
    Space,
    Enter,
    Backspace,
    Delete,
    Up,
    Down,
    Left,
    Right,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    Unknown(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyState {
    Released,
    Pressed,
    Held,
    Repeated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyboardEvent {
    pub key: Key,
    pub state: KeyState,
    pub modifiers: Modifiers,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

pub struct KeyboardState {
    keys: HashMap<Key, KeyState>,
    events: Vec<KeyboardEvent>,
    frame_events: Vec<KeyboardEvent>,
    max_events: usize,
}

impl KeyboardState {
    pub fn new() -> Self {
        KeyboardState {
            keys: HashMap::new(),
            events: Vec::with_capacity(64),
            frame_events: Vec::with_capacity(64),
            max_events: 256,
        }
    }

    pub fn process_event(&mut self, event: KeyboardEvent) {
        let prev = self.keys.get(&event.key).copied();
        self.keys.insert(event.key, event.state);
        if prev != Some(event.state) {
            self.events.push(event);
            self.frame_events.push(event);
        }
        if self.events.len() > self.max_events {
            self.events.drain(0..self.events.len() - self.max_events);
        }
    }

    pub fn is_pressed(&self, key: Key) -> bool {
        matches!(self.keys.get(&key), Some(KeyState::Pressed | KeyState::Held | KeyState::Repeated))
    }

    pub fn just_pressed(&self, key: Key) -> bool {
        self.frame_events.iter().any(|e| e.key == key && e.state == KeyState::Pressed)
    }

    pub fn just_released(&self, key: Key) -> bool {
        self.frame_events.iter().any(|e| e.key == key && e.state == KeyState::Released)
    }

    pub fn clear_frame(&mut self) {
        self.frame_events.clear();
    }

    pub fn held_keys(&self) -> Vec<Key> {
        self.keys
            .iter()
            .filter(|(_, s)| matches!(s, KeyState::Held | KeyState::Pressed | KeyState::Repeated))
            .map(|(k, _)| *k)
            .collect()
    }

    pub fn any_pressed(&self, keys: &[Key]) -> bool {
        keys.iter().any(|k| self.is_pressed(*k))
    }

    pub fn chord(&self, keys: &[Key]) -> bool {
        keys.iter().all(|k| self.is_pressed(*k))
    }
}

impl Default for KeyboardState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_event(key: Key, state: KeyState) -> KeyboardEvent {
        KeyboardEvent { key, state, modifiers: Modifiers::default(), timestamp_ms: 0 }
    }

    #[test]
    fn test_press_and_release() {
        let mut ks = KeyboardState::new();
        ks.process_event(make_event(Key::A, KeyState::Pressed));
        assert!(ks.is_pressed(Key::A));
        assert!(ks.just_pressed(Key::A));
        ks.clear_frame();
        ks.process_event(make_event(Key::A, KeyState::Released));
        assert!(!ks.is_pressed(Key::A));
        assert!(ks.just_released(Key::A));
    }

    #[test]
    fn test_chord() {
        let mut ks = KeyboardState::new();
        ks.process_event(make_event(Key::CtrlL, KeyState::Pressed));
        ks.process_event(make_event(Key::C, KeyState::Pressed));
        assert!(ks.chord(&[Key::CtrlL, Key::C]));
        assert!(!ks.chord(&[Key::CtrlL, Key::V]));
    }

    #[test]
    fn test_any_pressed() {
        let mut ks = KeyboardState::new();
        ks.process_event(make_event(Key::W, KeyState::Pressed));
        assert!(ks.any_pressed(&[Key::W, Key::A, Key::S, Key::D]));
        assert!(!ks.any_pressed(&[Key::A, Key::S, Key::D]));
    }

    #[test]
    fn test_held_keys() {
        let mut ks = KeyboardState::new();
        ks.process_event(make_event(Key::W, KeyState::Pressed));
        ks.process_event(make_event(Key::A, KeyState::Held));
        let held = ks.held_keys();
        assert_eq!(held.len(), 2);
    }

    #[test]
    fn test_clear_frame() {
        let mut ks = KeyboardState::new();
        ks.process_event(make_event(Key::Escape, KeyState::Pressed));
        assert!(ks.just_pressed(Key::Escape));
        ks.clear_frame();
        assert!(!ks.just_pressed(Key::Escape));
        assert!(ks.is_pressed(Key::Escape));
    }
}
