//! Экран создания и редактирования фильма.

use std::path::PathBuf;
use std::sync::Arc;

use eframe::egui::{self, Color32, RichText};

use crate::error::AppError;
use crate::models::{
    Film, FilmScreenshot, FilmStatus, MAX_RUNTIME_MINUTES, MAX_SEASONS, MAX_YEAR, NewFilm,
    RelatedFilm, RelationKind, split_comma_list,
};
use crate::storage::{self, AppPaths};
use crate::ui::poster_cache::{self, PosterCache};
use crate::ui::{theme, widgets};

const POSTER_WIDTH: f32 = 220.0;
const POSTER_HEIGHT: f32 = 305.0;

pub enum EditorAction {
    None,
    Save,
    Cancel,
    Delete(i64),
    /// Запрос метаданных по текущему названию/году.
    Autofill,
    AddRelation {
        source_id: i64,
        target_id: i64,
        kind: RelationKind,
    },
    RemoveRelation(i64),
    OpenRelated(i64),
    AddScreenshotFile(PathBuf),
    AddScreenshotClipboard(Arc<image::RgbaImage>),
    DeleteScreenshot(i64),
}

/// Обложка, подготовленная в редакторе, но ещё не сохранённая.
#[derive(Clone)]
pub enum PendingPoster {
    /// Файл, выбранный через системный диалог.
    File(PathBuf),
    /// Изображение, вставленное из буфера обмена.
    Clipboard { image: Arc<image::RgbaImage> },
}

pub struct EditorState {
    pub film_id: Option<i64>,
    pub title: String,
    pub original_title: String,
    pub category: String,
    pub new_category: String,
    pub director: String,
    pub year: Option<u16>,
    pub genres: String,
    pub country: String,
    pub runtime_minutes: Option<u16>,
    pub imdb_id: String,
    pub tags: String,
    pub status: FilmStatus,
    pub rating: Option<u8>,
    pub seasons_total: u16,
    pub watched_seasons: Vec<u16>,
    pub description: String,
    pub thoughts: String,
    pub quotes: Vec<String>,
    pub new_quote_input: String,
    pub screenshots: Vec<FilmScreenshot>,

    /// Связи текущего фильма (загружаются приложением из БД).
    pub relations: Vec<RelatedFilm>,
    /// Фильмы, с которыми можно создать связь (без текущего).
    pub relation_candidates: Vec<(i64, String)>,
    pub relation_kind: RelationKind,
    pub relation_target: Option<i64>,
    pub relation_search: String,

    /// Обложка, которая уже сохранена в БД.
    pub original_poster: Option<String>,
    /// Новая обложка. Копируется/записывается только при сохранении.
    pub pending_poster: Option<PendingPoster>,
    /// Готовая текстура для предпросмотра вставленного изображения.
    pub pending_texture: Option<egui::TextureHandle>,
    /// Пользователь нажал «Убрать обложку».
    pub remove_poster: bool,

    pub form_error: Option<String>,

    /// Уникальный идентификатор прокрутки страницы. Новый фильм получает
    /// новое значение, поэтому карточка открывается сверху, а не с той
    /// позиции, на которой был предыдущий экран.
    pub scroll_salt: u64,
}

impl EditorState {
    pub fn new() -> Self {
        Self {
            film_id: None,
            title: String::new(),
            original_title: String::new(),
            category: crate::models::DEFAULT_CATEGORY.to_owned(),
            new_category: String::new(),
            director: String::new(),
            year: None,
            genres: String::new(),
            country: String::new(),
            runtime_minutes: None,
            imdb_id: String::new(),
            tags: String::new(),
            status: FilmStatus::WantToWatch,
            rating: None,
            seasons_total: 0,
            watched_seasons: Vec::new(),
            description: String::new(),
            thoughts: String::new(),
            quotes: Vec::new(),
            new_quote_input: String::new(),
            screenshots: Vec::new(),
            relations: Vec::new(),
            relation_candidates: Vec::new(),
            relation_kind: RelationKind::Sequel,
            relation_target: None,
            relation_search: String::new(),
            original_poster: None,
            pending_poster: None,
            pending_texture: None,
            remove_poster: false,
            form_error: None,
            scroll_salt: 0,
        }
    }

    pub fn from_film(film: &Film) -> Self {
        Self {
            film_id: Some(film.id),
            title: film.title.clone(),
            original_title: film.original_title.clone(),
            category: film.category.clone(),
            new_category: String::new(),
            director: film.director.clone(),
            year: film.year,
            genres: film.genres.clone(),
            country: film.country.clone(),
            runtime_minutes: film.runtime_minutes,
            imdb_id: film.imdb_id.clone(),
            tags: film.tags.clone(),
            status: film.status,
            rating: film.rating,
            seasons_total: film.seasons_total,
            watched_seasons: film.watched_seasons.clone(),
            description: film.description.clone(),
            thoughts: film.thoughts.clone(),
            quotes: film.quotes.clone(),
            new_quote_input: String::new(),
            screenshots: Vec::new(),
            relations: Vec::new(),
            relation_candidates: Vec::new(),
            relation_kind: RelationKind::Sequel,
            relation_target: None,
            relation_search: String::new(),
            original_poster: film.poster_path.clone(),
            pending_poster: None,
            pending_texture: None,
            remove_poster: false,
            form_error: None,
            scroll_salt: 0,
        }
    }

    pub fn to_draft(&self, poster_path: Option<String>) -> NewFilm {
        NewFilm {
            title: self.title.trim().to_owned(),
            original_title: self.original_title.trim().to_owned(),
            category: self.category.trim().to_owned(),
            director: self.director.trim().to_owned(),
            year: self.year,
            genres: self.genres.trim().to_owned(),
            country: self.country.trim().to_owned(),
            runtime_minutes: self.runtime_minutes,
            imdb_id: self.imdb_id.trim().to_owned(),
            tags: self.tags.trim().to_owned(),
            status: self.status,
            rating: self.rating,
            description: self.description.trim().to_owned(),
            thoughts: self.thoughts.trim().to_owned(),
            poster_path,
            seasons_total: self.seasons_total,
            watched_seasons: self.watched_seasons.clone(),
            quotes: self.quotes.clone(),
        }
    }

    pub fn visible_poster(&self) -> Option<&str> {
        if self.remove_poster {
            None
        } else {
            self.original_poster.as_deref()
        }
    }
}

impl Default for EditorState {
    fn default() -> Self {
        Self::new()
    }
}

pub fn show(
    ui: &mut egui::Ui,
    state: &mut EditorState,
    paths: &AppPaths,
    categories: &[String],
    tag_suggestions: &[String],
    metadata_busy: bool,
    posters: &mut PosterCache,
) -> EditorAction {
    let mut action = EditorAction::None;
    let ctx = ui.ctx().clone();

    if ctx.input(|input| input.modifiers.command && input.key_pressed(egui::Key::Enter)) {
        action = EditorAction::Save;
    }

    // Ctrl+V в пустом месте формы вставляет картинку из буфера.
    // Если фокус в текстовом поле — не мешаем egui вставлять текст.
    if ctx.input(|input| input.modifiers.command && input.key_pressed(egui::Key::V))
        && !ctx.text_edit_focused()
    {
        paste_poster(&ctx, state);
    }

    let colors = theme::colors(ui.ctx());

    egui::Panel::top("editor_top_bar")
        .frame(
            egui::Frame::new()
                .fill(colors.surface)
                .stroke(egui::Stroke::new(1.0, colors.border))
                .inner_margin(egui::Margin::symmetric(16, 12)),
        )
        .show(ui, |ui| {
            let is_narrow = ui.available_width() < 640.0;
            if is_narrow {
                ui.horizontal(|ui| {
                    if ui.button("<- Назад").clicked() {
                        action = EditorAction::Cancel;
                    }
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(if state.film_id.is_some() {
                            "Редактирование фильма"
                        } else {
                            "Новый фильм"
                        })
                        .heading()
                        .strong()
                        .color(colors.text),
                    );
                });
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    if ui.add(save_button(colors)).clicked() {
                        action = EditorAction::Save;
                    }
                    if let Some(id) = state.film_id
                        && ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Удалить фильм").color(colors.danger),
                                )
                                .fill(colors.danger.gamma_multiply(0.12)),
                            )
                            .clicked()
                    {
                        action = EditorAction::Delete(id);
                    }
                    ui.label(
                        RichText::new("Ctrl+Enter / Ctrl+S")
                            .small()
                            .color(colors.text_muted),
                    );
                });
            } else {
                ui.horizontal(|ui| {
                    if ui.button("<- Назад").clicked() {
                        action = EditorAction::Cancel;
                    }

                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(if state.film_id.is_some() {
                            "Редактирование фильма"
                        } else {
                            "Новый фильм"
                        })
                        .heading()
                        .strong()
                        .color(colors.text),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(save_button(colors)).clicked() {
                            action = EditorAction::Save;
                        }

                        if let Some(id) = state.film_id
                            && ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("Удалить фильм").color(colors.danger),
                                    )
                                    .fill(colors.danger.gamma_multiply(0.12)),
                                )
                                .clicked()
                        {
                            action = EditorAction::Delete(id);
                        }

                        ui.label(
                            RichText::new("Ctrl+Enter / Ctrl+S")
                                .small()
                                .color(colors.text_muted),
                        );
                    });
                });
            }
        });

    egui::CentralPanel::default()
        .frame(
            egui::Frame::new()
                .fill(colors.bg)
                .inner_margin(egui::Margin::same(18)),
        )
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt(("editor_scroll", state.scroll_salt))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let is_narrow_content = ui.available_width() < 700.0;
                    if is_narrow_content {
                        poster_column(ui, state, paths, posters);
                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(10.0);
                        fields_column(
                            ui,
                            state,
                            categories,
                            tag_suggestions,
                            metadata_busy,
                            &mut action,
                        );
                    } else {
                        ui.horizontal_top(|ui| {
                            poster_column(ui, state, paths, posters);
                            ui.add_space(12.0);
                            ui.separator();
                            ui.add_space(12.0);
                            fields_column(
                                ui,
                                state,
                                categories,
                                tag_suggestions,
                                metadata_busy,
                                &mut action,
                            );
                        });
                    }

                    ui.add_space(8.0);
                    relations_editor(ui, state, &mut action);

                    ui.add_space(8.0);
                    quotes_editor(ui, state);

                    ui.add_space(8.0);
                    screenshots_gallery(ui, state, paths, posters, &mut action);

                    if let Some(error) = &state.form_error {
                        ui.add_space(12.0);
                        egui::Frame::new()
                            .fill(colors.danger.gamma_multiply(0.12))
                            .stroke(egui::Stroke::new(1.0, colors.danger.gamma_multiply(0.5)))
                            .corner_radius(egui::CornerRadius::same(8))
                            .inner_margin(egui::Margin::same(10))
                            .show(ui, |ui| {
                                ui.colored_label(colors.danger, error);
                            });
                    }

                    ui.add_space(18.0);
                    ui.horizontal(|ui| {
                        if ui.add(save_button(colors)).clicked() {
                            action = EditorAction::Save;
                        }
                        if ui.button("Отмена").clicked() {
                            action = EditorAction::Cancel;
                        }
                    });
                });
        });

    action
}

fn save_button(colors: theme::ThemeColors) -> egui::Button<'static> {
    let text_color = if colors.is_dark {
        Color32::from_rgb(20, 20, 20)
    } else {
        Color32::WHITE
    };
    egui::Button::new(RichText::new("Сохранить").strong().color(text_color)).fill(colors.accent)
}

fn poster_column(
    ui: &mut egui::Ui,
    state: &mut EditorState,
    paths: &AppPaths,
    posters: &mut PosterCache,
) {
    ui.vertical(|ui| {
        ui.set_width(POSTER_WIDTH);
        ui.label(RichText::new("Обложка").strong().size(16.0));
        ui.add_space(6.0);

        let texture = match &state.pending_poster {
            Some(PendingPoster::File(path)) => posters.texture_from_path(ui.ctx(), path, 900),
            Some(PendingPoster::Clipboard { image }) => {
                if state.pending_texture.is_none() {
                    state.pending_texture = poster_cache::texture_from_rgba(
                        ui.ctx(),
                        "clipboard-poster-preview",
                        image,
                        900,
                    );
                }
                state.pending_texture.clone()
            }
            None => state
                .visible_poster()
                .and_then(|stored| posters.texture(ui.ctx(), paths, stored, 900)),
        };

        widgets::paint_poster(
            ui,
            egui::vec2(POSTER_WIDTH, POSTER_HEIGHT),
            texture.as_ref(),
        );

        ui.add_space(8.0);

        ui.horizontal_wrapped(|ui| {
            if ui.button("Выбрать файл…").clicked() {
                action_pick_poster(state);
            }
            if ui.button("Вставить из буфера").clicked() {
                paste_poster(ui.ctx(), state);
            }
        });

        let can_remove = state.pending_poster.is_some()
            || (state.original_poster.is_some() && !state.remove_poster);
        if can_remove
            && ui
                .button(RichText::new("Убрать обложку").color(theme::DANGER))
                .on_hover_text(if state.pending_poster.is_some() {
                    "Сначала отменить выбранную обложку; повторное нажатие удалит сохранённую"
                } else {
                    "Удалить сохранённую обложку"
                })
                .clicked()
        {
            if state.pending_poster.is_some() {
                // Не удаляем старую обложку из-за отмены новой.
                state.pending_poster = None;
                state.pending_texture = None;
                state.remove_poster = false;
            } else {
                state.remove_poster = true;
            }
        }

        if state.remove_poster
            && state.original_poster.is_some()
            && ui.button("Вернуть прежнюю").clicked()
        {
            state.remove_poster = false;
        }

        ui.add_space(6.0);

        let caption = match &state.pending_poster {
            Some(PendingPoster::File(path)) => path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("выбранный файл")
                .to_owned(),
            Some(PendingPoster::Clipboard { .. }) => "Изображение из буфера обмена".to_owned(),
            None if state.remove_poster => "Обложка будет удалена".to_owned(),
            None => state
                .original_poster
                .as_deref()
                .map(|stored| {
                    storage::poster_file_name(stored)
                        .unwrap_or(stored)
                        .to_owned()
                })
                .unwrap_or_else(|| "Обложка не выбрана".to_owned()),
        };

        ui.add(
            egui::Label::new(RichText::new(caption).small().color(theme::TEXT_MUTED))
                .wrap_mode(egui::TextWrapMode::Truncate),
        );

        ui.label(
            RichText::new(
                "Можно выбрать файл или вставить картинку из буфера обмена (Ctrl+V). \
                 Копия сохраняется в папке приложения.",
            )
            .small()
            .color(theme::TEXT_MUTED),
        );
    });
}

fn action_pick_poster(state: &mut EditorState) {
    let selected = rfd::FileDialog::new()
        .set_title("Выберите обложку фильма")
        .add_filter("Изображения", &["png", "jpg", "jpeg", "webp", "bmp", "gif"])
        .pick_file();

    let Some(path) = selected else {
        return;
    };

    match storage::validate_poster(&path) {
        Ok(_) => {
            state.pending_poster = Some(PendingPoster::File(path));
            state.pending_texture = None;
            state.remove_poster = false;
            state.form_error = None;
        }
        Err(error) => {
            state.form_error = Some(format!("Не удалось использовать изображение: {error}"));
        }
    }
}

enum ClipboardPoster {
    Image(image::RgbaImage),
    File(PathBuf),
}

fn paste_poster(ctx: &egui::Context, state: &mut EditorState) {
    match read_clipboard_poster() {
        Ok(ClipboardPoster::Image(image)) => {
            state.pending_texture =
                poster_cache::texture_from_rgba(ctx, "clipboard-poster-preview", &image, 900);
            state.pending_poster = Some(PendingPoster::Clipboard {
                image: Arc::new(image),
            });
            state.remove_poster = false;
            state.form_error = None;
        }
        Ok(ClipboardPoster::File(path)) => {
            state.pending_poster = Some(PendingPoster::File(path));
            state.pending_texture = None;
            state.remove_poster = false;
            state.form_error = None;
        }
        Err(error) => {
            state.form_error = Some(format!("Не удалось вставить обложку: {error}"));
        }
    }
}

fn read_clipboard_poster() -> Result<ClipboardPoster, AppError> {
    let mut clipboard = arboard::Clipboard::new()?;

    match clipboard.get_image() {
        Ok(data) => {
            let width = u32::try_from(data.width).map_err(|_| AppError::ClipboardInvalidImage)?;
            let height = u32::try_from(data.height).map_err(|_| AppError::ClipboardInvalidImage)?;
            storage::validate_image_size(width, height)?;

            let image = image::RgbaImage::from_raw(width, height, data.bytes.into_owned())
                .ok_or(AppError::ClipboardInvalidImage)?;
            Ok(ClipboardPoster::Image(image))
        }
        Err(error) => {
            // Если в буфере лежит путь к картинке (скопирован как текст),
            // принимаем и его — это удобно при работе с проводником.
            // Linux-файловые менеджеры копируют `file://` URI списком.
            if let Ok(text) = clipboard.get_text()
                && let Some(candidate) = storage::path_from_clipboard_text(&text)
                && candidate.is_file()
                && storage::validate_poster(&candidate).is_ok()
            {
                return Ok(ClipboardPoster::File(candidate));
            }

            if matches!(error, arboard::Error::ContentNotAvailable) {
                Err(AppError::ClipboardNoImage)
            } else {
                Err(error.into())
            }
        }
    }
}

fn add_custom_category(state: &mut EditorState) {
    let category = state.new_category.trim();
    if category.is_empty() {
        state.form_error = Some("Название категории не может быть пустым".to_owned());
        return;
    }
    if category.chars().count() > NewFilm::MAX_CATEGORY_CHARS {
        state.form_error = Some(format!(
            "Название категории слишком длинное: максимум {} символов",
            NewFilm::MAX_CATEGORY_CHARS
        ));
        return;
    }

    state.category = category.to_owned();
    state.new_category.clear();
    state.form_error = None;
}

fn seasons_editor(ui: &mut egui::Ui, state: &mut EditorState) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Сезоны").strong());
        ui.add(
            egui::DragValue::new(&mut state.seasons_total)
                .range(0..=MAX_SEASONS)
                .speed(0.2),
        );
        ui.label(
            RichText::new("0 — не сериал / без сезонов")
                .small()
                .color(theme::TEXT_MUTED),
        );
    });

    // При уменьшении количества сезонов убираем отметки за пределами диапазона.
    if state.seasons_total == 0 {
        state.watched_seasons.clear();
        return;
    }

    let total = state.seasons_total;
    state
        .watched_seasons
        .retain(|season| *season >= 1 && *season <= total);

    let colors = theme::colors(ui.ctx());
    let watched_count = state.watched_seasons.len();
    let fully_watched = watched_count >= total as usize;

    ui.add_space(2.0);
    ui.label(
        RichText::new(if fully_watched {
            format!("Все {total} сезонов просмотрены")
        } else {
            format!("Просмотрено {watched_count} из {total} — сериал не досмотрен")
        })
        .small()
        .color(if fully_watched {
            colors.success
        } else {
            colors.accent
        }),
    );

    ui.horizontal(|ui| {
        if ui.button("Отметить все").clicked() {
            state.watched_seasons = (1..=total).collect();
        }
        if ui.button("Снять все").clicked() {
            state.watched_seasons.clear();
        }
    });

    ui.horizontal_wrapped(|ui| {
        for season in 1..=total {
            let watched = state.watched_seasons.contains(&season);
            let text_color = if watched {
                if colors.is_dark {
                    Color32::from_rgb(20, 20, 20)
                } else {
                    Color32::WHITE
                }
            } else {
                colors.text_muted
            };
            let button = egui::Button::new(
                RichText::new(season.to_string())
                    .size(13.0)
                    .strong()
                    .color(text_color),
            )
            .fill(if watched {
                colors.accent
            } else {
                colors.surface_hover
            })
            .stroke(egui::Stroke::new(
                1.0,
                if watched {
                    colors.accent
                } else {
                    colors.border
                },
            ))
            .min_size(egui::vec2(30.0, 26.0));

            let response = ui.add(button).on_hover_text(format!(
                "Сезон {season} — {}",
                if watched {
                    "просмотрен"
                } else {
                    "не просмотрен"
                }
            ));

            if response.clicked() {
                if watched {
                    state.watched_seasons.retain(|item| *item != season);
                } else {
                    state.watched_seasons.push(season);
                    state.watched_seasons.sort_unstable();
                }
            }
        }
    });

    ui.label(
        RichText::new("Нажмите на номер сезона, чтобы отметить его просмотренным.")
            .small()
            .color(theme::TEXT_MUTED),
    );
}

/// Есть ли тег в строке `"a, b, c"` (без учёта регистра).
fn tag_is_selected(tags: &str, candidate: &str) -> bool {
    let candidate = candidate.trim().to_lowercase();
    split_comma_list(tags)
        .iter()
        .any(|tag| tag.to_lowercase() == candidate)
}

/// Дописывает тег в строку, не создавая дублей и лишних запятых.
fn add_tag(tags: &mut String, tag: &str) {
    let tag = tag.trim();
    if tag.is_empty() || tag_is_selected(tags, tag) {
        return;
    }

    let existing = tags.trim().trim_end_matches(',').trim_end();
    *tags = if existing.is_empty() {
        tag.to_owned()
    } else {
        format!("{existing}, {tag}")
    };
}

fn relations_editor(ui: &mut egui::Ui, state: &mut EditorState, action: &mut EditorAction) {
    let colors = theme::colors(ui.ctx());
    ui.separator();
    ui.add_space(8.0);
    ui.label(
        RichText::new("Связи с другими фильмами")
            .strong()
            .size(16.0)
            .color(colors.text),
    );
    ui.label(
        RichText::new("Например: «Дюна: Часть вторая» — сиквел «Дюны».")
            .small()
            .color(colors.text_muted),
    );
    ui.add_space(4.0);

    let Some(source_id) = state.film_id else {
        ui.label(
            RichText::new("Сначала сохраните фильм — после этого можно добавить связи.")
                .small()
                .color(colors.text_muted),
        );
        return;
    };

    if state.relation_candidates.is_empty() {
        ui.label(
            RichText::new("В библиотеке пока нет других фильмов для связи.")
                .small()
                .color(colors.text_muted),
        );
        return;
    }

    let selected_target = state
        .relation_target
        .and_then(|id| {
            state
                .relation_candidates
                .iter()
                .find(|(candidate_id, _)| *candidate_id == id)
                .map(|(_, title)| title.clone())
        })
        .unwrap_or_else(|| "Выберите фильм".to_owned());

    ui.horizontal_wrapped(|ui| {
        egui::ComboBox::from_id_salt("editor_relation_kind")
            .width(170.0)
            .selected_text(state.relation_kind.label())
            .show_ui(ui, |ui| {
                for kind in RelationKind::ALL {
                    ui.selectable_value(&mut state.relation_kind, kind, kind.label());
                }
            });

        egui::ComboBox::from_id_salt("editor_relation_target")
            .width(280.0)
            .selected_text(selected_target)
            .show_ui(ui, |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut state.relation_search)
                        .hint_text("Поиск по названию…")
                        .desired_width(f32::INFINITY),
                );

                let needle = state.relation_search.trim().to_lowercase();
                egui::ScrollArea::vertical()
                    .max_height(240.0)
                    .show(ui, |ui| {
                        let mut shown = 0;
                        for (id, title) in &state.relation_candidates {
                            if !needle.is_empty() && !title.to_lowercase().contains(&needle) {
                                continue;
                            }
                            ui.selectable_value(&mut state.relation_target, Some(*id), title);
                            shown += 1;
                        }
                        if shown == 0 {
                            ui.label(
                                RichText::new("Ничего не найдено")
                                    .small()
                                    .color(colors.text_muted),
                            );
                        }
                    });
            });

        if ui
            .add_enabled(
                state.relation_target.is_some(),
                egui::Button::new("Добавить связь"),
            )
            .clicked()
            && let Some(target_id) = state.relation_target
        {
            *action = EditorAction::AddRelation {
                source_id,
                target_id,
                kind: state.relation_kind,
            };
        }
    });

    ui.add_space(6.0);

    if state.relations.is_empty() {
        ui.label(
            RichText::new("Связей пока нет.")
                .small()
                .color(colors.text_muted),
        );
    } else {
        for relation in &state.relations {
            ui.horizontal(|ui| {
                let fill_alpha = if colors.is_dark { 0.16 } else { 0.12 };
                let stroke_alpha = if colors.is_dark { 0.5 } else { 0.35 };
                egui::Frame::new()
                    .fill(colors.accent.gamma_multiply(fill_alpha))
                    .stroke(egui::Stroke::new(
                        1.0,
                        colors.accent.gamma_multiply(stroke_alpha),
                    ))
                    .corner_radius(egui::CornerRadius::same(6))
                    .inner_margin(egui::Margin::symmetric(8, 3))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(relation.label())
                                .size(11.0)
                                .strong()
                                .color(colors.accent),
                        );
                    });

                if ui
                    .link(RichText::new(&relation.film.title).strong())
                    .on_hover_text(
                        "Открыть страницу связанного фильма. \
                         Несохранённые изменения текущего фильма будут потеряны.",
                    )
                    .clicked()
                {
                    *action = EditorAction::OpenRelated(relation.film.id);
                }

                if let Some(year) = relation.film.year {
                    ui.label(
                        RichText::new(year.to_string())
                            .small()
                            .color(colors.text_muted),
                    );
                }
                if let Some(runtime) = relation.film.runtime_label() {
                    ui.label(RichText::new(runtime).small().color(colors.text_muted));
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .small_button(RichText::new("Удалить").color(colors.danger))
                        .clicked()
                    {
                        *action = EditorAction::RemoveRelation(relation.relation_id);
                    }
                });
            });
        }
    }

    ui.add_space(4.0);
    ui.label(
        RichText::new("Связи сохраняются сразу, независимо от кнопки «Сохранить».")
            .small()
            .color(colors.text_muted),
    );
}

fn fields_column(
    ui: &mut egui::Ui,
    state: &mut EditorState,
    categories: &[String],
    tag_suggestions: &[String],
    metadata_busy: bool,
    action: &mut EditorAction,
) {
    ui.vertical(|ui| {
        ui.label(RichText::new("Название *").strong());
        ui.add(
            egui::TextEdit::singleline(&mut state.title)
                .hint_text("Например, «Дюна»")
                .desired_width(f32::INFINITY),
        );

        ui.add_space(6.0);

        ui.label(RichText::new("Оригинальное название").strong());
        ui.add(
            egui::TextEdit::singleline(&mut state.original_title)
                .hint_text("Например, «Dune: Part Two»")
                .desired_width(f32::INFINITY),
        );

        ui.add_space(6.0);

        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(!metadata_busy, egui::Button::new("Заполнить из OMDb"))
                .on_hover_text(
                    "Год, жанры, режиссёр, страна, длительность, описание, IMDb ID и обложка. \
                     Заполняются только пустые поля — введённые вручную данные не меняются. \
                     Нужен бесплатный ключ OMDb в «Настройках».",
                )
                .clicked()
            {
                *action = EditorAction::Autofill;
            }

            if metadata_busy {
                ui.spinner();
                ui.label(
                    RichText::new("Запрос к OMDb…")
                        .small()
                        .color(theme::TEXT_MUTED),
                );
            } else {
                ui.label(
                    RichText::new("Лучше искать по оригинальному названию")
                        .small()
                        .color(theme::TEXT_MUTED),
                );
            }
        });

        ui.add_space(6.0);

        ui.label(RichText::new("Категория").strong());
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("editor_category")
                .width(200.0)
                .selected_text(&state.category)
                .show_ui(ui, |ui| {
                    for category in categories {
                        ui.selectable_value(
                            &mut state.category,
                            category.clone(),
                            category.as_str(),
                        );
                    }
                });

            let response = ui.add(
                egui::TextEdit::singleline(&mut state.new_category)
                    .hint_text("Своя категория…")
                    .desired_width(190.0),
            );
            let submitted =
                response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
            if ui.button("Добавить").clicked() || submitted {
                add_custom_category(state);
            }
        });

        ui.add_space(6.0);

        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new("Год").strong());
                let mut year = state.year.unwrap_or(0);
                if ui
                    .add(
                        egui::DragValue::new(&mut year)
                            .range(0..=MAX_YEAR)
                            .speed(0.5),
                    )
                    .changed()
                {
                    state.year = (year > 0).then_some(year);
                }
                ui.label(
                    RichText::new("0 — не указан")
                        .small()
                        .color(theme::TEXT_MUTED),
                );
            });

            ui.add_space(24.0);

            ui.vertical(|ui| {
                ui.label(RichText::new("Длительность").strong());
                let mut runtime = state.runtime_minutes.unwrap_or(0);
                if ui
                    .add(
                        egui::DragValue::new(&mut runtime)
                            .range(0..=MAX_RUNTIME_MINUTES)
                            .speed(0.5)
                            .suffix(" мин"),
                    )
                    .changed()
                {
                    state.runtime_minutes = (runtime > 0).then_some(runtime);
                }
                ui.label(
                    RichText::new("0 — не указана")
                        .small()
                        .color(theme::TEXT_MUTED),
                );
            });
        });

        ui.add_space(6.0);

        ui.label(RichText::new("Жанры").strong());
        ui.add(
            egui::TextEdit::singleline(&mut state.genres)
                .hint_text("Например, «Фантастика, Приключения»")
                .desired_width(f32::INFINITY),
        );

        ui.add_space(6.0);

        ui.label(RichText::new("Теги").strong());
        ui.add(
            egui::TextEdit::singleline(&mut state.tags)
                .hint_text("Например, «на пересмотр, с друзьями»")
                .desired_width(f32::INFINITY),
        );
        ui.label(
            RichText::new("Свои метки через запятую: настроение, компания, повод пересмотреть.")
                .small()
                .color(theme::TEXT_MUTED),
        );

        let suggestions: Vec<&String> = tag_suggestions
            .iter()
            .filter(|tag| !tag_is_selected(&state.tags, tag))
            .take(12)
            .collect();
        if !suggestions.is_empty() {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("Быстро добавить:")
                        .small()
                        .color(theme::TEXT_MUTED),
                );
                for tag in suggestions {
                    if ui.small_button(format!("+ {tag}")).clicked() {
                        add_tag(&mut state.tags, tag);
                    }
                }
            });
        }

        ui.add_space(6.0);

        ui.label(RichText::new("Страна").strong());
        ui.add(
            egui::TextEdit::singleline(&mut state.country)
                .hint_text("Например, «США»")
                .desired_width(f32::INFINITY),
        );

        ui.add_space(6.0);

        ui.label(RichText::new("Режиссёр").strong());
        ui.add(
            egui::TextEdit::singleline(&mut state.director)
                .hint_text("Например, «Дени Вильнёв»")
                .desired_width(f32::INFINITY),
        );

        if !state.imdb_id.trim().is_empty() {
            ui.label(
                RichText::new(format!("IMDb: {}", state.imdb_id))
                    .small()
                    .color(theme::TEXT_MUTED),
            );
        }

        ui.add_space(6.0);

        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new("Статус").strong());
                egui::ComboBox::from_id_salt("editor_status")
                    .width(220.0)
                    .selected_text(state.status.label())
                    .show_ui(ui, |ui| {
                        for status in FilmStatus::ALL {
                            ui.selectable_value(&mut state.status, status, status.label());
                        }
                    });
            });

            ui.add_space(24.0);

            ui.vertical(|ui| {
                ui.label(RichText::new("Оценка").strong());

                let mut rated = state.rating.is_some();
                if ui.checkbox(&mut rated, "Оценить").changed() {
                    state.rating = if rated { Some(7) } else { None };
                }

                if let Some(rating) = state.rating.as_mut() {
                    ui.spacing_mut().slider_width = 200.0;
                    ui.add(egui::Slider::new(rating, 1..=10).text("из 10"));
                } else {
                    ui.label(RichText::new("Без оценки").small().color(theme::TEXT_MUTED));
                }
            });
        });

        ui.add_space(8.0);

        seasons_editor(ui, state);

        ui.add_space(8.0);

        ui.label(RichText::new("Краткое описание").strong());
        ui.add(
            egui::TextEdit::multiline(&mut state.description)
                .hint_text("О чём фильм…")
                .desired_rows(4)
                .desired_width(f32::INFINITY),
        );

        ui.add_space(8.0);

        ui.label(RichText::new("Мои мысли / рецензия").strong());
        ui.add(
            egui::TextEdit::multiline(&mut state.thoughts)
                .hint_text("Что понравилось, что нет, стоит ли пересматривать…")
                .desired_rows(9)
                .desired_width(f32::INFINITY),
        );

        ui.add_space(4.0);
        ui.label(
            RichText::new("Все данные хранятся локально в library.db.")
                .small()
                .color(theme::TEXT_MUTED),
        );
    });
}

fn quotes_editor(ui: &mut egui::Ui, state: &mut EditorState) {
    let colors = theme::colors(ui.ctx());
    ui.separator();
    ui.add_space(8.0);
    ui.label(
        RichText::new("Цитаты и любимые моменты")
            .strong()
            .size(16.0)
            .color(colors.text),
    );
    ui.label(
        RichText::new("Запоминающиеся реплики и фразы из фильма.")
            .small()
            .color(colors.text_muted),
    );
    ui.add_space(6.0);

    ui.horizontal(|ui| {
        let text_res = ui.add(
            egui::TextEdit::singleline(&mut state.new_quote_input)
                .hint_text("Введите цитату...")
                .desired_width(450.0),
        );
        let add_clicked = ui.button("+ Добавить").clicked();
        let enter_pressed = text_res.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

        if (add_clicked || enter_pressed) && !state.new_quote_input.trim().is_empty() {
            state.quotes.push(state.new_quote_input.trim().to_owned());
            state.new_quote_input.clear();
        }
    });

    if !state.quotes.is_empty() {
        ui.add_space(8.0);
        let mut to_remove = None;
        for (idx, quote) in state.quotes.iter().enumerate() {
            egui::Frame::new()
                .fill(colors.card_bg)
                .stroke(egui::Stroke::new(1.0, colors.card_stroke))
                .corner_radius(egui::CornerRadius::same(6))
                .inner_margin(egui::Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (rect, _) = ui
                            .allocate_exact_size(egui::Vec2::new(3.0, 18.0), egui::Sense::hover());
                        ui.painter()
                            .rect_filled(rect, egui::CornerRadius::same(1), colors.accent);
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new(format!("«{quote}»"))
                                .italics()
                                .color(colors.text),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .small_button("x")
                                .on_hover_text("Удалить цитату")
                                .clicked()
                            {
                                to_remove = Some(idx);
                            }
                        });
                    });
                });
            ui.add_space(4.0);
        }
        if let Some(idx) = to_remove {
            state.quotes.remove(idx);
        }
    }
}

fn screenshots_gallery(
    ui: &mut egui::Ui,
    state: &mut EditorState,
    paths: &AppPaths,
    posters: &mut PosterCache,
    action: &mut EditorAction,
) {
    let colors = theme::colors(ui.ctx());
    ui.separator();
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Кадры и скриншоты")
                .strong()
                .size(16.0)
                .color(colors.text),
        );
        ui.add_space(8.0);

        if state.film_id.is_some() {
            if ui.button("+ Добавить кадр").clicked()
                && let Some(file) = rfd::FileDialog::new()
                    .add_filter("Изображения", &["png", "jpg", "jpeg", "webp", "bmp", "gif"])
                    .pick_file()
            {
                *action = EditorAction::AddScreenshotFile(file);
            }

            if ui.button("Вставить из буфера").clicked()
                && let Ok(mut clipboard) = arboard::Clipboard::new()
                && let Ok(img) = clipboard.get_image()
            {
                let rgba = image::RgbaImage::from_raw(
                    img.width as u32,
                    img.height as u32,
                    img.bytes.into_owned(),
                );
                if let Some(rgba) = rgba {
                    *action = EditorAction::AddScreenshotClipboard(Arc::new(rgba));
                }
            }
        }
    });

    if state.film_id.is_none() {
        ui.label(
            RichText::new("Сначала сохраните фильм — после этого можно добавлять кадры.")
                .small()
                .color(colors.text_muted),
        );
        return;
    }

    if state.screenshots.is_empty() {
        ui.add_space(4.0);
        ui.label(
            RichText::new("К фильму пока не добавлено ни одного кадра.")
                .small()
                .color(colors.text_muted),
        );
        return;
    }

    ui.add_space(8.0);
    let mut delete_id = None;

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::Vec2::new(10.0, 10.0);
        for shot in &state.screenshots {
            egui::Frame::new()
                .fill(colors.card_bg)
                .stroke(egui::Stroke::new(1.0, colors.card_stroke))
                .corner_radius(egui::CornerRadius::same(8))
                .inner_margin(egui::Margin::same(6))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.set_width(180.0);
                        let texture =
                            posters.screenshot_texture(ui.ctx(), paths, &shot.image_path, 360);
                        widgets::paint_poster(ui, egui::vec2(180.0, 110.0), texture.as_ref());

                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            if !shot.caption.is_empty() {
                                ui.label(
                                    RichText::new(&shot.caption)
                                        .size(11.0)
                                        .color(colors.text_muted),
                                );
                            }
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.small_button("x").on_hover_text("Удалить кадр").clicked()
                                    {
                                        delete_id = Some(shot.id);
                                    }
                                },
                            );
                        });
                    });
                });
        }
    });

    if let Some(id) = delete_id {
        *action = EditorAction::DeleteScreenshot(id);
    }
}
