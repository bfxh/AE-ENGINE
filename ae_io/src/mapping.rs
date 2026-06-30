use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionMap {
    pub name: String,
    pub bindings: Vec<InputBinding>,
    pub priority: i32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputBinding {
    Key(crate::keyboard::Key),
    MouseButton(crate::mouse::MouseButton),
    GamepadButton(crate::gamepad::GamepadButton),
    GamepadAxis(crate::gamepad::GamepadAxis, bool),
    MouseAxis { axis: MouseAxis, invert: bool },
    Combo(Vec<InputBinding>),
    Chord(Vec<InputBinding>),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MouseAxis {
    X,
    Y,
    Wheel,
}

#[derive(Debug, Clone)]
pub struct ActionState {
    pub value: f32,
    pub pressed: bool,
    pub just_pressed: bool,
    pub just_released: bool,
    pub held_duration_ms: f32,
}

impl Default for ActionState {
    fn default() -> Self {
        ActionState {
            value: 0.0,
            pressed: false,
            just_pressed: false,
            just_released: false,
            held_duration_ms: 0.0,
        }
    }
}

pub struct InputMapper {
    actions: HashMap<String, ActionMap>,
    action_states: HashMap<String, ActionState>,
    hold_timers: HashMap<String, f32>,
}

impl InputMapper {
    pub fn new() -> Self {
        InputMapper {
            actions: HashMap::new(),
            action_states: HashMap::new(),
            hold_timers: HashMap::new(),
        }
    }

    pub fn register_action(&mut self, action: ActionMap) {
        self.action_states.entry(action.name.clone()).or_default();
        self.hold_timers.entry(action.name.clone()).or_insert(0.0);
        self.actions.insert(action.name.clone(), action);
    }

    pub fn get_action(&self, name: &str) -> Option<&ActionState> {
        self.action_states.get(name)
    }

    pub fn is_pressed(&self, name: &str) -> bool {
        self.action_states.get(name).is_some_and(|s| s.pressed)
    }

    pub fn just_pressed(&self, name: &str) -> bool {
        self.action_states.get(name).is_some_and(|s| s.just_pressed)
    }

    pub fn just_released(&self, name: &str) -> bool {
        self.action_states.get(name).is_some_and(|s| s.just_released)
    }

    pub fn get_value(&self, name: &str) -> f32 {
        self.action_states.get(name).map_or(0.0, |s| s.value)
    }

    pub fn update(
        &mut self,
        keyboard: &crate::keyboard::KeyboardState,
        mouse: &crate::mouse::MouseState,
        _gamepads: &[crate::gamepad::GamepadState],
        delta_ms: f32,
    ) {
        let mut sorted_actions: Vec<&ActionMap> = self.actions.values().collect();
        sorted_actions.sort_by_key(|a| -a.priority);

        for action in &sorted_actions {
            if !action.enabled {
                continue;
            }
            let mut value = 0.0f32;
            let mut pressed = false;
            for binding in &action.bindings {
                let (v, p) = self.eval_binding(binding, keyboard, mouse);
                if v.abs() > value.abs() {
                    value = v;
                }
                pressed = pressed || p;
            }
            let state = self.action_states.get_mut(&action.name).unwrap();
            let prev_pressed = state.pressed;
            state.value = value;
            state.pressed = pressed;
            state.just_pressed = pressed && !prev_pressed;
            state.just_released = !pressed && prev_pressed;
            if pressed {
                let timer = self.hold_timers.get_mut(&action.name).unwrap();
                *timer += delta_ms;
                state.held_duration_ms = *timer;
            } else {
                let timer = self.hold_timers.get_mut(&action.name).unwrap();
                *timer = 0.0;
                state.held_duration_ms = 0.0;
            }
        }
    }

    fn eval_binding(
        &self,
        binding: &InputBinding,
        keyboard: &crate::keyboard::KeyboardState,
        mouse: &crate::mouse::MouseState,
    ) -> (f32, bool) {
        match binding {
            InputBinding::Key(key) => {
                let p = keyboard.is_pressed(*key);
                (if p { 1.0 } else { 0.0 }, p)
            },
            InputBinding::MouseButton(btn) => {
                let p = mouse.is_pressed(*btn);
                (if p { 1.0 } else { 0.0 }, p)
            },
            InputBinding::GamepadButton(_) => (0.0, false),
            InputBinding::GamepadAxis(_, _) => (0.0, false),
            InputBinding::MouseAxis { axis, invert } => {
                let v = match axis {
                    MouseAxis::X => mouse.position.delta_x,
                    MouseAxis::Y => mouse.position.delta_y,
                    MouseAxis::Wheel => mouse.position.wheel,
                };
                let v = if *invert { -v } else { v };
                (v, v.abs() > 0.001)
            },
            InputBinding::Combo(bindings) => {
                let all_pressed = bindings.iter().all(|b| self.eval_binding(b, keyboard, mouse).1);
                (if all_pressed { 1.0 } else { 0.0 }, all_pressed)
            },
            InputBinding::Chord(bindings) => {
                let all_pressed = bindings.iter().all(|b| {
                    if let InputBinding::Key(k) = b {
                        keyboard.just_pressed(*k)
                    } else {
                        self.eval_binding(b, keyboard, mouse).1
                    }
                });
                (if all_pressed { 1.0 } else { 0.0 }, all_pressed)
            },
        }
    }
}

impl Default for InputMapper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keyboard::{Key, KeyState, KeyboardEvent, KeyboardState, Modifiers};
    use crate::mouse::{ButtonState, MouseButton, MouseEvent, MousePosition, MouseState};

    fn make_key_event(key: Key, state: KeyState) -> KeyboardEvent {
        KeyboardEvent { key, state, modifiers: Modifiers::default(), timestamp_ms: 0 }
    }

    fn make_mouse_event(btn: MouseButton, state: ButtonState) -> MouseEvent {
        MouseEvent { button: btn, state, position: MousePosition::default(), timestamp_ms: 0 }
    }

    #[test]
    fn test_action_mapping_key() {
        let mut mapper = InputMapper::new();
        mapper.register_action(ActionMap {
            name: "jump".into(),
            bindings: vec![InputBinding::Key(Key::Space)],
            priority: 0,
            enabled: true,
        });
        let mut ks = KeyboardState::new();
        let ms = MouseState::new();
        ks.process_event(make_key_event(Key::Space, KeyState::Pressed));
        mapper.update(&ks, &ms, &[], 16.0);
        assert!(mapper.is_pressed("jump"));
        assert!(mapper.just_pressed("jump"));
    }

    #[test]
    fn test_action_mapping_mouse() {
        let mut mapper = InputMapper::new();
        mapper.register_action(ActionMap {
            name: "shoot".into(),
            bindings: vec![InputBinding::MouseButton(MouseButton::Left)],
            priority: 0,
            enabled: true,
        });
        let ks = KeyboardState::new();
        let mut ms = MouseState::new();
        ms.process_event(make_mouse_event(MouseButton::Left, ButtonState::Pressed));
        mapper.update(&ks, &ms, &[], 16.0);
        assert!(mapper.is_pressed("shoot"));
    }

    #[test]
    fn test_action_mapping_combo() {
        let mut mapper = InputMapper::new();
        mapper.register_action(ActionMap {
            name: "sprint".into(),
            bindings: vec![InputBinding::Combo(vec![
                InputBinding::Key(Key::ShiftL),
                InputBinding::Key(Key::W),
            ])],
            priority: 0,
            enabled: true,
        });
        let mut ks = KeyboardState::new();
        let ms = MouseState::new();
        ks.process_event(make_key_event(Key::ShiftL, KeyState::Pressed));
        ks.process_event(make_key_event(Key::W, KeyState::Pressed));
        mapper.update(&ks, &ms, &[], 16.0);
        assert!(mapper.is_pressed("sprint"));
    }

    #[test]
    fn test_action_mapping_priority() {
        let mut mapper = InputMapper::new();
        mapper.register_action(ActionMap {
            name: "high_prio".into(),
            bindings: vec![InputBinding::Key(Key::E)],
            priority: 10,
            enabled: true,
        });
        mapper.register_action(ActionMap {
            name: "low_prio".into(),
            bindings: vec![InputBinding::Key(Key::E)],
            priority: 0,
            enabled: false,
        });
        let mut ks = KeyboardState::new();
        let ms = MouseState::new();
        ks.process_event(make_key_event(Key::E, KeyState::Pressed));
        mapper.update(&ks, &ms, &[], 16.0);
        assert!(mapper.is_pressed("high_prio"));
        assert!(!mapper.is_pressed("low_prio"));
    }
}
