//! Небольшие переиспользуемые элементы интерфейса.

use eframe::egui::{self, Align2, Color32, FontId, Rect, RichText, Sense, Ui, Vec2};

use crate::models::FilmStatus;

use super::theme;

pub fn status_color(status: FilmStatus) -> Color32 {
    match status {
        FilmStatus::WantToWatch => Color32::from_rgb(111, 168, 255),
        FilmStatus::Watching => Color32::from_rgb(232, 168, 74),
        FilmStatus::Watched => Color32::from_rgb(104, 199, 142),
        FilmStatus::Dropped => Color32::from_rgb(170, 132, 190),
    }
}

pub fn status_badge(ui: &mut Ui, status: FilmStatus) {
    let color = status_color(status);
    egui::Frame::new()
        .fill(color.gamma_multiply(0.16))
        .stroke(egui::Stroke::new(1.0, color.gamma_multiply(0.5)))
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.label(
                RichText::new(status.label())
                    .size(12.0)
                    .strong()
                    .color(color),
            );
        });
}

/// Прогресс просмотра сезонов: «3/8 сезонов».
pub fn season_badge(ui: &mut Ui, total: u16, watched: usize) {
    let fully_watched = total > 0 && watched >= total as usize;
    let color = if fully_watched {
        theme::SUCCESS
    } else if watched == 0 {
        theme::TEXT_MUTED
    } else {
        theme::ACCENT
    };

    egui::Frame::new()
        .fill(color.gamma_multiply(0.16))
        .stroke(egui::Stroke::new(1.0, color.gamma_multiply(0.5)))
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.label(
                RichText::new(format!("{watched}/{total} сезонов"))
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
    let mut label: String = category.chars().take(18).collect();
    if category.chars().count() > 18 {
        label.push('…');
    }

    egui::Frame::new()
        .fill(theme::SURFACE_HOVER)
        .stroke(egui::Stroke::new(1.0, theme::BORDER))
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.label(RichText::new(label).size(11.0).color(theme::TEXT_MUTED))
                .on_hover_text(category);
        });
}

/// Компактная плашка жанра для карточки.
pub fn genre_badge(ui: &mut Ui, genre: &str) {
    egui::Frame::new()
        .fill(theme::ACCENT.gamma_multiply(0.10))
        .stroke(egui::Stroke::new(1.0, theme::ACCENT.gamma_multiply(0.30)))
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(7, 2))
        .show(ui, |ui| {
            ui.label(
                RichText::new(genre)
                    .size(10.5)
                    .color(theme::ACCENT.gamma_multiply(0.9)),
            );
        });
}

/// Компактная плашка тега: нейтральнее жанра, чтобы не спорить с ним.
pub fn tag_badge(ui: &mut Ui, tag: &str) {
    egui::Frame::new()
        .fill(theme::SURFACE)
        .stroke(egui::Stroke::new(1.0, theme::BORDER))
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(7, 2))
        .show(ui, |ui| {
            ui.label(
                RichText::new(format!("#{tag}"))
                    .size(10.5)
                    .color(theme::TEXT_MUTED),
            );
        });
}

/// Рисует постер с сохранением пропорций внутри заданной области.
/// Возвращает прямоугольник, который заняло изображение.
pub fn paint_poster(ui: &mut Ui, bounds: Vec2, texture: Option<&egui::TextureHandle>) -> Rect {
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
            .rect_filled(rect, egui::CornerRadius::same(10), theme::SURFACE_HOVER);
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            "Нет обложки",
            FontId::proportional(14.0),
            theme::TEXT_MUTED,
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
