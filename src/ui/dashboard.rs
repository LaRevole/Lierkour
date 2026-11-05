//! Экран профиля и статистики фильмотеки (Dashboard).

use eframe::egui::{self, Color32, RichText, Vec2};

use crate::models::DashboardStats;
use crate::ui::theme;

pub enum DashboardAction {
    Back,
}

pub fn show(ui: &mut egui::Ui, stats: &DashboardStats) -> Option<DashboardAction> {
    let mut action = None;
    let colors = theme::colors(ui.ctx());

    egui::Panel::top("dashboard_top_bar")
        .frame(
            egui::Frame::new()
                .fill(colors.surface)
                .stroke(egui::Stroke::new(1.0, colors.border))
                .inner_margin(egui::Margin::symmetric(16, 12)),
        )
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button("<- Назад").clicked() {
                    action = Some(DashboardAction::Back);
                }
                ui.add_space(8.0);
                ui.label(
                    RichText::new("Аналитика и профиль библиотеки")
                        .heading()
                        .strong()
                        .color(colors.text),
                );
            });
        });

    egui::CentralPanel::default()
        .frame(
            egui::Frame::new()
                .fill(colors.bg)
                .inner_margin(egui::Margin::same(20)),
        )
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add_space(8.0);

                    // Сетка ключевых показателей (KPI Cards)
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(14.0, 14.0);

                        stat_card(
                            ui,
                            "Всего фильмов",
                            &stats.total_films.to_string(),
                            "в коллекции",
                            colors.accent,
                            colors,
                        );

                        let watched_percent = if stats.total_films > 0 {
                            (stats.watched_films as f64 / stats.total_films as f64 * 100.0) as u32
                        } else {
                            0
                        };
                        stat_card(
                            ui,
                            "Просмотрено",
                            &stats.watched_films.to_string(),
                            &format!("{watched_percent}% от каталога"),
                            colors.success,
                            colors,
                        );

                        let rating_str = stats
                            .average_rating
                            .map(|r| format!("{:.1} / 10", r))
                            .unwrap_or_else(|| "—".to_owned());
                        stat_card(
                            ui,
                            "Средняя оценка",
                            &rating_str,
                            "оценённых фильмов",
                            if colors.is_dark {
                                Color32::from_rgb(250, 204, 21)
                            } else {
                                Color32::from_rgb(202, 138, 4)
                            },
                            colors,
                        );

                        let hours = stats.total_runtime_minutes / 60;
                        let mins = stats.total_runtime_minutes % 60;
                        let time_str = if hours > 0 {
                            format!("{hours} ч {mins} мин")
                        } else {
                            format!("{mins} мин")
                        };
                        stat_card(
                            ui,
                            "Время просмотра",
                            &time_str,
                            "суммарная длительность",
                            if colors.is_dark {
                                Color32::from_rgb(96, 165, 250)
                            } else {
                                Color32::from_rgb(37, 99, 235)
                            },
                            colors,
                        );
                    });

                    ui.add_space(22.0);

                    // Статусы просмотра (полоса распределения)
                    ui.label(
                        RichText::new("Распределение по статусам")
                            .strong()
                            .size(17.0)
                            .color(colors.text),
                    );
                    ui.add_space(6.0);
                    render_status_distribution(ui, stats, colors);

                    ui.add_space(24.0);

                    // Адаптивная секция: Топ-жанры и Топ-режиссёры
                    let is_wide = ui.available_width() >= 620.0;
                    if is_wide {
                        ui.columns(2, |columns| {
                            render_top_genres(&mut columns[0], stats, colors);
                            render_top_directors(&mut columns[1], stats, colors);
                        });
                    } else {
                        render_top_genres(ui, stats, colors);
                        ui.add_space(16.0);
                        render_top_directors(ui, stats, colors);
                    }

                    ui.add_space(24.0);
                });
        });

    action
}

fn render_top_genres(ui: &mut egui::Ui, stats: &DashboardStats, colors: theme::ThemeColors) {
    ui.label(
        RichText::new("Топ-5 жанров")
            .strong()
            .size(17.0)
            .color(colors.text),
    );
    ui.add_space(8.0);
    if stats.top_genres.is_empty() {
        ui.label(
            RichText::new("Нет данных о жанрах")
                .color(colors.text_muted)
                .italics(),
        );
    } else {
        let max_count = stats.top_genres.first().map(|g| g.1).unwrap_or(1).max(1);
        for (genre, count) in &stats.top_genres {
            render_bar(ui, genre, *count, max_count, colors.accent, colors);
        }
    }
}

fn render_top_directors(ui: &mut egui::Ui, stats: &DashboardStats, colors: theme::ThemeColors) {
    ui.label(
        RichText::new("Топ-5 режиссёров")
            .strong()
            .size(17.0)
            .color(colors.text),
    );
    ui.add_space(8.0);
    if stats.top_directors.is_empty() {
        ui.label(
            RichText::new("Нет данных о режиссёрах")
                .color(colors.text_muted)
                .italics(),
        );
    } else {
        let max_count = stats.top_directors.first().map(|d| d.1).unwrap_or(1).max(1);
        for (director, count) in &stats.top_directors {
            let color = if colors.is_dark {
                Color32::from_rgb(96, 165, 250)
            } else {
                Color32::from_rgb(37, 99, 235)
            };
            render_bar(ui, director, *count, max_count, color, colors);
        }
    }
}

fn stat_card(
    ui: &mut egui::Ui,
    title: &str,
    value: &str,
    subtitle: &str,
    accent_color: Color32,
    colors: theme::ThemeColors,
) {
    egui::Frame::new()
        .fill(colors.card_bg)
        .stroke(egui::Stroke::new(1.0, colors.card_stroke))
        .corner_radius(egui::CornerRadius::same(10))
        .inner_margin(egui::Margin::symmetric(18, 14))
        .show(ui, |ui| {
            ui.set_width(170.0);
            ui.vertical(|ui| {
                ui.label(RichText::new(title).size(13.0).color(colors.text_muted));
                ui.add_space(4.0);
                ui.label(RichText::new(value).size(24.0).strong().color(accent_color));
                ui.add_space(2.0);
                ui.label(RichText::new(subtitle).size(11.0).color(colors.text_muted));
            });
        });
}

fn render_status_distribution(
    ui: &mut egui::Ui,
    stats: &DashboardStats,
    colors: theme::ThemeColors,
) {
    egui::Frame::new()
        .fill(colors.card_bg)
        .stroke(egui::Stroke::new(1.0, colors.card_stroke))
        .corner_radius(egui::CornerRadius::same(10))
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            let total = stats.total_films;
            if total == 0 {
                ui.label(RichText::new("Библиотека пуста").color(colors.text_muted));
                return;
            }

            // Полоса пропорций
            let available_width = ui.available_width();
            let height = 18.0;
            let (rect, _) =
                ui.allocate_exact_size(Vec2::new(available_width, height), egui::Sense::hover());

            let painter = ui.painter();
            let track_color = if colors.is_dark {
                Color32::from_rgb(38, 43, 56)
            } else {
                Color32::from_rgb(226, 232, 240)
            };
            painter.rect_filled(rect, egui::CornerRadius::same(6), track_color);

            let watched_w = (stats.watched_films as f32 / total as f32) * available_width;
            let watching_w = (stats.watching as f32 / total as f32) * available_width;
            let want_w = (stats.want_to_watch as f32 / total as f32) * available_width;
            let dropped_w = (stats.dropped as f32 / total as f32) * available_width;

            let watched_color = colors.success;
            let watching_color = if colors.is_dark {
                Color32::from_rgb(96, 165, 250)
            } else {
                Color32::from_rgb(37, 99, 235)
            };
            let want_color = colors.accent;
            let dropped_color = colors.danger;

            let mut cur_x = rect.min.x;
            if watched_w > 0.0 {
                let seg = egui::Rect::from_min_max(
                    egui::pos2(cur_x, rect.min.y),
                    egui::pos2(cur_x + watched_w, rect.max.y),
                );
                painter.rect_filled(seg, egui::CornerRadius::ZERO, watched_color);
                cur_x += watched_w;
            }
            if watching_w > 0.0 {
                let seg = egui::Rect::from_min_max(
                    egui::pos2(cur_x, rect.min.y),
                    egui::pos2(cur_x + watching_w, rect.max.y),
                );
                painter.rect_filled(seg, egui::CornerRadius::ZERO, watching_color);
                cur_x += watching_w;
            }
            if want_w > 0.0 {
                let seg = egui::Rect::from_min_max(
                    egui::pos2(cur_x, rect.min.y),
                    egui::pos2(cur_x + want_w, rect.max.y),
                );
                painter.rect_filled(seg, egui::CornerRadius::ZERO, want_color);
                cur_x += want_w;
            }
            if dropped_w > 0.0 {
                let seg = egui::Rect::from_min_max(
                    egui::pos2(cur_x, rect.min.y),
                    egui::pos2(cur_x + dropped_w, rect.max.y),
                );
                painter.rect_filled(seg, egui::CornerRadius::ZERO, dropped_color);
            }

            ui.add_space(10.0);

            // Легенда под полосой
            ui.horizontal_wrapped(|ui| {
                legend_item(
                    ui,
                    "Просмотрено",
                    stats.watched_films,
                    watched_color,
                    colors,
                );
                ui.add_space(14.0);
                legend_item(ui, "Смотрю", stats.watching, watching_color, colors);
                ui.add_space(14.0);
                legend_item(
                    ui,
                    "Хочу посмотреть",
                    stats.want_to_watch,
                    want_color,
                    colors,
                );
                ui.add_space(14.0);
                legend_item(ui, "Брошено", stats.dropped, dropped_color, colors);
            });
        });
}

fn legend_item(
    ui: &mut egui::Ui,
    label: &str,
    count: usize,
    color: Color32,
    colors: theme::ThemeColors,
) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::new(10.0, 10.0), egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, egui::CornerRadius::same(2), color);
        ui.label(
            RichText::new(format!("{label}: {count}"))
                .size(12.0)
                .color(colors.text),
        );
    });
}

fn render_bar(
    ui: &mut egui::Ui,
    name: &str,
    count: usize,
    max: usize,
    color: Color32,
    colors: theme::ThemeColors,
) {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(name).size(13.0).strong().color(colors.text));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(count.to_string())
                        .size(12.0)
                        .color(colors.text_muted),
                );
            });
        });

        let available_width = ui.available_width().max(50.0);
        let ratio = (count as f32 / max as f32).clamp(0.05, 1.0);
        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(available_width, 8.0), egui::Sense::hover());

        let painter = ui.painter();
        let track_color = if colors.is_dark {
            Color32::from_rgb(38, 43, 56)
        } else {
            Color32::from_rgb(226, 232, 240)
        };
        painter.rect_filled(rect, egui::CornerRadius::same(4), track_color);

        let filled = egui::Rect::from_min_max(
            rect.min,
            egui::pos2(rect.min.x + available_width * ratio, rect.max.y),
        );
        painter.rect_filled(filled, egui::CornerRadius::same(4), color);
        ui.add_space(6.0);
    });
}
