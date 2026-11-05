//! Небольшие переиспользуемые элементы интерфейса без эмодзи-символов.

use eframe::egui::{self, Align2, Color32, FontId, Rect, RichText, Sense, Ui, Vec2};

use crate::models::FilmStatus;

use super::theme;

pub fn status_color(status: FilmStatus, is_dark: bool) -> Color32 {
    if is_dark {
        match status {
            FilmStatus::WantToWatch => Color32::from_rgb(96, 165, 250), // blue-400
            FilmStatus::Watching => Color32::from_rgb(245, 158, 11),    // amber-500
            FilmStatus::Watched => Color32::from_rgb(52, 211, 153),     // emerald-400
            FilmStatus::Dropped => Color32::from_rgb(192, 132, 252),    // purple-400
        }
    } else {
        match status {
            FilmStatus::WantToWatch => Color32::from_rgb(37, 99, 235), // blue-600
            FilmStatus::Watching => Color32::from_rgb(217, 119, 6),    // amber-600
            FilmStatus::Watched => Color32::from_rgb(16, 149, 93),     // emerald-600
            FilmStatus::Dropped => Color32::from_rgb(147, 51, 234),    // purple-600
        }
    }
}

pub fn status_badge(ui: &mut Ui, status: FilmStatus) {
    let colors = theme::colors(ui.ctx());
    let color = status_color(status, colors.is_dark);
    let fill_alpha = if colors.is_dark { 0.16 } else { 0.12 };
    let stroke_alpha = if colors.is_dark { 0.45 } else { 0.35 };

    egui::Frame::new()
        .fill(color.gamma_multiply(fill_alpha))
        .stroke(egui::Stroke::new(1.0, color.gamma_multiply(stroke_alpha)))
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.label(
                RichText::new(status.label())
                    .size(11.5)
                    .strong()
                    .color(color),
            );
        });
}

/// Прогресс просмотра сезонов: «3/8 сез.».
pub fn season_badge(ui: &mut Ui, total: u16, watched: usize) {
    let colors = theme::colors(ui.ctx());
    let fully_watched = total > 0 && watched >= total as usize;
    let color = if fully_watched {
        colors.success
    } else if watched == 0 {
        colors.text_muted
    } else {
        colors.accent
    };

    let fill_alpha = if colors.is_dark { 0.16 } else { 0.12 };
    let stroke_alpha = if colors.is_dark { 0.45 } else { 0.35 };

    egui::Frame::new()
        .fill(color.gamma_multiply(fill_alpha))
        .stroke(egui::Stroke::new(1.0, color.gamma_multiply(stroke_alpha)))
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.label(
                RichText::new(format!("{watched}/{total} сез."))
                    .size(11.0)
                    .strong()
                    .color(color),
            )
            .on_hover_text(if fully_watched {
                format!("Все {total} сезонов просмотрены")
            } else {
                format!("Просмотрено {watched} из {total} — сериал не досмотрен")
            });
        });
}

/// Нейтральный бейдж категории. Длинные названия обрезаются, чтобы не ломать карточку.
pub fn category_badge(ui: &mut Ui, category: &str) {
    let colors = theme::colors(ui.ctx());
    let mut label: String = category.chars().take(16).collect();
    if category.chars().count() > 16 {
        label.push('…');
    }

    egui::Frame::new()
        .fill(colors.surface_hover)
        .stroke(egui::Stroke::new(1.0, colors.border))
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(7, 3))
        .show(ui, |ui| {
            ui.add(
                egui::Label::new(RichText::new(label).size(11.0).color(colors.text_muted))
                    .wrap_mode(egui::TextWrapMode::Truncate),
            )
            .on_hover_text(category);
        });
}

/// Компактная плашка жанра для карточки с защитой от вертикального переноса по буквам.
pub fn genre_badge(ui: &mut Ui, genre: &str) {
    let colors = theme::colors(ui.ctx());
    let fill_alpha = if colors.is_dark { 0.12 } else { 0.08 };
    let stroke_alpha = if colors.is_dark { 0.32 } else { 0.25 };

    let trimmed = genre.trim();
    let mut display_name = trimmed.to_owned();
    if display_name.chars().count() > 14 {
        display_name = format!("{}…", display_name.chars().take(13).collect::<String>());
    }

    egui::Frame::new()
        .fill(colors.accent.gamma_multiply(fill_alpha))
        .stroke(egui::Stroke::new(
            1.0,
            colors.accent.gamma_multiply(stroke_alpha),
        ))
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(6, 2))
        .show(ui, |ui| {
            ui.add(
                egui::Label::new(
                    RichText::new(display_name)
                        .size(10.5)
                        .color(colors.accent.gamma_multiply(0.95)),
                )
                .wrap_mode(egui::TextWrapMode::Truncate),
            )
            .on_hover_text(trimmed);
        });
}

/// Компактная плашка тега: нейтральнее жанра, с защитой от вертикального разбиения.
#[allow(dead_code)]
pub fn tag_badge(ui: &mut Ui, tag: &str) {
    let colors = theme::colors(ui.ctx());
    let trimmed = tag.trim();
    let mut display_name = trimmed.to_owned();
    if display_name.chars().count() > 14 {
        display_name = format!("{}…", display_name.chars().take(13).collect::<String>());
    }

    egui::Frame::new()
        .fill(colors.surface_hover)
        .stroke(egui::Stroke::new(1.0, colors.border_subtle))
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(6, 2))
        .show(ui, |ui| {
            ui.add(
                egui::Label::new(
                    RichText::new(format!("#{display_name}"))
                        .size(10.5)
                        .color(colors.text_muted),
                )
                .wrap_mode(egui::TextWrapMode::Truncate),
            )
            .on_hover_text(trimmed);
        });
}

/// Рисует постер с сохранением пропорций внутри заданной области.
/// Возвращает прямоугольник, который заняло изображение.
pub fn paint_poster(ui: &mut Ui, bounds: Vec2, texture: Option<&egui::TextureHandle>) -> Rect {
    let colors = theme::colors(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(bounds, Sense::hover());

    if let Some(texture) = texture {
        let image_size = fit_size(texture.size_vec2(), bounds);
        let image_rect = Rect::from_center_size(rect.center(), image_size);
        egui::Image::new(egui::load::SizedTexture::new(
            texture.id(),
            texture.size_vec2(),
        ))
        .paint_at(ui, image_rect);
    } else {
        ui.painter()
            .rect_filled(rect, egui::CornerRadius::same(10), colors.surface_hover);
        ui.painter().rect_stroke(
            rect,
            egui::CornerRadius::same(10),
            egui::Stroke::new(1.0, colors.border_subtle),
            egui::StrokeKind::Inside,
        );
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            "Нет обложки",
            FontId::proportional(13.0),
            colors.text_muted,
        );
    }

    rect
}

fn fit_size(original: Vec2, bounds: Vec2) -> Vec2 {
    if original.x <= 0.0 || original.y <= 0.0 {
        return Vec2::ZERO;
    }

    let scale = (bounds.x / original.x).min(bounds.y / original.y);
    original * scale
}
