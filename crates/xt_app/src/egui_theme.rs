#![allow(dead_code)]

use egui::{
    Color32, FontDefinitions, FontFamily, FontId, Margin, Rounding, Style, TextStyle, Vec2,
    Visuals,
};

// Mappings from xt_app/assets/main.css color tokens.
const LINE: Color32 = Color32::from_rgb(0xb4, 0xb4, 0xb4);
const BG: Color32 = Color32::from_rgb(0xef, 0xef, 0xef);
const PANEL: Color32 = Color32::from_rgb(0xf8, 0xf8, 0xf8);
const TEXT: Color32 = Color32::from_rgb(0x1c, 0x1c, 0x1c);
const PRIMARY: Color32 = Color32::from_rgb(0x2a, 0x6f, 0xd6);
const DANGER: Color32 = Color32::from_rgb(0x9e, 0x1b, 0x1b);
const BG_ROOT: Color32 = Color32::from_rgb(0xd9, 0xd9, 0xd9);
const HOVER: Color32 = Color32::from_rgb(0xdf, 0xee, 0xff);
const SUCCESS: Color32 = Color32::from_rgb(0x1c, 0xa7, 0x4c);

pub fn base_font_definitions() -> FontDefinitions {
    FontDefinitions::default()
}

pub fn base_style() -> Style {
    let mut style = Style::default();
    style.spacing.item_spacing = Vec2::new(8.0, 8.0);
    style.spacing.button_padding = Vec2::new(12.0, 6.0);
    style.spacing.window_margin = Margin::same(8.0);
    style.spacing.menu_margin = Margin::same(8.0);
    style.spacing.interact_size = Vec2::new(28.0, 28.0);
    style.spacing.slider_width = 150.0;
    style.visuals = base_visuals();
    style.text_styles = [
        (
            TextStyle::Heading,
            FontId::new(16.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Body,
            FontId::new(12.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Monospace,
            FontId::new(12.0, FontFamily::Monospace),
        ),
        (
            TextStyle::Button,
            FontId::new(12.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Small,
            FontId::new(11.0, FontFamily::Proportional),
        ),
    ]
    .into();
    style
}

pub fn base_visuals() -> Visuals {
    let mut visuals = Visuals::light();
    visuals.override_text_color = Some(TEXT);
    visuals.window_fill = BG;
    visuals.panel_fill = PANEL;
    visuals.faint_bg_color = Color32::from_rgb(0xfa, 0xfa, 0xfa);
    visuals.extreme_bg_color = BG_ROOT;
    visuals.widgets.noninteractive.bg_fill = PANEL;
    visuals.widgets.noninteractive.bg_stroke.color = LINE;
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(0xfa, 0xfa, 0xfa);
    visuals.widgets.inactive.bg_stroke.color = LINE;
    visuals.widgets.hovered.bg_fill = HOVER;
    visuals.widgets.hovered.bg_stroke.color = PRIMARY;
    visuals.widgets.active.bg_fill = Color32::from_rgb(0xd8, 0xe8, 0xff);
    visuals.widgets.active.bg_stroke.color = PRIMARY;
    visuals.selection.bg_fill = PRIMARY;
    visuals.selection.stroke.color = Color32::WHITE;
    visuals.window_stroke.color = LINE;
    visuals.window_shadow.extrusion = 0.0;
    visuals.window_rounding = Rounding::same(4.0);
    visuals.popup_shadow.extrusion = 0.0;
    visuals.error_fg_color = DANGER;
    visuals.warn_fg_color = DANGER;
    visuals.success_fg_color = SUCCESS;
    visuals
}

pub fn apply_base_theme(ctx: &egui::Context) {
    ctx.set_fonts(base_font_definitions());
    ctx.set_style(base_style());
}
