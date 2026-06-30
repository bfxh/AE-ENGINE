//! 输入后端抽象：设备轮询与事件采集

use crate::gamepad::{GamepadAxis, GamepadButton, GamepadState};
use crate::keyboard::{Key, KeyState, KeyboardState, Modifiers};
use crate::mouse::{MouseButton, MouseState};

/// 输入事件（从窗口系统采集）
#[derive(Debug, Clone)]
pub enum InputEvent {
    /// 键盘按键
    Keyboard { key: Key, state: KeyState, modifiers: Modifiers, timestamp_ms: u64 },
    /// 鼠标移动
    MouseMove { x: f32, y: f32, dx: f32, dy: f32, timestamp_ms: u64 },
    /// 鼠标按键
    MouseButton { button: MouseButton, pressed: bool, x: f32, y: f32, timestamp_ms: u64 },
    /// 鼠标滚轮
    MouseWheel { delta_x: f32, delta_y: f32, timestamp_ms: u64 },
    /// 手柄按键
    GamepadButton { gamepad_id: u32, button: GamepadButton, pressed: bool, timestamp_ms: u64 },
    /// 手柄轴
    GamepadAxis { gamepad_id: u32, axis: GamepadAxis, value: f32, timestamp_ms: u64 },
    /// 手柄连接
    GamepadConnected { gamepad_id: u32, name: String },
    /// 手柄断开
    GamepadDisconnected { gamepad_id: u32 },
    /// 文本输入
    Text { text: String, timestamp_ms: u64 },
}

/// 输入后端 trait：由具体窗口系统实现
pub trait InputBackend {
    /// 轮询所有待处理的输入事件
    fn poll_events(&mut self) -> Vec<InputEvent>;

    /// 获取当前鼠标位置
    fn mouse_position(&self) -> (f32, f32);

    /// 是否有焦点（窗口是否激活）
    fn has_focus(&self) -> bool;

    /// 请求鼠标锁定（FPS 模式）
    fn set_cursor_grabbed(&mut self, grabbed: bool);

    /// 设置鼠标可见性
    fn set_cursor_visible(&mut self, visible: bool);

    /// 设置鼠标位置
    fn set_cursor_position(&mut self, x: f32, y: f32);
}

/// 输入收集器：统一管理所有输入设备状态
pub struct InputCollector {
    pub keyboard: KeyboardState,
    pub mouse: MouseState,
    pub gamepads: hashbrown::HashMap<u32, GamepadState>,
    text_input: String,
    frame_text: String,
    cursor_grabbed: bool,
}

impl Default for InputCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl InputCollector {
    pub fn new() -> Self {
        Self {
            keyboard: KeyboardState::new(),
            mouse: MouseState::new(),
            gamepads: hashbrown::HashMap::new(),
            text_input: String::new(),
            frame_text: String::new(),
            cursor_grabbed: false,
        }
    }

    /// 处理一个输入事件
    pub fn process_event(&mut self, event: &InputEvent) {
        match event {
            InputEvent::Keyboard { key, state, modifiers, timestamp_ms } => {
                self.keyboard.process_event(crate::keyboard::KeyboardEvent {
                    key: *key,
                    state: *state,
                    modifiers: *modifiers,
                    timestamp_ms: *timestamp_ms,
                });
            },
            InputEvent::MouseMove { x, y, .. } => {
                self.mouse.move_cursor(*x, *y);
            },
            InputEvent::MouseButton { button, pressed, x, y, .. } => {
                self.mouse.move_cursor(*x, *y);
                self.mouse.process_event(crate::mouse::MouseEvent {
                    button: *button,
                    state: if *pressed {
                        crate::mouse::ButtonState::Pressed
                    } else {
                        crate::mouse::ButtonState::Released
                    },
                    position: self.mouse.position,
                    timestamp_ms: 0,
                });
            },
            InputEvent::MouseWheel { delta_x, delta_y, .. } => {
                self.mouse.scroll(*delta_y, *delta_x);
            },
            InputEvent::GamepadButton { gamepad_id, button, pressed, .. } => {
                let gamepad = self
                    .gamepads
                    .entry(*gamepad_id)
                    .or_insert_with(|| GamepadState::new(*gamepad_id));
                gamepad.set_button(*button, *pressed);
            },
            InputEvent::GamepadAxis { gamepad_id, axis, value, .. } => {
                let gamepad = self
                    .gamepads
                    .entry(*gamepad_id)
                    .or_insert_with(|| GamepadState::new(*gamepad_id));
                gamepad.update_axis(*axis, *value);
            },
            InputEvent::GamepadConnected { gamepad_id, name: _ } => {
                self.gamepads.entry(*gamepad_id).or_insert_with(|| GamepadState::new(*gamepad_id));
            },
            InputEvent::GamepadDisconnected { gamepad_id } => {
                self.gamepads.remove(gamepad_id);
            },
            InputEvent::Text { text, .. } => {
                self.text_input.push_str(text);
                self.frame_text.push_str(text);
            },
        }
    }

    /// 批量处理事件
    pub fn process_events(&mut self, events: &[InputEvent]) {
        for event in events {
            self.process_event(event);
        }
    }

    /// 从后端轮询并处理事件
    pub fn poll_from_backend<B: InputBackend>(&mut self, backend: &mut B) {
        let events = backend.poll_events();
        self.process_events(&events);
    }

    /// 帧结束清理
    pub fn end_frame(&mut self) {
        self.keyboard.clear_frame();
        self.mouse.clear_frame();
        for gamepad in self.gamepads.values_mut() {
            gamepad.end_frame();
        }
        self.frame_text.clear();
    }

    /// 获取本帧文本输入
    pub fn frame_text(&self) -> &str {
        &self.frame_text
    }

    /// 获取所有文本输入
    pub fn text_input(&self) -> &str {
        &self.text_input
    }

    /// 清空文本输入缓冲
    pub fn clear_text_input(&mut self) {
        self.text_input.clear();
    }

    /// 设置鼠标锁定状态
    pub fn set_cursor_grabbed(&mut self, grabbed: bool) {
        self.cursor_grabbed = grabbed;
    }

    /// 鼠标是否被锁定
    pub fn is_cursor_grabbed(&self) -> bool {
        self.cursor_grabbed
    }

    /// 获取已连接的手柄数量
    pub fn gamepad_count(&self) -> usize {
        self.gamepads.len()
    }

    /// 获取手柄状态
    pub fn get_gamepad(&self, id: u32) -> Option<&GamepadState> {
        self.gamepads.get(&id)
    }

    /// 获取手柄状态（可变）
    pub fn get_gamepad_mut(&mut self, id: u32) -> Option<&mut GamepadState> {
        self.gamepads.get_mut(&id)
    }
}

/// 输入动作：抽象的输入映射目标
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputAction {
    MoveForward,
    MoveBackward,
    MoveLeft,
    MoveRight,
    Jump,
    Crouch,
    Sprint,
    Interact,
    Attack,
    Defend,
    UseItem,
    SwitchItem,
    OpenInventory,
    OpenMap,
    OpenMenu,
    Pause,
    Confirm,
    Cancel,
    Custom(u32),
}

/// 输入绑定：将物理输入映射到抽象动作
pub struct InputBindings {
    keyboard_bindings: hashbrown::HashMap<Key, InputAction>,
    mouse_bindings: hashbrown::HashMap<MouseButton, InputAction>,
    gamepad_bindings: hashbrown::HashMap<(u32, GamepadButton), InputAction>,
}

impl Default for InputBindings {
    fn default() -> Self {
        Self::new()
    }
}

impl InputBindings {
    pub fn new() -> Self {
        Self {
            keyboard_bindings: hashbrown::HashMap::new(),
            mouse_bindings: hashbrown::HashMap::new(),
            gamepad_bindings: hashbrown::HashMap::new(),
        }
    }

    /// 绑定键盘按键到动作
    pub fn bind_key(&mut self, key: Key, action: InputAction) {
        self.keyboard_bindings.insert(key, action);
    }

    /// 绑定鼠标按键到动作
    pub fn bind_mouse(&mut self, button: MouseButton, action: InputAction) {
        self.mouse_bindings.insert(button, action);
    }

    /// 绑定手柄按键到动作
    pub fn bind_gamepad(&mut self, gamepad_id: u32, button: GamepadButton, action: InputAction) {
        self.gamepad_bindings.insert((gamepad_id, button), action);
    }

    /// 设置默认 WASD 绑定
    pub fn set_default_fps_bindings(&mut self) {
        self.bind_key(Key::W, InputAction::MoveForward);
        self.bind_key(Key::S, InputAction::MoveBackward);
        self.bind_key(Key::A, InputAction::MoveLeft);
        self.bind_key(Key::D, InputAction::MoveRight);
        self.bind_key(Key::Space, InputAction::Jump);
        self.bind_key(Key::CtrlL, InputAction::Crouch);
        self.bind_key(Key::ShiftL, InputAction::Sprint);
        self.bind_key(Key::E, InputAction::Interact);
        self.bind_key(Key::Tab, InputAction::OpenInventory);
        self.bind_key(Key::M, InputAction::OpenMap);
        self.bind_key(Key::Escape, InputAction::OpenMenu);
        self.bind_mouse(MouseButton::Left, InputAction::Attack);
        self.bind_mouse(MouseButton::Right, InputAction::Defend);
    }

    /// 查询某个动作是否被触发（按下）
    pub fn is_action_pressed(&self, collector: &InputCollector, action: InputAction) -> bool {
        // 检查键盘
        for (&key, &a) in &self.keyboard_bindings {
            if a == action && collector.keyboard.is_pressed(key) {
                return true;
            }
        }
        // 检查鼠标
        for (&button, &a) in &self.mouse_bindings {
            if a == action && collector.mouse.is_pressed(button) {
                return true;
            }
        }
        // 检查手柄
        for (&(gamepad_id, button), &a) in &self.gamepad_bindings {
            if a == action {
                if let Some(gamepad) = collector.get_gamepad(gamepad_id) {
                    if gamepad.is_pressed(button) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// 查询某个动作是否在本帧刚按下
    pub fn is_action_just_pressed(&self, collector: &InputCollector, action: InputAction) -> bool {
        for (&key, &a) in &self.keyboard_bindings {
            if a == action && collector.keyboard.just_pressed(key) {
                return true;
            }
        }
        for (&button, &a) in &self.mouse_bindings {
            if a == action && collector.mouse.just_pressed(button) {
                return true;
            }
        }
        for (&(gamepad_id, button), &a) in &self.gamepad_bindings {
            if a == action {
                if let Some(gamepad) = collector.get_gamepad(gamepad_id) {
                    if gamepad.just_pressed(button) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// 获取所有当前激活的动作
    pub fn active_actions(&self, collector: &InputCollector) -> Vec<InputAction> {
        let mut actions = hashbrown::HashSet::new();
        for (&key, &a) in &self.keyboard_bindings {
            if collector.keyboard.is_pressed(key) {
                actions.insert(a);
            }
        }
        for (&button, &a) in &self.mouse_bindings {
            if collector.mouse.is_pressed(button) {
                actions.insert(a);
            }
        }
        for (&(gamepad_id, button), &a) in &self.gamepad_bindings {
            if let Some(gamepad) = collector.get_gamepad(gamepad_id) {
                if gamepad.is_pressed(button) {
                    actions.insert(a);
                }
            }
        }
        actions.into_iter().collect()
    }
}

/// 空输入后端（用于测试）
pub struct NullInputBackend {
    mouse_pos: (f32, f32),
    focused: bool,
}

impl Default for NullInputBackend {
    fn default() -> Self {
        Self { mouse_pos: (0.0, 0.0), focused: true }
    }
}

impl NullInputBackend {
    pub fn new() -> Self {
        Self::default()
    }
}

impl InputBackend for NullInputBackend {
    fn poll_events(&mut self) -> Vec<InputEvent> {
        Vec::new()
    }
    fn mouse_position(&self) -> (f32, f32) {
        self.mouse_pos
    }
    fn has_focus(&self) -> bool {
        self.focused
    }
    fn set_cursor_grabbed(&mut self, _grabbed: bool) {}
    fn set_cursor_visible(&mut self, _visible: bool) {}
    fn set_cursor_position(&mut self, x: f32, y: f32) {
        self.mouse_pos = (x, y);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_collector_creation() {
        let collector = InputCollector::new();
        assert_eq!(collector.gamepad_count(), 0);
        assert!(!collector.is_cursor_grabbed());
        assert!(collector.frame_text().is_empty());
    }

    #[test]
    fn input_collector_keyboard_event() {
        let mut collector = InputCollector::new();
        collector.process_event(&InputEvent::Keyboard {
            key: Key::A,
            state: KeyState::Pressed,
            modifiers: Modifiers::default(),
            timestamp_ms: 0,
        });
        assert!(collector.keyboard.is_pressed(Key::A));
    }

    #[test]
    fn input_collector_mouse_move() {
        let mut collector = InputCollector::new();
        collector.process_event(&InputEvent::MouseMove {
            x: 100.0,
            y: 200.0,
            dx: 10.0,
            dy: 5.0,
            timestamp_ms: 0,
        });
        let pos = (collector.mouse.position.x, collector.mouse.position.y);
        assert_eq!(pos, (100.0, 200.0));
    }

    #[test]
    fn input_collector_mouse_button() {
        let mut collector = InputCollector::new();
        collector.process_event(&InputEvent::MouseButton {
            button: MouseButton::Left,
            pressed: true,
            x: 50.0,
            y: 50.0,
            timestamp_ms: 0,
        });
        assert!(collector.mouse.is_pressed(MouseButton::Left));
    }

    #[test]
    fn input_collector_mouse_wheel() {
        let mut collector = InputCollector::new();
        collector.process_event(&InputEvent::MouseWheel {
            delta_x: 0.0,
            delta_y: 1.0,
            timestamp_ms: 0,
        });
        // wheel delta 应该被记录
        let (_dx, dy) = (collector.mouse.position.wheel_h, collector.mouse.position.wheel);
        assert_eq!(dy, 1.0);
    }

    #[test]
    fn input_collector_gamepad_connect() {
        let mut collector = InputCollector::new();
        collector.process_event(&InputEvent::GamepadConnected {
            gamepad_id: 0,
            name: "Test Pad".to_string(),
        });
        assert_eq!(collector.gamepad_count(), 1);
        assert!(collector.get_gamepad(0).is_some());
    }

    #[test]
    fn input_collector_gamepad_disconnect() {
        let mut collector = InputCollector::new();
        collector.process_event(&InputEvent::GamepadConnected {
            gamepad_id: 0,
            name: "Test Pad".to_string(),
        });
        assert_eq!(collector.gamepad_count(), 1);
        collector.process_event(&InputEvent::GamepadDisconnected { gamepad_id: 0 });
        assert_eq!(collector.gamepad_count(), 0);
    }

    #[test]
    fn input_collector_text_input() {
        let mut collector = InputCollector::new();
        collector.process_event(&InputEvent::Text { text: "hello".to_string(), timestamp_ms: 0 });
        assert_eq!(collector.frame_text(), "hello");
        assert_eq!(collector.text_input(), "hello");
        collector.end_frame();
        assert!(collector.frame_text().is_empty());
        assert_eq!(collector.text_input(), "hello");
    }

    #[test]
    fn input_collector_end_frame_clears() {
        let mut collector = InputCollector::new();
        collector.process_event(&InputEvent::Keyboard {
            key: Key::A,
            state: KeyState::Pressed,
            modifiers: Modifiers::default(),
            timestamp_ms: 0,
        });
        assert!(collector.keyboard.just_pressed(Key::A));
        collector.end_frame();
        assert!(!collector.keyboard.just_pressed(Key::A));
        // 但仍然 pressed
        assert!(collector.keyboard.is_pressed(Key::A));
    }

    #[test]
    fn input_bindings_default() {
        let bindings = InputBindings::default();
        let collector = InputCollector::new();
        assert!(!bindings.is_action_pressed(&collector, InputAction::MoveForward));
    }

    #[test]
    fn input_bindings_fps_layout() {
        let mut bindings = InputBindings::new();
        bindings.set_default_fps_bindings();
        let mut collector = InputCollector::new();
        collector.process_event(&InputEvent::Keyboard {
            key: Key::W,
            state: KeyState::Pressed,
            modifiers: Modifiers::default(),
            timestamp_ms: 0,
        });
        assert!(bindings.is_action_pressed(&collector, InputAction::MoveForward));
        assert!(!bindings.is_action_pressed(&collector, InputAction::MoveBackward));
    }

    #[test]
    fn input_bindings_just_pressed() {
        let mut bindings = InputBindings::new();
        bindings.bind_key(Key::Space, InputAction::Jump);
        let mut collector = InputCollector::new();
        collector.process_event(&InputEvent::Keyboard {
            key: Key::Space,
            state: KeyState::Pressed,
            modifiers: Modifiers::default(),
            timestamp_ms: 0,
        });
        assert!(bindings.is_action_just_pressed(&collector, InputAction::Jump));
        collector.end_frame();
        assert!(!bindings.is_action_just_pressed(&collector, InputAction::Jump));
    }

    #[test]
    fn input_bindings_active_actions() {
        let mut bindings = InputBindings::new();
        bindings.bind_key(Key::W, InputAction::MoveForward);
        bindings.bind_key(Key::Space, InputAction::Jump);
        let mut collector = InputCollector::new();
        collector.process_event(&InputEvent::Keyboard {
            key: Key::W,
            state: KeyState::Pressed,
            modifiers: Modifiers::default(),
            timestamp_ms: 0,
        });
        collector.process_event(&InputEvent::Keyboard {
            key: Key::Space,
            state: KeyState::Pressed,
            modifiers: Modifiers::default(),
            timestamp_ms: 0,
        });
        let actions = bindings.active_actions(&collector);
        assert!(actions.contains(&InputAction::MoveForward));
        assert!(actions.contains(&InputAction::Jump));
    }

    #[test]
    fn null_input_backend() {
        let mut backend = NullInputBackend::new();
        assert!(backend.has_focus());
        assert_eq!(backend.mouse_position(), (0.0, 0.0));
        let events = backend.poll_events();
        assert!(events.is_empty());
        backend.set_cursor_position(10.0, 20.0);
        assert_eq!(backend.mouse_position(), (10.0, 20.0));
    }

    #[test]
    fn input_collector_poll_from_backend() {
        let mut collector = InputCollector::new();
        let mut backend = NullInputBackend::new();
        collector.poll_from_backend(&mut backend);
        // 空后端不应产生事件
        assert!(!collector.keyboard.is_pressed(Key::A));
    }

    #[test]
    fn input_action_equality() {
        assert_eq!(InputAction::MoveForward, InputAction::MoveForward);
        assert_ne!(InputAction::MoveForward, InputAction::MoveBackward);
        assert_eq!(InputAction::Custom(42), InputAction::Custom(42));
        assert_ne!(InputAction::Custom(42), InputAction::Custom(43));
    }
}
