//! 追加字体（游戏 spec E-1，ADR-010）：把一个 TTF / OTF 作为回退字体加入 egui 的比例与等宽字体族。
//!
//! `DrawList` 文字与 egui 控件共用同一个 egui 上下文，因此两者都能显示该字体中的字形（如中文）。

use std::sync::Arc;

/// 追加字体在 `FontDefinitions` 中的名字。
pub const CJK_FONT_NAME: &str = "trellia-extra";

/// 在 egui 默认字体之后追加 `bytes` 作为回退字体（默认字体缺字时使用）。
#[must_use]
pub fn font_definitions(bytes: Vec<u8>) -> egui::FontDefinitions {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        CJK_FONT_NAME.to_owned(),
        Arc::new(egui::FontData::from_owned(bytes)),
    );
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .push(CJK_FONT_NAME.to_owned());
    }
    fonts
}

/// 后端在创建窗口时调用：安装追加字体。
pub(crate) fn setup_fonts(ctx: &egui::Context, bytes: Vec<u8>) {
    ctx.set_fonts(font_definitions(bytes));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extra_font_is_fallback_in_both_families() {
        let defaults = egui::FontDefinitions::default();
        let fonts = font_definitions(vec![1, 2, 3]);
        assert!(fonts.font_data.contains_key(CJK_FONT_NAME));
        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            let list = &fonts.families[&family];
            assert_eq!(
                list.last().map(String::as_str),
                Some(CJK_FONT_NAME),
                "追加在末尾作为回退"
            );
            assert_eq!(
                list.len(),
                defaults.families[&family].len() + 1,
                "默认字体保持不变"
            );
        }
    }
}
