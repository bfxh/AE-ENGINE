use godot::prelude::*;

use ae_io::feedback::ForceFeedbackDevice;
use ae_io::gamepad::{GamepadAxis, GamepadButton, GamepadState};
use ae_io::keyboard::{Key, KeyState, KeyboardEvent, KeyboardState, Modifiers};
use ae_io::mapping::{ActionMap, InputBinding, InputMapper};
use ae_io::mouse::{ButtonState, MouseButton, MouseEvent, MousePosition, MouseState};

#[derive(GodotClass)]
#[class(base=Node)]
pub(crate) struct WastelandIO {
    #[var]
    mouse_sensitivity: f32,
    #[var]
    gamepad_deadzone: f32,
    #[var]
    force_feedback_enabled: bool,

    keyboard: KeyboardState,
    mouse: MouseState,
    gamepad: GamepadState,
    mapping: InputMapper,
    feedback: ForceFeedbackDevice,
    input_events: i64,
    active_actions: i64,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandIO {
    fn init(base: Base<Node>) -> Self {
        Self {
            mouse_sensitivity: 1.0,
            gamepad_deadzone: 0.15,
            force_feedback_enabled: true,
            keyboard: KeyboardState::new(),
            mouse: MouseState::new(),
            gamepad: GamepadState::new(0),
            mapping: InputMapper::new(),
            feedback: ForceFeedbackDevice::new(),
            input_events: 0,
            active_actions: 0,
            base,
        }
    }
}

#[godot_api]
impl WastelandIO {
    #[func]
    fn process_key_event(&mut self, key_name: GString, pressed: bool, held: bool, repeated: bool) {
        let key = match key_name.to_string().to_uppercase().as_str() {
            "A" => Key::A,
            "B" => Key::B,
            "C" => Key::C,
            "D" => Key::D,
            "E" => Key::E,
            "F" => Key::F,
            "G" => Key::G,
            "H" => Key::H,
            "I" => Key::I,
            "J" => Key::J,
            "K" => Key::K,
            "L" => Key::L,
            "M" => Key::M,
            "N" => Key::N,
            "O" => Key::O,
            "P" => Key::P,
            "Q" => Key::Q,
            "R" => Key::R,
            "S" => Key::S,
            "T" => Key::T,
            "U" => Key::U,
            "V" => Key::V,
            "W" => Key::W,
            "X" => Key::X,
            "Y" => Key::Y,
            "Z" => Key::Z,
            "SPACE" => Key::Space,
            "ENTER" => Key::Enter,
            "ESCAPE" => Key::Escape,
            "SHIFT" => Key::ShiftL,
            "CTRL" => Key::CtrlL,
            "ALT" => Key::AltL,
            "TAB" => Key::Tab,
            "BACKSPACE" => Key::Backspace,
            "DELETE" => Key::Delete,
            "UP" => Key::Up,
            "DOWN" => Key::Down,
            "LEFT" => Key::Left,
            "RIGHT" => Key::Right,
            "F1" => Key::F1,
            "F2" => Key::F2,
            "F3" => Key::F3,
            "F4" => Key::F4,
            "F5" => Key::F5,
            "F6" => Key::F6,
            "F7" => Key::F7,
            "F8" => Key::F8,
            "F9" => Key::F9,
            "F10" => Key::F10,
            "F11" => Key::F11,
            "F12" => Key::F12,
            _ => Key::Unknown(0),
        };
        let state = if repeated {
            KeyState::Repeated
        } else if held {
            KeyState::Held
        } else if pressed {
            KeyState::Pressed
        } else {
            KeyState::Released
        };
        let event = KeyboardEvent { key, state, modifiers: Modifiers::default(), timestamp_ms: 0 };
        self.keyboard.process_event(event);
        self.input_events += 1;
    }

    #[func]
    fn is_key_pressed(&self, key_name: GString) -> bool {
        let key = match key_name.to_string().to_uppercase().as_str() {
            "A" => Key::A,
            "W" => Key::W,
            "S" => Key::S,
            "D" => Key::D,
            "SPACE" => Key::Space,
            "SHIFT" => Key::ShiftL,
            "E" => Key::E,
            "Q" => Key::Q,
            _ => return false,
        };
        self.keyboard.is_pressed(key)
    }

    #[func]
    fn is_key_held(&self, key_name: GString) -> bool {
        let key = match key_name.to_string().to_uppercase().as_str() {
            "A" => Key::A,
            "W" => Key::W,
            "S" => Key::S,
            "D" => Key::D,
            "SPACE" => Key::Space,
            "SHIFT" => Key::ShiftL,
            "E" => Key::E,
            "Q" => Key::Q,
            _ => return false,
        };
        self.keyboard.is_pressed(key)
    }

    #[func]
    fn process_mouse_event(
        &mut self,
        x: f32,
        y: f32,
        dx: f32,
        dy: f32,
        button: i64,
        pressed: bool,
    ) {
        let btn = match button {
            0 => MouseButton::Left,
            1 => MouseButton::Right,
            2 => MouseButton::Middle,
            3 => MouseButton::Button4,
            4 => MouseButton::Button5,
            _ => return,
        };
        let position = MousePosition { x, y, delta_x: dx, delta_y: dy, wheel: 0.0, wheel_h: 0.0 };
        let event = MouseEvent {
            button: btn,
            state: if pressed { ButtonState::Pressed } else { ButtonState::Released },
            position,
            timestamp_ms: 0,
        };
        self.mouse.process_event(event);
        self.input_events += 1;
    }

    #[func]
    fn get_mouse_position(&self) -> Vector2 {
        Vector2::new(self.mouse.position.x, self.mouse.position.y)
    }

    #[func]
    fn get_mouse_delta(&self) -> Vector2 {
        Vector2::new(
            self.mouse.position.delta_x * self.mouse_sensitivity,
            self.mouse.position.delta_y * self.mouse_sensitivity,
        )
    }

    #[func]
    fn process_gamepad_axis(&mut self, axis: i64, value: f32) {
        let a = match axis {
            0 => GamepadAxis::LeftX,
            1 => GamepadAxis::LeftY,
            2 => GamepadAxis::RightX,
            3 => GamepadAxis::RightY,
            4 => GamepadAxis::L2,
            5 => GamepadAxis::R2,
            _ => return,
        };
        let clamped = if value.abs() < self.gamepad_deadzone { 0.0 } else { value };
        self.gamepad.update_axis(a, clamped);
    }

    #[func]
    fn process_gamepad_button(&mut self, button: i64, pressed: bool) {
        let btn = match button {
            0 => GamepadButton::A,
            1 => GamepadButton::B,
            2 => GamepadButton::X,
            3 => GamepadButton::Y,
            4 => GamepadButton::LB,
            5 => GamepadButton::RB,
            6 => GamepadButton::Back,
            7 => GamepadButton::Start,
            8 => GamepadButton::L3,
            9 => GamepadButton::R3,
            10 => GamepadButton::DPadUp,
            11 => GamepadButton::DPadDown,
            12 => GamepadButton::DPadLeft,
            13 => GamepadButton::DPadRight,
            14 => GamepadButton::Guide,
            _ => GamepadButton::Unknown(button as u32),
        };
        self.gamepad.set_button(btn, pressed);
    }

    #[func]
    fn map_action(&mut self, action_name: GString, key_name: GString) {
        let key = match key_name.to_string().to_uppercase().as_str() {
            "A" => Key::A,
            "W" => Key::W,
            "S" => Key::S,
            "D" => Key::D,
            "SPACE" => Key::Space,
            "SHIFT" => Key::ShiftL,
            "E" => Key::E,
            "Q" => Key::Q,
            "ENTER" => Key::Enter,
            "ESCAPE" => Key::Escape,
            _ => Key::Unknown(0),
        };
        self.mapping.register_action(ActionMap {
            name: action_name.to_string(),
            bindings: vec![InputBinding::Key(key)],
            priority: 0,
            enabled: true,
        });
        self.active_actions += 1;
    }

    #[func]
    fn is_action_active(&self, action_name: GString) -> bool {
        self.mapping.is_pressed(&action_name.to_string())
    }

    #[func]
    fn update_mapping(&mut self, delta_ms: f32) {
        self.mapping.update(
            &self.keyboard,
            &self.mouse,
            std::slice::from_ref(&self.gamepad),
            delta_ms,
        );
    }

    #[func]
    fn set_force_feedback(&mut self, _motor: i64, strength: f32) {
        if self.force_feedback_enabled {
            self.feedback.set_master_gain(strength);
        }
    }

    #[func]
    fn stop_feedback(&mut self) {
        self.feedback.stop_all();
    }

    #[func]
    fn get_stats(&self) -> Dictionary<Variant, Variant> {
        dict! {
            "input_events" => self.input_events,
            "active_actions" => self.active_actions,
            "mouse_sensitivity" => self.mouse_sensitivity,
            "gamepad_deadzone" => self.gamepad_deadzone,
            "force_feedback_enabled" => self.force_feedback_enabled,
        }
    }
}
