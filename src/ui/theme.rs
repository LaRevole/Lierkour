//! Общая тема оформления: светлая и тёмная, адаптивная палитра цветов.

use eframe::egui::{self, Color32, FontFamily, FontId, Stroke, TextStyle, Vec2};

use crate::models::AppTheme;

#[allow(dead_code)]
pub const BG: Color32 = Color32::from_rgb(15, 17, 23);
#[allow(dead_code)]
pub const SURFACE: Color32 = Color32::from_rgb(24, 27, 36);
#[allow(dead_code)]
pub const SURFACE_HOVER: Color32 = Color32::from_rgb(32, 36, 48);
#[allow(dead_code)]
pub const BORDER: Color32 = Color32::from_rgb(43, 48, 62);
#[allow(dead_code)]
pub const ACCENT: Color32 = Color32::from_rgb(232, 168, 74);
#[allow(dead_code)]
pub const TEXT_MUTED: Color32 = Color32::from_rgb(148, 156, 174);
#[allow(dead_code)]
pub const DANGER: Color32 = Color32::from_rgb(232, 98, 98);
#[allow(dead_code)]
pub const SUCCESS: Color32 = Color32::from_rgb(104, 199, 142);

/// Динамическая палитра цветов, автоматически адаптирующаяся под текущую тему (светлую/тёмную).
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub struct ThemeColors {
    pub bg: Color32,
    pub surface: Color32,
    pub surface_hover: Color32,
    pub card_bg: Color32,
    pub card_hover: Color32,
    pub card_stroke: Color32,
    pub border: Color32,
    pub border_subtle: Color32,
    pub text: Color32,
    pub text_muted: Color32,
    pub accent: Color32,
    pub accent_hover: Color32,
    pub danger: Color32,
    pub success: Color32,
    pub is_dark: bool,
}

pub fn colors(ctx: &egui::Context) -> ThemeColors {
    let dark = matches!(ctx.theme(), egui::Theme::Dark);
    if dark {
        ThemeColors {
            bg: Color32::from_rgb(15, 17, 23),
            surface: Color32::from_rgb(24, 27, 36),
            surface_hover: Color32::from_rgb(32, 36, 48),
            card_bg: Color32::from_rgb(24, 27, 36),
            card_hover: Color32::from_rgb(34, 39, 52),
            card_stroke: Color32::from_rgb(43, 48, 62),
            border: Color32::from_rgb(43, 48, 62),
            border_subtle: Color32::from_rgb(29, 33, 43),
            text: Color32::from_rgb(241, 245, 249),
            text_muted: Color32::from_rgb(148, 163, 184),
            accent: Color32::from_rgb(232, 168, 74),
            accent_hover: Color32::from_rgb(245, 185, 96),
            danger: Color32::from_rgb(239, 68, 68),
            success: Color32::from_rgb(34, 197, 94),
            is_dark: true,
        }
    } else {
        ThemeColors {
            bg: Color32::from_rgb(243, 245, 249),
            surface: Color32::from_rgb(255, 255, 255),
            surface_hover: Color32::from_rgb(246, 248, 252),
            card_bg: Color32::from_rgb(255, 255, 255),
            card_hover: Color32::from_rgb(249, 251, 254),
            card_stroke: Color32::from_rgb(222, 228, 237),
            border: Color32::from_rgb(222, 228, 237),
            border_subtle: Color32::from_rgb(238, 242, 248),
            text: Color32::from_rgb(15, 23, 42),
            text_muted: Color32::from_rgb(100, 116, 139),
            accent: Color32::from_rgb(217, 119, 6),
            accent_hover: Color32::from_rgb(180, 83, 9),
            danger: Color32::from_rgb(220, 38, 38),
            success: Color32::from_rgb(22, 163, 74),
            is_dark: false,
        }
    }
}

pub fn apply(ctx: &egui::Context, theme: AppTheme) {
    match theme {
        AppTheme::Dark => apply_dark(ctx),
        AppTheme::Light => apply_light(ctx),
    }
}

fn apply_dark(ctx: &egui::Context) {
    ctx.set_theme(egui::ThemePreference::Dark);
    ctx.send_viewport_cmd(egui::ViewportCommand::SetTheme(egui::SystemTheme::Dark));

    let mut visuals = egui::Visuals::dark();

    let bg = Color32::from_rgb(15, 17, 23);
    let surface = Color32::from_rgb(24, 27, 36);
    let surface_hover = Color32::from_rgb(32, 36, 48);
    let border = Color32::from_rgb(43, 48, 62);
    let accent = Color32::from_rgb(232, 168, 74);
    let text = Color32::from_rgb(241, 245, 249);

    visuals.dark_mode = true;
    visuals.panel_fill = bg;
    visuals.window_fill = surface;
    visuals.faint_bg_color = Color32::from_rgb(20, 23, 31);
    visuals.extreme_bg_color = Color32::from_rgb(12, 14, 19);
    visuals.override_text_color = Some(text);
    visuals.hyperlink_color = accent;
    visuals.selection.bg_fill = accent.gamma_multiply(0.35);
    visuals.selection.stroke = Stroke::new(1.0, accent);

    let radius = egui::CornerRadius::same(8);
    visuals.window_corner_radius = egui::CornerRadius::same(12);
    visuals.menu_corner_radius = egui::CornerRadius::same(8);

    visuals.widgets.noninteractive.bg_fill = surface;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, border);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, text);
    visuals.widgets.noninteractive.corner_radius = radius;

    visuals.widgets.inactive.bg_fill = surface;
    visuals.widgets.inactive.weak_bg_fill = surface;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, border);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, text);
    visuals.widgets.inactive.corner_radius = radius;

    visuals.widgets.hovered.bg_fill = surface_hover;
    visuals.widgets.hovered.weak_bg_fill = surface_hover;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, accent.gamma_multiply(0.75));
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    visuals.widgets.hovered.corner_radius = radius;

    visuals.widgets.active.bg_fill = accent.gamma_multiply(0.28);
    visuals.widgets.active.weak_bg_fill = accent.gamma_multiply(0.28);
    visuals.widgets.active.bg_stroke = Stroke::new(1.2, accent);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    visuals.widgets.active.corner_radius = radius;

    visuals.widgets.open.bg_fill = surface_hover;
    visuals.widgets.open.weak_bg_fill = surface_hover;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0, accent);
    visuals.widgets.open.corner_radius = radius;

    ctx.set_visuals(visuals);
    apply_common_styles(ctx);
}

fn apply_light(ctx: &egui::Context) {
    ctx.set_theme(egui::ThemePreference::Light);
    ctx.send_viewport_cmd(egui::ViewportCommand::SetTheme(egui::SystemTheme::Light));

    let mut visuals = egui::Visuals::light();

    let bg = Color32::from_rgb(243, 245, 249);
    let surface = Color32::from_rgb(255, 255, 255);
    let surface_hover = Color32::from_rgb(246, 248, 252);
    let border = Color32::from_rgb(222, 228, 237);
    let accent = Color32::from_rgb(217, 119, 6);
    let text = Color32::from_rgb(15, 23, 42);

    visuals.dark_mode = false;
    visuals.panel_fill = bg;
    visuals.window_fill = surface;
    visuals.faint_bg_color = Color32::from_rgb(248, 250, 253);
    visuals.extreme_bg_color = Color32::from_rgb(255, 255, 255);
    visuals.override_text_color = Some(text);
    visuals.hyperlink_color = accent;
    visuals.selection.bg_fill = accent.gamma_multiply(0.22);
    visuals.selection.stroke = Stroke::new(1.0, accent);

    let radius = egui::CornerRadius::same(8);
    visuals.window_corner_radius = egui::CornerRadius::same(12);
    visuals.menu_corner_radius = egui::CornerRadius::same(8);

    visuals.widgets.noninteractive.bg_fill = surface;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, border);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, text);
    visuals.widgets.noninteractive.corner_radius = radius;

    visuals.widgets.inactive.bg_fill = surface;
    visuals.widgets.inactive.weak_bg_fill = surface;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, border);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, text);
    visuals.widgets.inactive.corner_radius = radius;

    visuals.widgets.hovered.bg_fill = surface_hover;
    visuals.widgets.hovered.weak_bg_fill = surface_hover;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, accent.gamma_multiply(0.75));
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, text);
    visuals.widgets.hovered.corner_radius = radius;

    visuals.widgets.active.bg_fill = accent.gamma_multiply(0.18);
    visuals.widgets.active.weak_bg_fill = accent.gamma_multiply(0.18);
    visuals.widgets.active.bg_stroke = Stroke::new(1.2, accent);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, text);
    visuals.widgets.active.corner_radius = radius;

    visuals.widgets.open.bg_fill = surface_hover;
    visuals.widgets.open.weak_bg_fill = surface_hover;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0, accent);
    visuals.widgets.open.corner_radius = radius;

    ctx.set_visuals(visuals);
    apply_common_styles(ctx);
}

fn apply_common_styles(ctx: &egui::Context) {
    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = Vec2::new(10.0, 8.0);
        style.spacing.button_padding = Vec2::new(12.0, 7.0);
        style.spacing.window_margin = egui::Margin::same(12);
        style.spacing.menu_margin = egui::Margin::same(8);
        style.text_styles = [
            (
                TextStyle::Heading,
                FontId::new(24.0, FontFamily::Proportional),
            ),
            (TextStyle::Body, FontId::new(15.0, FontFamily::Proportional)),
            (
                TextStyle::Button,
                FontId::new(14.0, FontFamily::Proportional),
            ),
            (
                TextStyle::Small,
                FontId::new(12.0, FontFamily::Proportional),
            ),
            (
                TextStyle::Monospace,
                FontId::new(13.5, FontFamily::Monospace),
            ),
        ]
        .into();
    });
}
