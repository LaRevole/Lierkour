//! Общая тема оформления.

use eframe::egui::{self, Color32, FontFamily, FontId, Stroke, TextStyle, Vec2};

pub const BG: Color32 = Color32::from_rgb(15, 17, 23);
pub const SURFACE: Color32 = Color32::from_rgb(24, 27, 36);
pub const SURFACE_HOVER: Color32 = Color32::from_rgb(32, 36, 48);
pub const BORDER: Color32 = Color32::from_rgb(43, 48, 62);
pub const ACCENT: Color32 = Color32::from_rgb(232, 168, 74);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(148, 156, 174);
pub const DANGER: Color32 = Color32::from_rgb(232, 98, 98);
pub const SUCCESS: Color32 = Color32::from_rgb(104, 199, 142);

pub fn apply(ctx: &egui::Context) {
    ctx.set_theme(egui::ThemePreference::Dark);

    let mut visuals = egui::Visuals::dark();

    visuals.panel_fill = BG;
    visuals.window_fill = SURFACE;
    visuals.faint_bg_color = Color32::from_rgb(21, 24, 32);
    visuals.extreme_bg_color = Color32::from_rgb(10, 12, 17);
    visuals.override_text_color = Some(Color32::from_rgb(232, 235, 242));
    visuals.hyperlink_color = ACCENT;
    visuals.selection.bg_fill = ACCENT.gamma_multiply(0.35);
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);

    visuals.widgets.noninteractive.bg_fill = SURFACE;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    visuals.widgets.inactive.bg_fill = SURFACE;
    visuals.widgets.inactive.weak_bg_fill = SURFACE;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
    visuals.widgets.hovered.bg_fill = SURFACE_HOVER;
    visuals.widgets.hovered.weak_bg_fill = SURFACE_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT.gamma_multiply(0.7));
    visuals.widgets.active.bg_fill = ACCENT.gamma_multiply(0.3);
    visuals.widgets.active.weak_bg_fill = ACCENT.gamma_multiply(0.3);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);

    ctx.set_visuals(visuals);

    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = Vec2::new(10.0, 8.0);
        style.spacing.button_padding = Vec2::new(12.0, 7.0);
        style.text_styles = [
            (
                TextStyle::Heading,
                FontId::new(26.0, FontFamily::Proportional),
            ),
            (TextStyle::Body, FontId::new(15.0, FontFamily::Proportional)),
            (
                TextStyle::Button,
                FontId::new(15.0, FontFamily::Proportional),
            ),
            (
                TextStyle::Small,
                FontId::new(12.0, FontFamily::Proportional),
            ),
            (
                TextStyle::Monospace,
                FontId::new(14.0, FontFamily::Monospace),
            ),
        ]
        .into();
    });
}
