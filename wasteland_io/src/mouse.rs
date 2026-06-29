#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct MousePosition {
    pub x: f32,
    pub y: f32,
    pub delta_x: f32,
    pub delta_y: f32,
    pub wheel: f32,
    pub wheel_h: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Button4,
    Button5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    Released,
    Pressed,
    DoubleClicked,
    Held,
}

#[derive(Debug, Clone, Copy)]
pub struct MouseEvent {
    pub button: MouseButton,
    pub state: ButtonState,
    pub position: MousePosition,
    pub timestamp_ms: u64,
}

pub struct MouseState {
    pub position: MousePosition,
    buttons: [ButtonState; 5],
    events: Vec<MouseEvent>,
    frame_events: Vec<MouseEvent>,
    sensitivity: f32,
}

impl MouseState {
    pub fn new() -> Self {
        MouseState {
            position: MousePosition::default(),
            buttons: [ButtonState::Released; 5],
            events: Vec::with_capacity(32),
            frame_events: Vec::with_capacity(32),
            sensitivity: 1.0,
        }
    }

    pub fn set_sensitivity(&mut self, sens: f32) {
        self.sensitivity = sens;
    }

    pub fn move_cursor(&mut self, x: f32, y: f32) {
        self.position.delta_x = (x - self.position.x) * self.sensitivity;
        self.position.delta_y = (y - self.position.y) * self.sensitivity;
        self.position.x = x;
        self.position.y = y;
    }

    pub fn move_relative(&mut self, dx: f32, dy: f32) {
        self.position.delta_x = dx * self.sensitivity;
        self.position.delta_y = dy * self.sensitivity;
        self.position.x += self.position.delta_x;
        self.position.y += self.position.delta_y;
    }

    pub fn process_event(&mut self, event: MouseEvent) {
        let idx = self.button_index(event.button);
        let prev = self.buttons[idx];
        self.buttons[idx] = event.state;
        self.position = event.position;
        if prev != event.state {
            self.events.push(event);
            self.frame_events.push(event);
        }
    }

    fn button_index(&self, button: MouseButton) -> usize {
        match button {
            MouseButton::Left => 0,
            MouseButton::Right => 1,
            MouseButton::Middle => 2,
            MouseButton::Button4 => 3,
            MouseButton::Button5 => 4,
        }
    }

    pub fn is_pressed(&self, button: MouseButton) -> bool {
        matches!(
            self.buttons[self.button_index(button)],
            ButtonState::Pressed | ButtonState::DoubleClicked | ButtonState::Held
        )
    }

    pub fn just_pressed(&self, button: MouseButton) -> bool {
        self.frame_events.iter().any(|e| {
            e.button == button
                && matches!(e.state, ButtonState::Pressed | ButtonState::DoubleClicked)
        })
    }

    pub fn just_released(&self, button: MouseButton) -> bool {
        self.frame_events.iter().any(|e| e.button == button && e.state == ButtonState::Released)
    }

    pub fn clear_frame(&mut self) {
        self.position.delta_x = 0.0;
        self.position.delta_y = 0.0;
        self.position.wheel = 0.0;
        self.position.wheel_h = 0.0;
        self.frame_events.clear();
    }

    pub fn scroll(&mut self, wheel: f32, wheel_h: f32) {
        self.position.wheel = wheel;
        self.position.wheel_h = wheel_h;
    }
}

impl Default for MouseState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_event(button: MouseButton, state: ButtonState) -> MouseEvent {
        MouseEvent { button, state, position: MousePosition::default(), timestamp_ms: 0 }
    }

    #[test]
    fn test_mouse_press_release() {
        let mut ms = MouseState::new();
        ms.process_event(make_event(MouseButton::Left, ButtonState::Pressed));
        assert!(ms.is_pressed(MouseButton::Left));
        assert!(ms.just_pressed(MouseButton::Left));
        ms.clear_frame();
        ms.process_event(make_event(MouseButton::Left, ButtonState::Released));
        assert!(!ms.is_pressed(MouseButton::Left));
        assert!(ms.just_released(MouseButton::Left));
    }

    #[test]
    fn test_mouse_movement() {
        let mut ms = MouseState::new();
        ms.move_cursor(100.0, 200.0);
        assert_eq!(ms.position.x, 100.0);
        ms.move_cursor(150.0, 250.0);
        assert_eq!(ms.position.delta_x, 50.0);
        assert_eq!(ms.position.delta_y, 50.0);
    }

    #[test]
    fn test_mouse_sensitivity() {
        let mut ms = MouseState::new();
        ms.set_sensitivity(0.5);
        ms.move_relative(100.0, 0.0);
        assert_eq!(ms.position.delta_x, 50.0);
    }

    #[test]
    fn test_mouse_scroll() {
        let mut ms = MouseState::new();
        ms.scroll(3.0, 0.0);
        assert_eq!(ms.position.wheel, 3.0);
        ms.clear_frame();
        assert_eq!(ms.position.wheel, 0.0);
    }

    #[test]
    fn test_mouse_all_buttons() {
        let mut ms = MouseState::new();
        for btn in &[
            MouseButton::Left,
            MouseButton::Right,
            MouseButton::Middle,
            MouseButton::Button4,
            MouseButton::Button5,
        ] {
            ms.process_event(make_event(*btn, ButtonState::Pressed));
            assert!(ms.is_pressed(*btn));
        }
    }
}
