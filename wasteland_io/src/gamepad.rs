#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum GamepadButton {
    A,
    B,
    X,
    Y,
    LB,
    RB,
    Back,
    Start,
    L3,
    R3,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
    Guide,
    Unknown(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum GamepadAxis {
    LeftX,
    LeftY,
    RightX,
    RightY,
    L2,
    R2,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GamepadAxes {
    pub left_x: f32,
    pub left_y: f32,
    pub right_x: f32,
    pub right_y: f32,
    pub l2: f32,
    pub r2: f32,
}

impl GamepadAxes {
    pub fn set_axis(&mut self, axis: GamepadAxis, value: f32) {
        let v = value.clamp(-1.0, 1.0);
        match axis {
            GamepadAxis::LeftX => self.left_x = v,
            GamepadAxis::LeftY => self.left_y = v,
            GamepadAxis::RightX => self.right_x = v,
            GamepadAxis::RightY => self.right_y = v,
            GamepadAxis::L2 => self.l2 = v.clamp(0.0, 1.0),
            GamepadAxis::R2 => self.r2 = v.clamp(0.0, 1.0),
        }
    }

    pub fn get_axis(&self, axis: GamepadAxis) -> f32 {
        match axis {
            GamepadAxis::LeftX => self.left_x,
            GamepadAxis::LeftY => self.left_y,
            GamepadAxis::RightX => self.right_x,
            GamepadAxis::RightY => self.right_y,
            GamepadAxis::L2 => self.l2,
            GamepadAxis::R2 => self.r2,
        }
    }

    pub fn deadzone(&self, threshold: f32) -> GamepadAxes {
        let dz = |v: f32| if v.abs() < threshold { 0.0 } else { v };
        GamepadAxes {
            left_x: dz(self.left_x),
            left_y: dz(self.left_y),
            right_x: dz(self.right_x),
            right_y: dz(self.right_y),
            l2: dz(self.l2),
            r2: dz(self.r2),
        }
    }
}

pub struct GamepadState {
    pub id: u32,
    pub connected: bool,
    pub axes: GamepadAxes,
    buttons: Vec<bool>,
    prev_buttons: Vec<bool>,
    button_count: usize,
    vibration: (f32, f32),
}

impl GamepadState {
    pub fn new(id: u32) -> Self {
        GamepadState {
            id,
            connected: false,
            axes: GamepadAxes::default(),
            buttons: vec![false; 32],
            prev_buttons: vec![false; 32],
            button_count: 32,
            vibration: (0.0, 0.0),
        }
    }

    fn button_index(button: GamepadButton) -> usize {
        match button {
            GamepadButton::A => 0,
            GamepadButton::B => 1,
            GamepadButton::X => 2,
            GamepadButton::Y => 3,
            GamepadButton::LB => 4,
            GamepadButton::RB => 5,
            GamepadButton::Back => 6,
            GamepadButton::Start => 7,
            GamepadButton::L3 => 8,
            GamepadButton::R3 => 9,
            GamepadButton::DPadUp => 10,
            GamepadButton::DPadDown => 11,
            GamepadButton::DPadLeft => 12,
            GamepadButton::DPadRight => 13,
            GamepadButton::Guide => 14,
            GamepadButton::Unknown(n) => 15 + n as usize,
        }
    }

    pub fn set_button(&mut self, button: GamepadButton, pressed: bool) {
        let idx = Self::button_index(button);
        if idx < self.button_count {
            self.prev_buttons[idx] = self.buttons[idx];
            self.buttons[idx] = pressed;
        }
    }

    pub fn is_pressed(&self, button: GamepadButton) -> bool {
        let idx = Self::button_index(button);
        idx < self.button_count && self.buttons[idx]
    }

    pub fn just_pressed(&self, button: GamepadButton) -> bool {
        let idx = Self::button_index(button);
        idx < self.button_count && self.buttons[idx] && !self.prev_buttons[idx]
    }

    pub fn just_released(&self, button: GamepadButton) -> bool {
        let idx = Self::button_index(button);
        idx < self.button_count && !self.buttons[idx] && self.prev_buttons[idx]
    }

    pub fn update_axis(&mut self, axis: GamepadAxis, value: f32) {
        self.axes.set_axis(axis, value);
    }

    pub fn set_vibration(&mut self, left: f32, right: f32) {
        self.vibration = (left.clamp(0.0, 1.0), right.clamp(0.0, 1.0));
    }

    pub fn vibration(&self) -> (f32, f32) {
        self.vibration
    }

    pub fn end_frame(&mut self) {
        self.prev_buttons.copy_from_slice(&self.buttons);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_button_press_release() {
        let mut gs = GamepadState::new(0);
        gs.set_button(GamepadButton::A, true);
        assert!(gs.is_pressed(GamepadButton::A));
        assert!(gs.just_pressed(GamepadButton::A));
        gs.end_frame();
        gs.set_button(GamepadButton::A, false);
        assert!(!gs.is_pressed(GamepadButton::A));
        assert!(gs.just_released(GamepadButton::A));
    }

    #[test]
    fn test_axes() {
        let mut gs = GamepadState::new(0);
        gs.update_axis(GamepadAxis::LeftX, 0.5);
        gs.update_axis(GamepadAxis::LeftY, -0.3);
        assert_eq!(gs.axes.left_x, 0.5);
        assert_eq!(gs.axes.left_y, -0.3);
    }

    #[test]
    fn test_deadzone() {
        let mut axes = GamepadAxes::default();
        axes.set_axis(GamepadAxis::LeftX, 0.05);
        axes.set_axis(GamepadAxis::LeftY, 0.3);
        let dz = axes.deadzone(0.1);
        assert_eq!(dz.left_x, 0.0);
        assert_eq!(dz.left_y, 0.3);
    }

    #[test]
    fn test_vibration() {
        let mut gs = GamepadState::new(0);
        gs.set_vibration(0.5, 0.8);
        assert_eq!(gs.vibration(), (0.5, 0.8));
    }

    #[test]
    fn test_l2_r2_clamped() {
        let mut axes = GamepadAxes::default();
        axes.set_axis(GamepadAxis::L2, 1.5);
        axes.set_axis(GamepadAxis::R2, -0.5);
        assert_eq!(axes.l2, 1.0);
        assert_eq!(axes.r2, 0.0);
    }
}
