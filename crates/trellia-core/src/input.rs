//! 输入状态与帧 IO（SPEC.md 4.9）。

use glam::Vec2;

/// 鼠标按键。对应 UC-40。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum MouseButton {
    /// 左键。
    Left,
    /// 右键。
    Right,
    /// 中键。
    Middle,
}

/// v1 支持的键盘按键（UC-41）。新增需引用用例。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Key {
    /// Esc（暂停）。
    Escape,
    /// 空格（继续）。
    Space,
    /// 回车。
    Enter,
    /// F1（调试面板）。
    F1,
    /// F2。
    F2,
    /// F3。
    F3,
    /// 数字键 1（快捷选卡）。
    Digit1,
    /// 数字键 2。
    Digit2,
    /// 数字键 3。
    Digit3,
    /// 数字键 4。
    Digit4,
    /// 数字键 5。
    Digit5,
    /// 数字键 6。
    Digit6,
    /// 数字键 7。
    Digit7,
    /// 数字键 8。
    Digit8,
    /// 数字键 9。
    Digit9,
    /// 数字键 0。
    Digit0,
    /// 方向键上。
    ArrowUp,
    /// 方向键下。
    ArrowDown,
    /// 方向键左。
    ArrowLeft,
    /// 方向键右。
    ArrowRight,
}

/// 每帧开始时快照一次的输入状态，一帧内所有系统看到一致的值（UC-40、UC-41、UC-42）。
///
/// `just_pressed` / `just_released` 仅在事件所在帧的 `Stage::First` 之后到下一帧 `Stage::First`
/// 之前为真。多个 FixedUpdate tick 共享同一帧输入快照。
#[derive(Clone, Debug, Default)]
pub struct Input {
    /// 鼠标在逻辑坐标系中的位置；光标不在窗口内时为 `None`。
    ///
    /// App 先写入物理像素值，`GfxPlugin` 在 `Stage::First` 中改写为逻辑坐标（ADR-003）。
    pub mouse_pos: Option<Vec2>,
    /// 本帧 `FrameInput` 的原始鼠标物理像素坐标（ADR-003）。
    pub mouse_pos_px: Option<Vec2>,
    /// 本帧 `FrameInput` 的窗口物理像素尺寸（ADR-003）。
    pub window_size_px: Vec2,
    mouse_down: u8,
    mouse_pressed: u8,
    mouse_released: u8,
    key_down: u32,
    key_pressed: u32,
    key_released: u32,
}

fn mouse_bit(b: MouseButton) -> u8 {
    1 << (b as u8)
}

fn key_bit(k: Key) -> u32 {
    1 << (k as u32)
}

impl Input {
    /// 按键当前是否按住。
    #[must_use]
    pub fn mouse_down(&self, b: MouseButton) -> bool {
        self.mouse_down & mouse_bit(b) != 0
    }

    /// 按键是否在本帧刚按下。
    #[must_use]
    pub fn mouse_just_pressed(&self, b: MouseButton) -> bool {
        self.mouse_pressed & mouse_bit(b) != 0
    }

    /// 按键是否在本帧刚释放。
    #[must_use]
    pub fn mouse_just_released(&self, b: MouseButton) -> bool {
        self.mouse_released & mouse_bit(b) != 0
    }

    /// 键当前是否按住。
    #[must_use]
    pub fn key_down(&self, k: Key) -> bool {
        self.key_down & key_bit(k) != 0
    }

    /// 键是否在本帧刚按下。
    #[must_use]
    pub fn key_just_pressed(&self, k: Key) -> bool {
        self.key_pressed & key_bit(k) != 0
    }

    /// 键是否在本帧刚释放。
    #[must_use]
    pub fn key_just_released(&self, k: Key) -> bool {
        self.key_released & key_bit(k) != 0
    }

    /// 用本帧 `FrameInput` 写入快照。返回本帧是否收到 `CloseRequested`。
    pub(crate) fn begin_frame(&mut self, input: &FrameInput) -> bool {
        let (w, h) = input.window_size_px;
        let size = Vec2::new(w as f32, h as f32);
        self.window_size_px = size;
        self.mouse_pos_px = input.mouse_pos_px.map(|(x, y)| Vec2::new(x, y));
        self.mouse_pos = self.mouse_pos_px;
        self.mouse_pressed = 0;
        self.mouse_released = 0;
        self.key_pressed = 0;
        self.key_released = 0;
        let mut close = false;
        for event in &input.events {
            match *event {
                InputEvent::MousePressed(b) => {
                    if !self.mouse_down(b) {
                        self.mouse_pressed |= mouse_bit(b);
                    }
                    self.mouse_down |= mouse_bit(b);
                }
                InputEvent::MouseReleased(b) => {
                    if self.mouse_down(b) {
                        self.mouse_released |= mouse_bit(b);
                    }
                    self.mouse_down &= !mouse_bit(b);
                }
                InputEvent::KeyPressed(k) => {
                    if !self.key_down(k) {
                        self.key_pressed |= key_bit(k);
                    }
                    self.key_down |= key_bit(k);
                }
                InputEvent::KeyReleased(k) => {
                    if self.key_down(k) {
                        self.key_released |= key_bit(k);
                    }
                    self.key_down &= !key_bit(k);
                }
                InputEvent::CloseRequested => close = true,
            }
        }
        close
    }
}

/// 后端每帧传入 `App::frame` 的输入。坐标为物理窗口像素。对应 UC-10、UC-40、UC-42。
#[derive(Clone, Debug, Default)]
pub struct FrameInput {
    /// 本帧真实秒数。
    pub real_dt: f32,
    /// 窗口物理像素尺寸。
    pub window_size_px: (u32, u32),
    /// 鼠标物理像素坐标；光标不在窗口内时为 `None`。
    pub mouse_pos_px: Option<(f32, f32)>,
    /// 本帧按发生顺序排列的输入事件。
    pub events: Vec<InputEvent>,
}

/// 原始输入事件。对应 UC-40、UC-41。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputEvent {
    /// 鼠标按下。
    MousePressed(MouseButton),
    /// 鼠标释放。
    MouseReleased(MouseButton),
    /// 键按下（后端应过滤系统按键重复）。
    KeyPressed(Key),
    /// 键释放。
    KeyReleased(Key),
    /// 窗口请求关闭。
    CloseRequested,
}

/// `App::frame` 的产出，由后端消费。对应 UC-60、UC-62。
///
/// `DrawList` 与音频请求以资源形式存放在 World 中，后端自行读取（避免 core 依赖 gfx / audio）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FrameOutput {
    /// 是否请求退出程序（见 `AppExit`）。
    pub exit_requested: bool,
}
