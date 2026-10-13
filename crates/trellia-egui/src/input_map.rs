//! egui 原始输入 → `FrameInput` 的映射（SPEC.md 11.2，ADR-005）。仅覆盖 SPEC.md 4.9 的 `Key` 枚举。

use trellia_core::{FrameInput, InputEvent, Key, MouseButton};

/// egui 按键 → Trellia 按键；不在 v1 `Key` 枚举中的键返回 `None`。
pub(crate) fn map_key(key: egui::Key) -> Option<Key> {
    use egui::Key as E;
    Some(match key {
        E::Escape => Key::Escape,
        E::Space => Key::Space,
        E::Enter => Key::Enter,
        E::F1 => Key::F1,
        E::F2 => Key::F2,
        E::F3 => Key::F3,
        E::Num1 => Key::Digit1,
        E::Num2 => Key::Digit2,
        E::Num3 => Key::Digit3,
        E::Num4 => Key::Digit4,
        E::Num5 => Key::Digit5,
        E::Num6 => Key::Digit6,
        E::Num7 => Key::Digit7,
        E::Num8 => Key::Digit8,
        E::Num9 => Key::Digit9,
        E::Num0 => Key::Digit0,
        E::ArrowUp => Key::ArrowUp,
        E::ArrowDown => Key::ArrowDown,
        E::ArrowLeft => Key::ArrowLeft,
        E::ArrowRight => Key::ArrowRight,
        _ => return None,
    })
}

/// egui 指针按键 → Trellia 鼠标按键。
pub(crate) fn map_button(button: egui::PointerButton) -> Option<MouseButton> {
    match button {
        egui::PointerButton::Primary => Some(MouseButton::Left),
        egui::PointerButton::Secondary => Some(MouseButton::Right),
        egui::PointerButton::Middle => Some(MouseButton::Middle),
        _ => None,
    }
}

/// 从本帧 egui 输入构造 `FrameInput`（物理像素坐标）。
pub(crate) fn collect_frame_input(ctx: &egui::Context, real_dt: f32) -> FrameInput {
    let ppp = ctx.pixels_per_point();
    let pointer_blocked = ctx.is_pointer_over_egui() || ctx.egui_is_using_pointer();
    let keyboard_blocked = ctx.egui_wants_keyboard_input();
    ctx.input(|i| {
        let size = i.content_rect().size() * ppp;
        let mut events = Vec::new();
        for event in &i.events {
            match event {
                egui::Event::Key {
                    key,
                    pressed,
                    repeat,
                    ..
                } if !repeat && !keyboard_blocked => {
                    if let Some(k) = map_key(*key) {
                        events.push(if *pressed {
                            InputEvent::KeyPressed(k)
                        } else {
                            InputEvent::KeyReleased(k)
                        });
                    }
                }
                egui::Event::PointerButton {
                    button, pressed, ..
                } => {
                    if let Some(b) = map_button(*button) {
                        if !*pressed {
                            events.push(InputEvent::MouseReleased(b));
                        } else if !pointer_blocked {
                            events.push(InputEvent::MousePressed(b));
                        }
                    }
                }
                _ => {}
            }
        }
        if i.viewport().close_requested() {
            events.push(InputEvent::CloseRequested);
        }
        FrameInput {
            real_dt,
            window_size_px: (size.x.round() as u32, size.y.round() as u32),
            mouse_pos_px: i.pointer.hover_pos().map(|p| (p.x * ppp, p.y * ppp)),
            events,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_v1_keys_only() {
        assert_eq!(map_key(egui::Key::Escape), Some(Key::Escape));
        assert_eq!(map_key(egui::Key::Num0), Some(Key::Digit0));
        assert_eq!(map_key(egui::Key::A), None);
        assert_eq!(
            map_button(egui::PointerButton::Secondary),
            Some(MouseButton::Right)
        );
    }

    #[test]
    fn collects_events_and_filters_repeats() {
        let ctx = egui::Context::default();
        let screen_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(800.0, 600.0),
        ));
        // egui 在首个 pass 结束后才记录根 UI 区域，之前一律视为“指针在 egui 上”；先空跑一帧。
        let warmup = egui::RawInput {
            screen_rect,
            ..egui::RawInput::default()
        };
        ctx.run_ui(warmup, |_| {}).textures_delta.clear();
        let raw = egui::RawInput {
            screen_rect,
            events: vec![
                egui::Event::PointerMoved(egui::pos2(10.0, 20.0)),
                egui::Event::PointerButton {
                    pos: egui::pos2(10.0, 20.0),
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
                egui::Event::Key {
                    key: egui::Key::Space,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                },
                egui::Event::Key {
                    key: egui::Key::Space,
                    physical_key: None,
                    pressed: true,
                    repeat: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            ..egui::RawInput::default()
        };
        let mut collected = None;
        let mut output = ctx.run_ui(raw, |ui| {
            collected = Some(collect_frame_input(ui.ctx(), 0.016));
        });
        output.textures_delta.clear();
        let input = collected.expect("run_ui 调用了闭包");
        assert_eq!(input.window_size_px, (800, 600));
        assert_eq!(input.mouse_pos_px, Some((10.0, 20.0)));
        assert_eq!(
            input.events,
            vec![
                InputEvent::MousePressed(MouseButton::Left),
                InputEvent::KeyPressed(Key::Space)
            ]
        );
    }
}
