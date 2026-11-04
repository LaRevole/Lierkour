//! Главный экран: панель поиска и сетка карточек.

use eframe::egui::{self, Color32, RichText};

use crate::models::{Film, FilmFilter, FilmStatus};
use crate::storage::AppPaths;
use crate::ui::poster_cache::PosterCache;
use crate::ui::{theme, widgets};

const CARD_WIDTH: f32 = 210.0;
const GRID_SPACING: f32 = 14.0;

pub enum LibraryAction {
    AddNew,
    Edit(i64),
    Move { film_id: i64, target_id: i64 },
    Export,
    Import,
    Settings,
}

#[derive(Default)]
pub struct LibraryState {
    /// Фильм, который сейчас перетаскивают.
    pub dragging: Option<i64>,
    /// Меняется при смене фильтра, чтобы список открывался сверху.
    /// При возврате «назад» значение сохраняется и позиция прокрутки восстанавливается.
    pub scroll_salt: u64,
}

pub fn show(
    ui: &mut egui::Ui,
    films: &[Film],
    filter: &mut FilmFilter,
    categories: &[String],
    state: &mut LibraryState,
    paths: &AppPaths,
    posters: &mut PosterCache,
) -> Option<LibraryAction> {
    let mut action = None;

    egui::Panel::top("library_toolbar")
        .frame(
            egui::Frame::new()
                .fill(theme::SURFACE)
                .inner_margin(egui::Margin::symmetric(16, 10)),
        )
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Моя фильмография");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let add = egui::Button::new(
                        RichText::new("+  Добавить фильм")
                            .strong()
                            .color(Color32::from_rgb(22, 20, 15)),
                    )
                    .fill(theme::ACCENT);
                    if ui.add(add).clicked() {
                        action = Some(LibraryAction::AddNew);
                    }

                    ui.add_space(8.0);
                    if ui
                        .small_button("Настройки")
                        .on_hover_text("Ключ OMDb для автозаполнения")
                        .clicked()
                    {
                        action = Some(LibraryAction::Settings);
                    }
                    if ui
                        .small_button("Импорт")
                        .on_hover_text("Добавить фильмы и обложки из ZIP-архива")
                        .clicked()
                    {
                        action = Some(LibraryAction::Import);
                    }
                    if ui
                        .small_button("Экспорт")
                        .on_hover_text("Сохранить фильмографию и обложки в ZIP-архив")
                        .clicked()
                    {
                        action = Some(LibraryAction::Export);
                    }

                    ui.add_space(12.0);
                    ui.label(
                        RichText::new(format!("Найдено: {}", films.len()))
                            .small()
                            .color(theme::TEXT_MUTED),
                    );
                });
            });

            ui.add_space(4.0);

            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Поиск").color(theme::TEXT_MUTED));
                ui.add(
                    egui::TextEdit::singleline(&mut filter.search)
                        .hint_text("Название, оригинал, режиссёр…")
                        .desired_width(230.0),
                );

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);

                ui.label(RichText::new("Статус").color(theme::TEXT_MUTED));
                egui::ComboBox::from_id_salt("library_status_filter")
                    .width(170.0)
                    .selected_text(filter.status.map_or("Все статусы", FilmStatus::label))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut filter.status, None, "Все статусы");
                        for status in FilmStatus::ALL {
                            ui.selectable_value(&mut filter.status, Some(status), status.label());
                        }
                    });

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);

                ui.label(RichText::new("Категория").color(theme::TEXT_MUTED));
                egui::ComboBox::from_id_salt("library_category_filter")
                    .width(170.0)
                    .selected_text(filter.category.as_deref().unwrap_or("Все категории"))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut filter.category, None, "Все категории");
                        for category in categories {
                            ui.selectable_value(
                                &mut filter.category,
                                Some(category.clone()),
                                category.as_str(),
                            );
                        }
                    });

                ui.add_space(4.0);
                ui.label(
                    RichText::new("Клик — открыть, перетаскивание — порядок")
                        .small()
                        .color(theme::TEXT_MUTED),
                );
            });
        });

    egui::CentralPanel::default()
        .frame(
            egui::Frame::new()
                .fill(theme::BG)
                .inner_margin(egui::Margin::same(16)),
        )
        .show(ui, |ui| {
            if films.is_empty() {
                state.dragging = None;
                empty_state(ui, filter, &mut action);
                return;
            }

            let pointer_pos = ui.ctx().pointer_interact_pos();
            let (released, pointer_down) = ui
                .ctx()
                .input(|input| (input.pointer.any_released(), input.pointer.any_down()));

            egui::ScrollArea::vertical()
                .id_salt(("library_scroll", state.scroll_salt))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let available_width = ui.available_width();
                    let columns = (((available_width + GRID_SPACING) / (CARD_WIDTH + GRID_SPACING))
                        .floor() as usize)
                        .max(1);

                    egui::Grid::new("films_grid")
                        .num_columns(columns)
                        .spacing([GRID_SPACING, GRID_SPACING])
                        .show(ui, |ui| {
                            for (index, film) in films.iter().enumerate() {
                                film_card(
                                    ui,
                                    film,
                                    pointer_pos,
                                    released,
                                    state,
                                    paths,
                                    posters,
                                    &mut action,
                                );
                                if (index + 1) % columns == 0 {
                                    ui.end_row();
                                }
                            }
                        });
                });

            // Также сбрасываем состояние, если кнопка уже отпущена или окно
            // потеряло фокус посреди перетаскивания.
            if released || !pointer_down {
                state.dragging = None;
            }
        });

    action
}

fn empty_state(ui: &mut egui::Ui, filter: &FilmFilter, action: &mut Option<LibraryAction>) {
    ui.vertical_centered(|ui| {
        ui.add_space(120.0);

        if filter.is_active() {
            ui.heading("Ничего не найдено");
            ui.label(
                RichText::new("Попробуйте изменить запрос или сбросить фильтр по статусу.")
                    .color(theme::TEXT_MUTED),
            );
        } else {
            ui.heading("Библиотека пока пуста");
            ui.label(
                RichText::new("Добавьте первый фильм — всё хранится только на этом компьютере.")
                    .color(theme::TEXT_MUTED),
            );
            ui.add_space(12.0);
            if ui
                .add(
                    egui::Button::new(
                        RichText::new("Добавить фильм")
                            .strong()
                            .color(Color32::from_rgb(22, 20, 15)),
                    )
                    .fill(theme::ACCENT),
                )
                .clicked()
            {
                *action = Some(LibraryAction::AddNew);
            }
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn film_card(
    ui: &mut egui::Ui,
    film: &Film,
    pointer_pos: Option<egui::Pos2>,
    released: bool,
    state: &mut LibraryState,
    paths: &AppPaths,
    posters: &mut PosterCache,
    action: &mut Option<LibraryAction>,
) {
    let inner_width = CARD_WIDTH - 20.0;

    let frame = egui::Frame::new()
        .fill(theme::SURFACE)
        .stroke(egui::Stroke::new(1.0, theme::BORDER))
        .corner_radius(egui::CornerRadius::same(12))
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.set_width(inner_width);

                let texture = film
                    .poster_path
                    .as_deref()
                    .and_then(|stored| posters.texture(ui.ctx(), paths, stored, 640));

                widgets::paint_poster(ui, egui::vec2(inner_width, 260.0), texture.as_ref());

                ui.add_space(6.0);

                ui.add(
                    egui::Label::new(
                        RichText::new(&film.title)
                            .size(16.0)
                            .strong()
                            .color(Color32::from_rgb(238, 241, 247)),
                    )
                    .wrap_mode(egui::TextWrapMode::Truncate),
                )
                .on_hover_text(&film.title);

                if !film.original_title.trim().is_empty() {
                    ui.add(
                        egui::Label::new(
                            RichText::new(&film.original_title)
                                .size(12.0)
                                .italics()
                                .color(theme::TEXT_MUTED),
                        )
                        .wrap_mode(egui::TextWrapMode::Truncate),
                    )
                    .on_hover_text(&film.original_title);
                }

                ui.add_space(2.0);

                ui.horizontal(|ui| {
                    if let Some(year) = film.year {
                        ui.label(
                            RichText::new(year.to_string())
                                .small()
                                .strong()
                                .color(theme::TEXT_MUTED),
                        );
                    }
                    if let Some(runtime) = film.runtime_label() {
                        ui.label(RichText::new(runtime).small().color(theme::TEXT_MUTED));
                    }

                    widgets::category_badge(ui, &film.category);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let color = if film.rating.is_some() {
                            theme::ACCENT
                        } else {
                            theme::TEXT_MUTED
                        };
                        ui.label(
                            RichText::new(film.rating_label())
                                .small()
                                .strong()
                                .color(color),
                        )
                        .on_hover_text("Ваша оценка");
                    });
                });

                ui.add_space(2.0);

                ui.horizontal_wrapped(|ui| {
                    widgets::status_badge(ui, film.status);
                    if film.has_seasons() {
                        widgets::season_badge(ui, film.seasons_total, film.watched_season_count());
                    }
                });

                let genres = film.genres_list();
                if !genres.is_empty() {
                    ui.add_space(3.0);
                    ui.horizontal_wrapped(|ui| {
                        for genre in genres.iter().take(3) {
                            widgets::genre_badge(ui, genre);
                        }
                        if genres.len() > 3 {
                            ui.label(
                                RichText::new(format!("+{}", genres.len() - 3))
                                    .small()
                                    .color(theme::TEXT_MUTED),
                            );
                        }
                    });
                }

                let tags = film.tags_list();
                if !tags.is_empty() {
                    ui.add_space(3.0);
                    ui.horizontal_wrapped(|ui| {
                        for tag in tags.iter().take(3) {
                            widgets::tag_badge(ui, tag);
                        }
                        if tags.len() > 3 {
                            ui.label(
                                RichText::new(format!("+{}", tags.len() - 3))
                                    .small()
                                    .color(theme::TEXT_MUTED),
                            )
                            .on_hover_text(film.tags.clone());
                        }
                    });
                }

                if !film.director.trim().is_empty() {
                    ui.add_space(2.0);
                    ui.add(
                        egui::Label::new(
                            RichText::new(format!("Реж.: {}", film.director))
                                .size(11.0)
                                .color(theme::TEXT_MUTED),
                        )
                        .wrap_mode(egui::TextWrapMode::Truncate),
                    )
                    .on_hover_text(format!("Режиссёр: {}", film.director));
                }

                ui.add_space(2.0);

                let description = film.description.trim();
                if description.is_empty() {
                    ui.label(
                        RichText::new("Без описания")
                            .size(12.0)
                            .italics()
                            .color(theme::TEXT_MUTED),
                    );
                } else {
                    ui.add(
                        egui::Label::new(
                            RichText::new(description)
                                .size(12.0)
                                .color(theme::TEXT_MUTED),
                        )
                        .wrap_mode(egui::TextWrapMode::Truncate),
                    )
                    .on_hover_text(description);
                }
            });
        });

    let response = frame.response.interact(egui::Sense::click_and_drag());

    if response.drag_started() {
        state.dragging = Some(film.id);
    }

    let pointer_over = pointer_pos.is_some_and(|pos| response.rect.contains(pos));
    let is_dragging = state.dragging == Some(film.id);
    let is_drop_target = state.dragging.is_some_and(|id| id != film.id) && pointer_over;

    if is_drop_target && released {
        if let Some(dragged_id) = state.dragging.take() {
            *action = Some(LibraryAction::Move {
                film_id: dragged_id,
                target_id: film.id,
            });
        }
    } else if response.clicked() && state.dragging.is_none() {
        *action = Some(LibraryAction::Edit(film.id));
    }

    let stroke = if is_drop_target {
        Some(egui::Stroke::new(2.0, theme::SUCCESS))
    } else if is_dragging {
        Some(egui::Stroke::new(2.0, theme::ACCENT))
    } else if response.hovered() {
        Some(egui::Stroke::new(1.5, theme::ACCENT.gamma_multiply(0.7)))
    } else {
        None
    };

    if let Some(stroke) = stroke {
        ui.painter().rect_stroke(
            response.rect,
            egui::CornerRadius::same(12),
            stroke,
            egui::StrokeKind::Inside,
        );
    }

    response
        .clone()
        .on_hover_text("Нажмите, чтобы открыть. Перетащите, чтобы изменить порядок.");
}
