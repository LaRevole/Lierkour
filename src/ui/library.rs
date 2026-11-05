//! Главный экран: адаптивная панель поиска, виртуализированная плавная сетка карточек
//! и плавающий предпросмотр при перетаскивании.

use eframe::egui::{self, Color32, RichText};

use crate::models::{Film, FilmFilter, FilmStatus};
use crate::storage::AppPaths;
use crate::ui::poster_cache::PosterCache;
use crate::ui::{icons, theme, widgets};

const CARD_WIDTH: f32 = 210.0;
const CARD_HEIGHT: f32 = 450.0;
const GRID_SPACING: f32 = 14.0;
const ROW_HEIGHT: f32 = CARD_HEIGHT + GRID_SPACING;

pub enum LibraryAction {
    AddNew,
    Edit(i64),
    Move { film_id: i64, target_id: i64 },
    Export,
    ExportCsv,
    ExportMarkdown,
    Import,
    Settings,
    Dashboard,
    ToggleTheme,
}

#[derive(Default)]
pub struct LibraryState {
    /// Фильм, который сейчас перетаскивают.
    pub dragging: Option<i64>,
    /// Меняется при смене фильтра, чтобы список открывался сверху.
    pub scroll_salt: u64,
    /// Флаг для установки фокуса на строку поиска (Ctrl+F).
    pub focus_search: bool,
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
    let colors = theme::colors(ui.ctx());

    // Опрашиваем фоновый загрузчик текстур
    posters.poll(ui.ctx());

    egui::Panel::top("library_toolbar")
        .frame(
            egui::Frame::new()
                .fill(colors.surface)
                .stroke(egui::Stroke::new(1.0, colors.border))
                .inner_margin(egui::Margin::symmetric(16, 10)),
        )
        .show(ui, |ui| {
            let available_width = ui.available_width();
            let is_narrow = available_width < 820.0;

            if is_narrow {
                // Узкий режим (< 820px): Заголовок сверху, кнопки отдельной строкой снизу
                ui.horizontal(|ui| {
                    render_header_brand(ui, colors, films.len());
                });

                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                    render_toolbar_buttons_ltr(ui, colors, &mut action);
                });
            } else {
                // Стандартный широкий режим: Заголовок слева, кнопки справа
                ui.horizontal(|ui| {
                    render_header_brand(ui, colors, films.len());

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                        render_toolbar_buttons_rtl(ui, colors, &mut action);
                    });
                });
            }

            ui.add_space(8.0);

            // Адаптивная панель поиска и фильтров с автоматическим переносом
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(8.0, 6.0);

                ui.label(RichText::new("Поиск:").color(colors.text_muted));
                let search_width = (ui.available_width() * 0.35).clamp(160.0, 260.0);
                let search_res = ui.add(
                    egui::TextEdit::singleline(&mut filter.search)
                        .hint_text("Название, режиссёр... (Ctrl+F)")
                        .desired_width(search_width),
                );
                if state.focus_search {
                    search_res.request_focus();
                    state.focus_search = false;
                }

                ui.separator();

                ui.label(RichText::new("Статус:").color(colors.text_muted));
                egui::ComboBox::from_id_salt("library_status_filter")
                    .width(140.0)
                    .selected_text(filter.status.map_or("Все статусы", FilmStatus::label))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut filter.status, None, "Все статусы");
                        for status in FilmStatus::ALL {
                            ui.selectable_value(&mut filter.status, Some(status), status.label());
                        }
                    });

                ui.separator();

                ui.label(RichText::new("Категория:").color(colors.text_muted));
                egui::ComboBox::from_id_salt("library_category_filter")
                    .width(140.0)
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

                if filter.is_active() && ui.small_button("Сброс").clicked() {
                    *filter = FilmFilter::default();
                }
            });
        });

    egui::CentralPanel::default()
        .frame(
            egui::Frame::new()
                .fill(colors.bg)
                .inner_margin(egui::Margin::same(16)),
        )
        .show(ui, |ui| {
            if films.is_empty() {
                state.dragging = None;
                empty_state(ui, filter, colors, &mut action);
                return;
            }

            let pointer_pos = ui.ctx().pointer_interact_pos();
            let (released, pointer_down) = ui
                .ctx()
                .input(|input| (input.pointer.any_released(), input.pointer.any_down()));

            let available_width = ui.available_width();
            let columns = (((available_width + GRID_SPACING) / (CARD_WIDTH + GRID_SPACING)).floor()
                as usize)
                .max(1);
            let total_rows = films.len().div_ceil(columns);

            egui::ScrollArea::vertical()
                .id_salt(("library_scroll", state.scroll_salt))
                .auto_shrink([false, false])
                .show_rows(ui, ROW_HEIGHT, total_rows, |ui, row_range| {
                    for row_idx in row_range {
                        let start = row_idx * columns;
                        let end = (start + columns).min(films.len());
                        let row_films = &films[start..end];

                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = GRID_SPACING;
                            for film in row_films {
                                film_card(
                                    ui,
                                    film,
                                    pointer_pos,
                                    released,
                                    state,
                                    paths,
                                    posters,
                                    colors,
                                    &mut action,
                                );
                            }
                        });
                        ui.add_space(GRID_SPACING);
                    }
                });

            // Плавающая карточка при перетаскивании (как на рабочем столе Windows)
            if let Some(dragged_id) = state.dragging
                && pointer_down
                && let Some(pos) = pointer_pos
                && let Some(dragged_film) = films.iter().find(|f| f.id == dragged_id)
            {
                render_floating_drag_preview(ui.ctx(), dragged_film, pos, paths, posters, colors);
            }

            // Сбрасываем перетаскивание при отпускании мыши
            if released || !pointer_down {
                state.dragging = None;
            }
        });

    action
}

fn render_header_brand(ui: &mut egui::Ui, colors: theme::ThemeColors, film_count: usize) {
    ui.label(
        RichText::new("Моя фильмография")
            .heading()
            .strong()
            .color(colors.text),
    );
    ui.add_space(4.0);
    render_version_badge(ui, colors);
    ui.add_space(6.0);
    ui.label(
        RichText::new(format!("(найдено: {film_count})"))
            .small()
            .color(colors.text_muted),
    );
}

/// Кнопки тулбара для стандартного режима (справа налево).
fn render_toolbar_buttons_rtl(
    ui: &mut egui::Ui,
    colors: theme::ThemeColors,
    action: &mut Option<LibraryAction>,
) {
    // В RTL порядок обратный: Настройки (крайняя справа) -> Тема -> Импорт -> Экспорт -> Статистика -> + Добавить
    if icons::icon_button(ui, "Настройки (OMDb API)", icons::paint_gear).clicked() {
        *action = Some(LibraryAction::Settings);
    }

    let is_dark = colors.is_dark;
    let theme_tip = if is_dark {
        "Переключить на светлую тему"
    } else {
        "Переключить на тёмную тему"
    };
    if icons::icon_button(ui, theme_tip, |p, r, c| {
        icons::paint_theme_sun_moon(p, r, is_dark, c)
    })
    .clicked()
    {
        *action = Some(LibraryAction::ToggleTheme);
    }

    if ui
        .button("Импорт")
        .on_hover_text("Добавить фильмы, обложки и кадры из ZIP-архива")
        .clicked()
    {
        *action = Some(LibraryAction::Import);
    }

    render_export_menu(ui, colors, action);

    if icons::icon_button(ui, "Статистика и аналитика", icons::paint_chart).clicked()
    {
        *action = Some(LibraryAction::Dashboard);
    }

    let add_text_color = if colors.is_dark {
        Color32::from_rgb(20, 20, 20)
    } else {
        Color32::WHITE
    };
    let add_btn = egui::Button::new(
        RichText::new("+ Добавить фильм")
            .strong()
            .color(add_text_color),
    )
    .fill(colors.accent);
    if ui
        .add(add_btn)
        .on_hover_text("Добавить новый фильм в библиотеку (Ctrl+N)")
        .clicked()
    {
        *action = Some(LibraryAction::AddNew);
    }
}

/// Кнопки тулбара для компактного режима (слева направо).
fn render_toolbar_buttons_ltr(
    ui: &mut egui::Ui,
    colors: theme::ThemeColors,
    action: &mut Option<LibraryAction>,
) {
    let add_text_color = if colors.is_dark {
        Color32::from_rgb(20, 20, 20)
    } else {
        Color32::WHITE
    };
    let add_btn = egui::Button::new(
        RichText::new("+ Добавить фильм")
            .strong()
            .color(add_text_color),
    )
    .fill(colors.accent);
    if ui
        .add(add_btn)
        .on_hover_text("Добавить новый фильм в библиотеку (Ctrl+N)")
        .clicked()
    {
        *action = Some(LibraryAction::AddNew);
    }

    if icons::icon_button(ui, "Статистика и аналитика", icons::paint_chart).clicked()
    {
        *action = Some(LibraryAction::Dashboard);
    }

    render_export_menu(ui, colors, action);

    if ui
        .button("Импорт")
        .on_hover_text("Добавить фильмы, обложки и кадры из ZIP-архива")
        .clicked()
    {
        *action = Some(LibraryAction::Import);
    }

    let is_dark = colors.is_dark;
    let theme_tip = if is_dark {
        "Переключить на светлую тему"
    } else {
        "Переключить на тёмную тему"
    };
    if icons::icon_button(ui, theme_tip, |p, r, c| {
        icons::paint_theme_sun_moon(p, r, is_dark, c)
    })
    .clicked()
    {
        *action = Some(LibraryAction::ToggleTheme);
    }

    if icons::icon_button(ui, "Настройки (OMDb API)", icons::paint_gear).clicked() {
        *action = Some(LibraryAction::Settings);
    }
}

fn render_export_menu(
    ui: &mut egui::Ui,
    colors: theme::ThemeColors,
    action: &mut Option<LibraryAction>,
) {
    ui.menu_button("Экспорт v", |ui| {
        ui.set_min_width(240.0);
        ui.spacing_mut().button_padding = egui::vec2(10.0, 6.0);

        ui.label(
            RichText::new("Выберите формат экспорта:")
                .size(12.0)
                .color(colors.text_muted),
        );
        ui.add_space(4.0);

        if ui
            .button("Полный архив (ZIP)")
            .on_hover_text("База данных JSON, обложки и кадры для переноса на другой ПК")
            .clicked()
        {
            *action = Some(LibraryAction::Export);
            ui.close();
        }

        if ui
            .button("Таблица Excel (CSV)")
            .on_hover_text("Таблица всех фильмов с поддержкой кириллицы (UTF-8 BOM)")
            .clicked()
        {
            *action = Some(LibraryAction::ExportCsv);
            ui.close();
        }

        if ui
            .button("Каталог Markdown (.md)")
            .on_hover_text("Форматированный документ с постерами и подробной информацией")
            .clicked()
        {
            *action = Some(LibraryAction::ExportMarkdown);
            ui.close();
        }
    });
}

fn render_version_badge(ui: &mut egui::Ui, colors: theme::ThemeColors) {
    egui::Frame::new()
        .fill(
            colors
                .accent
                .gamma_multiply(if colors.is_dark { 0.16 } else { 0.12 }),
        )
        .stroke(egui::Stroke::new(1.0, colors.accent.gamma_multiply(0.35)))
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(6, 2))
        .show(ui, |ui| {
            ui.label(
                RichText::new("v1.1.2")
                    .size(11.0)
                    .strong()
                    .color(colors.accent),
            );
        });
}

fn empty_state(
    ui: &mut egui::Ui,
    filter: &FilmFilter,
    colors: theme::ThemeColors,
    action: &mut Option<LibraryAction>,
) {
    ui.vertical_centered(|ui| {
        ui.add_space(120.0);

        if filter.is_active() {
            ui.label(
                RichText::new("Ничего не найдено")
                    .heading()
                    .strong()
                    .color(colors.text),
            );
            ui.add_space(4.0);
            ui.label(
                RichText::new("Попробуйте изменить поисковый запрос или сбросить фильтры.")
                    .color(colors.text_muted),
            );
            ui.add_space(12.0);
            if ui.button("Сбросить фильтры").clicked() {
                // Вызывающий код обновит фильтр
            }
        } else {
            ui.label(
                RichText::new("Библиотека пока пуста")
                    .heading()
                    .strong()
                    .color(colors.text),
            );
            ui.add_space(4.0);
            ui.label(
                RichText::new(
                    "Добавьте свой первый фильм — коллекция надёжно хранится на вашем компьютере.",
                )
                .color(colors.text_muted),
            );
            ui.add_space(16.0);
            let add_text_color = if colors.is_dark {
                Color32::from_rgb(20, 20, 20)
            } else {
                Color32::WHITE
            };
            if ui
                .add(
                    egui::Button::new(
                        RichText::new("+ Добавить первый фильм")
                            .strong()
                            .color(add_text_color),
                    )
                    .fill(colors.accent),
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
    colors: theme::ThemeColors,
    action: &mut Option<LibraryAction>,
) {
    let inner_width = CARD_WIDTH - 20.0;
    let inner_height = CARD_HEIGHT - 20.0;

    let frame = egui::Frame::new()
        .fill(colors.card_bg)
        .stroke(egui::Stroke::new(1.0, colors.card_stroke))
        .corner_radius(egui::CornerRadius::same(12))
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(inner_width);
            ui.set_height(inner_height);

            ui.vertical(|ui| {
                // 1. Постер (фиксированная высота 250px)
                let texture = film
                    .poster_path
                    .as_deref()
                    .and_then(|stored| posters.thumbnail_texture(ui.ctx(), paths, stored, 450));

                widgets::paint_poster(ui, egui::vec2(inner_width, 250.0), texture.as_ref());

                ui.add_space(6.0);

                // 2. Название фильма (строго 1 строка)
                ui.add(
                    egui::Label::new(
                        RichText::new(&film.title)
                            .size(15.0)
                            .strong()
                            .color(colors.text),
                    )
                    .wrap_mode(egui::TextWrapMode::Truncate),
                )
                .on_hover_text(&film.title);

                // 3. Подзаголовок: оригинал или режиссёр (строго 1 строка)
                let sub = if !film.original_title.trim().is_empty() {
                    film.original_title.trim()
                } else if !film.director.trim().is_empty() {
                    film.director.trim()
                } else {
                    ""
                };
                ui.add(
                    egui::Label::new(
                        RichText::new(sub)
                            .size(11.5)
                            .italics()
                            .color(colors.text_muted),
                    )
                    .wrap_mode(egui::TextWrapMode::Truncate),
                )
                .on_hover_text(sub);

                ui.add_space(3.0);

                // 4. Год, длительность, категория и оценка
                ui.horizontal(|ui| {
                    if let Some(year) = film.year {
                        ui.label(
                            RichText::new(year.to_string())
                                .small()
                                .strong()
                                .color(colors.text_muted),
                        );
                    }
                    if let Some(runtime) = film.runtime_label() {
                        ui.label(RichText::new(runtime).small().color(colors.text_muted));
                    }

                    widgets::category_badge(ui, &film.category);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let color = if film.rating.is_some() {
                            colors.accent
                        } else {
                            colors.text_muted
                        };
                        let score_text =
                            film.rating.map_or("—".to_owned(), |r| format!("{r}/10"));
                        ui.label(RichText::new(score_text).small().strong().color(color))
                            .on_hover_text(format!("Ваша оценка: {}", film.rating_label()));
                    });
                });

                ui.add_space(3.0);

                // 5. Статус и сезоны сериала (строго 1 горизонтальная строка)
                ui.horizontal(|ui| {
                    widgets::status_badge(ui, film.status);
                    if film.has_seasons() {
                        widgets::season_badge(ui, film.seasons_total, film.watched_season_count());
                    }
                });

                ui.add_space(3.0);

                // 6. Жанры: до 2 штук на карточке в 1 строку (без вертикального разрыва)
                let genres = film.genres_list();
                if !genres.is_empty() {
                    ui.horizontal(|ui| {
                        for genre in genres.iter().take(2) {
                            widgets::genre_badge(ui, genre);
                        }
                        if genres.len() > 2 {
                            ui.label(
                                RichText::new(format!("+{}", genres.len() - 2))
                                    .small()
                                    .color(colors.text_muted),
                            )
                            .on_hover_text(film.genres.clone());
                        }
                    });
                }

                ui.add_space(3.0);

                // 7. Описание или тег (строго 1 строка с обрезкой)
                let desc = film.description.trim();
                let desc_label = if !desc.is_empty() {
                    desc
                } else if !film.tags.trim().is_empty() {
                    film.tags.trim()
                } else {
                    "Без описания"
                };
                ui.add(
                    egui::Label::new(
                        RichText::new(desc_label)
                            .size(11.0)
                            .color(colors.text_muted),
                    )
                    .wrap_mode(egui::TextWrapMode::Truncate),
                )
                .on_hover_text(desc_label);
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
        Some(egui::Stroke::new(2.5, colors.success))
    } else if is_dragging {
        Some(egui::Stroke::new(2.0, colors.accent))
    } else if response.hovered() {
        Some(egui::Stroke::new(1.5, colors.accent.gamma_multiply(0.7)))
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
        .on_hover_text("Нажмите, чтобы открыть карточку. Перетащите для смены порядка.");
}

/// Плавающий полупрозрачный предпросмотр карточки под курсором при перетаскивании.
fn render_floating_drag_preview(
    ctx: &egui::Context,
    film: &Film,
    pointer_pos: egui::Pos2,
    paths: &AppPaths,
    posters: &mut PosterCache,
    colors: theme::ThemeColors,
) {
    egui::Area::new(egui::Id::unique("floating_drag_preview"))
        .fixed_pos(pointer_pos + egui::vec2(16.0, 16.0))
        .order(egui::Order::Tooltip)
        .interactable(false)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(colors.card_bg.gamma_multiply(0.92))
                .stroke(egui::Stroke::new(2.0, colors.accent))
                .corner_radius(egui::CornerRadius::same(10))
                .inner_margin(egui::Margin::same(8))
                .show(ui, |ui| {
                    ui.set_width(130.0);
                    let texture = film
                        .poster_path
                        .as_deref()
                        .and_then(|stored| posters.thumbnail_texture(ctx, paths, stored, 200));

                    widgets::paint_poster(ui, egui::vec2(130.0, 170.0), texture.as_ref());
                    ui.add_space(4.0);

                    ui.add(
                        egui::Label::new(
                            RichText::new(&film.title)
                                .size(12.5)
                                .strong()
                                .color(colors.text),
                        )
                        .wrap_mode(egui::TextWrapMode::Truncate),
                    );

                    if let Some(year) = film.year {
                        ui.label(
                            RichText::new(year.to_string())
                                .small()
                                .color(colors.text_muted),
                        );
                    }
                });
        });
}
