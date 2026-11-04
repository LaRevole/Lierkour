//! Состояние приложения и связка UI ↔ база данных.

use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, RichText};

use crate::db::Database;
use crate::error::AppError;
use crate::metadata::{self, MovieMetadata};
use crate::models::{DEFAULT_CATEGORIES, Film, FilmFilter, RelatedFilm, RelationKind};
use crate::nav;
use crate::storage::{self, AppPaths};
use crate::transfer;
use crate::ui::editor::{self, EditorAction, EditorState, PendingPoster};
use crate::ui::library::{self, LibraryAction, LibraryState};
use crate::ui::poster_cache::{self, PosterCache};
use crate::ui::theme;

pub struct FilmApp {
    paths: AppPaths,
    db: Option<Database>,
    startup_error: Option<String>,

    films: Vec<Film>,
    categories: Vec<String>,
    tag_suggestions: Vec<String>,
    filter: FilmFilter,
    library_state: LibraryState,
    screen: Screen,
    /// История переходов для кнопок «назад/вперёд» мыши и жестов тачпада.
    back_stack: Vec<Screen>,
    forward_stack: Vec<Screen>,
    /// Счётчик уникальных идентификаторов прокрутки страниц.
    next_page_salt: u64,
    navigation: nav::Detector,
    poster_cache: PosterCache,

    toast: Option<Toast>,
    confirm_delete: Option<Film>,

    settings_open: bool,
    settings_key: String,
    metadata_job: Option<MetadataJob>,
}

enum Screen {
    Library,
    Editor(Box<EditorState>),
}

enum AppAction {
    AddNew,
    Edit(i64),
    AskDeleteById(i64),
    MoveFilm {
        film_id: i64,
        target_id: i64,
    },
    SaveEditor,
    CancelEditor,
    Autofill,
    AddRelation {
        source_id: i64,
        target_id: i64,
        kind: RelationKind,
    },
    RemoveRelation(i64),
    OpenRelated(i64),
    Export,
    Import,
    OpenSettings,
}

struct MetadataJob {
    rx: Receiver<Result<MovieMetadata, metadata::MetadataError>>,
    /// Какой фильм был открыт в момент запроса.
    film_id: Option<i64>,
    /// По какому названию искали — чтобы не применить ответ к другой карточке.
    query: String,
}

struct Toast {
    message: String,
    is_error: bool,
    until: Instant,
}

impl FilmApp {
    pub fn new(cc: &eframe::CreationContext<'_>, paths: AppPaths) -> Self {
        theme::apply(&cc.egui_ctx);

        let mut app = Self {
            paths,
            db: None,
            startup_error: None,
            films: Vec::new(),
            categories: DEFAULT_CATEGORIES
                .iter()
                .map(|category| (*category).to_owned())
                .collect(),
            tag_suggestions: Vec::new(),
            filter: FilmFilter::default(),
            library_state: LibraryState::default(),
            screen: Screen::Library,
            back_stack: Vec::new(),
            forward_stack: Vec::new(),
            next_page_salt: 0,
            navigation: nav::Detector::default(),
            poster_cache: PosterCache::new(),
            toast: None,
            confirm_delete: None,
            settings_open: false,
            settings_key: String::new(),
            metadata_job: None,
        };

        match Database::open(&app.paths.db_path) {
            Ok(db) => {
                app.db = Some(db);
                app.reload_films();
                app.reload_categories();
            }
            Err(error) => {
                app.startup_error = Some(error.to_string());
            }
        }

        app
    }

    fn reload_films(&mut self) -> bool {
        let result = match self.db.as_ref() {
            Some(db) => db.list_films(&self.filter),
            None => return false,
        };

        match result {
            Ok(films) => {
                self.films = films;
                self.reload_tag_suggestions();
                true
            }
            Err(error) => {
                self.notify(
                    format!("Не удалось загрузить список фильмов: {error}"),
                    true,
                );
                false
            }
        }
    }

    /// Собирает все теги библиотеки для подсказок в редакторе.
    /// Ошибку здесь не показываем: подсказки не критичны для работы.
    fn reload_tag_suggestions(&mut self) {
        let Some(db) = self.db.as_ref() else {
            return;
        };
        let Ok(films) = db.list_all_films() else {
            return;
        };

        let mut tags: Vec<String> = Vec::new();
        for film in &films {
            for tag in film.tags_list() {
                let lowercase = tag.to_lowercase();
                if !tags.iter().any(|known| known.to_lowercase() == lowercase) {
                    tags.push(tag.to_owned());
                }
            }
        }
        tags.sort_by_cached_key(|tag| tag.to_lowercase());
        self.tag_suggestions = tags;
    }

    fn reload_categories(&mut self) {
        let result = match self.db.as_ref() {
            Some(db) => db.list_categories(),
            None => return,
        };

        match result {
            Ok(categories) => self.categories = categories,
            Err(error) => {
                self.notify(format!("Не удалось загрузить категории: {error}"), true);
            }
        }
    }

    fn retry_startup(&mut self) {
        match Database::open(&self.paths.db_path) {
            Ok(db) => {
                self.db = Some(db);
                self.startup_error = None;
                self.reload_films();
                self.reload_categories();
                self.notify("База данных открыта", false);
            }
            Err(error) => self.startup_error = Some(error.to_string()),
        }
    }

    /// Выдаёт уникальный идентификатор прокрутки для только что открытой страницы.
    /// Благодаря этому новая карточка открывается сверху, а не с позиции,
    /// на которой остался предыдущий экран.
    fn alloc_scroll_salt(&mut self) -> u64 {
        self.next_page_salt = self.next_page_salt.wrapping_add(1).max(1);
        self.next_page_salt
    }

    /// Открывает новый экран и запоминает текущий для перехода «назад».
    fn open_screen(&mut self, screen: Screen) {
        let previous = std::mem::replace(&mut self.screen, screen);
        self.back_stack.push(previous);
        // Длинная сессия не должна бесконечно копить состояния редакторов.
        if self.back_stack.len() > 32 {
            self.back_stack.remove(0);
        }
        self.forward_stack.clear();
    }

    /// Возврат в библиотеку после сохранения, отмены или удаления.
    /// История при этом очищается: возвращаться в уже сохранённую карточку не нужно.
    fn return_to_library(&mut self) {
        self.screen = Screen::Library;
        self.back_stack.clear();
        self.forward_stack.clear();
    }

    fn navigate(&mut self, direction: nav::Direction) {
        let target = match direction {
            nav::Direction::Back => self.back_stack.pop(),
            nav::Direction::Forward => self.forward_stack.pop(),
        };
        let Some(target) = target else {
            return;
        };

        let current = std::mem::replace(&mut self.screen, target);
        match direction {
            nav::Direction::Back => self.forward_stack.push(current),
            nav::Direction::Forward => self.back_stack.push(current),
        }

        // Список мог измениться, пока мы были в редакторе.
        if matches!(self.screen, Screen::Library) {
            self.reload_films();
        }
    }

    /// Обрабатывает боковые кнопки мыши и горизонтальные жесты тачпада.
    fn poll_navigation(&mut self, ctx: &egui::Context) {
        // eframe может выполнить несколько проходов UI за один кадр.
        // Обрабатываем ввод только в первом, иначе одно нажатие
        // переключит сразу две страницы.
        if ctx.current_pass_index() != 0 {
            return;
        }

        let enabled =
            self.startup_error.is_none() && !self.settings_open && self.confirm_delete.is_none();
        let line_scroll_speed = ctx.options(|options| options.input_options.line_scroll_speed);

        let direction = ctx.input(|input| {
            self.navigation.poll(
                &input.events,
                input.time,
                input.viewport_rect().height(),
                line_scroll_speed,
                enabled,
            )
        });

        if let Some(direction) = direction {
            self.navigate(direction);
        }
    }

    fn open_new_editor(&mut self) {
        let editor = EditorState {
            scroll_salt: self.alloc_scroll_salt(),
            ..Default::default()
        };
        self.open_screen(Screen::Editor(Box::new(editor)));
    }

    fn open_editor(&mut self, id: i64) {
        let result = match self.db.as_ref() {
            Some(db) => (|| -> Result<(Film, Vec<RelatedFilm>, Vec<Film>), AppError> {
                let film = db.get_film(id)?;
                let relations = db.relations_for(id)?;
                let all_films = db.list_all_films()?;
                Ok((film, relations, all_films))
            })(),
            None => Err(AppError::DatabaseUnavailable),
        };

        match result {
            Ok((film, relations, all_films)) => {
                let mut editor = EditorState::from_film(&film);
                editor.relations = relations;
                editor.relation_candidates = relation_candidates(&all_films, film.id);
                editor.scroll_salt = self.alloc_scroll_salt();
                self.open_screen(Screen::Editor(Box::new(editor)));
            }
            Err(error) => {
                self.notify(format!("Не удалось открыть фильм: {error}"), true);
            }
        }
    }

    fn refresh_editor_relations(&mut self) {
        let Some(film_id) = (match &self.screen {
            Screen::Editor(editor) => editor.film_id,
            Screen::Library => None,
        }) else {
            return;
        };

        let result = match self.db.as_ref() {
            Some(db) => (|| -> Result<(Vec<RelatedFilm>, Vec<Film>), AppError> {
                Ok((db.relations_for(film_id)?, db.list_all_films()?))
            })(),
            None => Err(AppError::DatabaseUnavailable),
        };

        match result {
            Ok((relations, all_films)) => {
                if let Screen::Editor(editor) = &mut self.screen {
                    editor.relations = relations;
                    editor.relation_candidates = relation_candidates(&all_films, film_id);
                    editor.relation_target = None;
                    editor.relation_search.clear();
                }
            }
            Err(error) => {
                self.notify(format!("Не удалось загрузить связи: {error}"), true);
            }
        }
    }

    fn add_relation(&mut self, source_id: i64, target_id: i64, kind: RelationKind) {
        let result = match self.db.as_mut() {
            Some(db) => db.add_relation(source_id, target_id, kind),
            None => Err(AppError::DatabaseUnavailable),
        };

        match result {
            Ok(true) => {
                self.refresh_editor_relations();
                self.notify("Связь добавлена", false);
            }
            Ok(false) => {
                self.refresh_editor_relations();
                self.notify("Такая связь уже существует", false);
            }
            Err(error) => {
                self.notify(format!("Не удалось добавить связь: {error}"), true);
            }
        }
    }

    fn remove_relation(&mut self, relation_id: i64) {
        let result = match self.db.as_mut() {
            Some(db) => db.remove_relation(relation_id),
            None => Err(AppError::DatabaseUnavailable),
        };

        match result {
            Ok(()) => {
                self.refresh_editor_relations();
                self.notify("Связь удалена", false);
            }
            Err(error) => {
                self.notify(format!("Не удалось удалить связь: {error}"), true);
            }
        }
    }

    fn start_autofill(&mut self) {
        if self.metadata_job.is_some() {
            self.notify("Запрос к OMDb уже выполняется", false);
            return;
        }

        let api_key = match self.db.as_ref() {
            Some(db) => db
                .get_setting(metadata::OMDB_API_KEY_SETTING)
                .ok()
                .flatten()
                .unwrap_or_default(),
            None => String::new(),
        };

        if api_key.trim().is_empty() {
            self.open_settings();
            self.notify("Сначала укажите бесплатный ключ OMDb в настройках", true);
            return;
        }

        let (query, year, film_id) = match &self.screen {
            Screen::Editor(editor) => {
                let query = if editor.original_title.trim().is_empty() {
                    editor.title.trim().to_owned()
                } else {
                    editor.original_title.trim().to_owned()
                };
                (query, editor.year, editor.film_id)
            }
            Screen::Library => return,
        };

        if query.is_empty() {
            self.notify("Введите название фильма для поиска в OMDb", true);
            return;
        }

        let (tx, rx) = mpsc::channel();
        let query_for_job = query.clone();
        std::thread::spawn(move || {
            let _ = tx.send(metadata::fetch(&api_key, &query, year));
        });

        self.metadata_job = Some(MetadataJob {
            rx,
            film_id,
            query: query_for_job,
        });
    }

    fn poll_metadata(&mut self, ctx: &egui::Context) {
        let Some(job) = self.metadata_job.take() else {
            return;
        };

        match job.rx.try_recv() {
            Ok(Ok(metadata)) => {
                // Пока шёл запрос, пользователь мог закрыть карточку
                // или открыть другую — тогда результат уже неактуален.
                let still_same_card = match &self.screen {
                    Screen::Editor(editor) => {
                        editor.film_id == job.film_id && editor_query(editor) == job.query
                    }
                    Screen::Library => false,
                };

                if still_same_card {
                    self.apply_metadata(ctx, metadata);
                } else if matches!(&self.screen, Screen::Editor(_)) {
                    self.notify(
                        "Автозаполнение: карточка изменилась, результат пропущен",
                        false,
                    );
                }
            }
            Ok(Err(error)) => {
                self.notify(format!("Автозаполнение: {error}"), true);
            }
            Err(TryRecvError::Empty) => {
                self.metadata_job = Some(job);
                ctx.request_repaint_after(Duration::from_millis(120));
            }
            Err(TryRecvError::Disconnected) => {
                self.notify("Автозаполнение прервано", true);
            }
        }
    }

    /// Заполняет только пустые поля, чтобы не перезаписывать данные,
    /// которые пользователь уже ввёл вручную.
    fn apply_metadata(&mut self, ctx: &egui::Context, metadata: MovieMetadata) {
        let Screen::Editor(editor) = &mut self.screen else {
            return;
        };

        let mut filled = 0;

        if !metadata.title.trim().is_empty() {
            if editor.title.trim().is_empty() {
                editor.title = metadata.title.clone();
                filled += 1;
            }
            if editor.original_title.trim().is_empty() {
                editor.original_title = metadata.title.clone();
                filled += 1;
            }
        }

        if editor.year.is_none() && metadata.year.is_some() {
            editor.year = metadata.year;
            filled += 1;
        }
        if editor.director.trim().is_empty() && !metadata.director.trim().is_empty() {
            editor.director = metadata.director.clone();
            filled += 1;
        }
        if editor.genres.trim().is_empty() && !metadata.genres.trim().is_empty() {
            editor.genres = metadata.genres.clone();
            filled += 1;
        }
        if editor.country.trim().is_empty() && !metadata.country.trim().is_empty() {
            editor.country = metadata.country.clone();
            filled += 1;
        }
        if editor.runtime_minutes.is_none() && metadata.runtime_minutes.is_some() {
            editor.runtime_minutes = metadata.runtime_minutes;
            filled += 1;
        }
        if editor.imdb_id.trim().is_empty() && !metadata.imdb_id.trim().is_empty() {
            editor.imdb_id = metadata.imdb_id.clone();
            filled += 1;
        }
        if editor.description.trim().is_empty() && !metadata.description.trim().is_empty() {
            editor.description = metadata.description.clone();
            filled += 1;
        }
        if editor.seasons_total == 0 && metadata.total_seasons > 0 {
            editor.seasons_total = metadata.total_seasons;
            filled += 1;
        }

        editor.form_error = None;

        // Обложку подставляем только если своей ещё нет.
        let can_take_poster = editor.visible_poster().is_none() && editor.pending_poster.is_none();
        let mut poster_applied = false;

        if let Some(bytes) = metadata.poster_bytes
            && can_take_poster
        {
            match image::load_from_memory(&bytes) {
                Ok(image) => {
                    let image = image.to_rgba8();
                    match storage::validate_image_size(image.width(), image.height()) {
                        Ok(()) => {
                            editor.pending_texture = poster_cache::texture_from_rgba(
                                ctx,
                                "omdb-poster-preview",
                                &image,
                                900,
                            );
                            editor.pending_poster = Some(PendingPoster::Clipboard {
                                image: Arc::new(image),
                            });
                            editor.remove_poster = false;
                            poster_applied = true;
                        }
                        Err(error) => {
                            editor.form_error = Some(format!(
                                "Метаданные заполнены, но обложка не подходит: {error}"
                            ));
                        }
                    }
                }
                Err(error) => {
                    editor.form_error = Some(format!(
                        "Метаданные заполнены, но обложку загрузить не удалось: {error}"
                    ));
                }
            }
        }

        let message = match (filled, poster_applied) {
            (0, false) => "OMDb: все поля уже заполнены — ничего не изменено".to_owned(),
            (0, true) => "OMDb: добавлена только обложка, поля уже были заполнены".to_owned(),
            (count, false) => {
                format!("OMDb: заполнено полей — {count}. Существующие значения сохранены")
            }
            (count, true) => format!(
                "OMDb: заполнено полей — {count}, добавлена обложка. Существующие значения сохранены"
            ),
        };
        self.notify(message, false);
    }

    fn open_settings(&mut self) {
        let key = match self.db.as_ref() {
            Some(db) => db
                .get_setting(metadata::OMDB_API_KEY_SETTING)
                .ok()
                .flatten()
                .unwrap_or_default(),
            None => String::new(),
        };
        self.settings_key = key;
        self.settings_open = true;
    }

    fn save_settings(&mut self) {
        let key = self.settings_key.trim().to_owned();
        let result = match self.db.as_mut() {
            Some(db) => db.set_setting(metadata::OMDB_API_KEY_SETTING, &key),
            None => Err(AppError::DatabaseUnavailable),
        };

        match result {
            Ok(()) => {
                self.settings_open = false;
                self.notify(
                    if key.is_empty() {
                        "Ключ OMDb удалён"
                    } else {
                        "Ключ OMDb сохранён"
                    },
                    false,
                );
            }
            Err(error) => {
                self.notify(format!("Не удалось сохранить настройки: {error}"), true);
            }
        }
    }

    fn export_library(&mut self) {
        let Some(db) = self.db.as_ref() else {
            self.notify("База данных недоступна", true);
            return;
        };

        let Some(path) = rfd::FileDialog::new()
            .set_title("Экспорт фильмографии")
            .set_file_name("filmography-backup.zip")
            .add_filter("ZIP-архив", &["zip"])
            .save_file()
        else {
            return;
        };

        match transfer::export_library(db, &self.paths, &path) {
            Ok(summary) => self.notify(
                format!(
                    "Экспортировано фильмов: {}, связей: {}, обложек: {}",
                    summary.films, summary.relations, summary.posters
                ),
                false,
            ),
            Err(error) => {
                self.notify(format!("Не удалось экспортировать: {error}"), true);
            }
        }
    }

    fn import_library(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Импорт фильмографии")
            .add_filter("ZIP-архив", &["zip"])
            .pick_file()
        else {
            return;
        };

        let result = match self.db.as_mut() {
            Some(db) => transfer::import_library(db, &self.paths, &path),
            None => Err(AppError::DatabaseUnavailable),
        };

        match result {
            Ok(summary) => {
                self.reload_films();
                self.reload_categories();
                self.notify(summary.message(), !summary.warnings.is_empty());
            }
            Err(error) => {
                self.notify(format!("Не удалось импортировать: {error}"), true);
            }
        }
    }

    fn take_editor(&mut self) -> Option<Box<EditorState>> {
        match std::mem::replace(&mut self.screen, Screen::Library) {
            Screen::Editor(editor) => Some(editor),
            other => {
                self.screen = other;
                None
            }
        }
    }

    fn save_editor(&mut self) {
        let Some(mut editor) = self.take_editor() else {
            return;
        };

        // 1. Проверяем текстовые поля до любых операций с файлами и БД.
        if let Err(error) = editor.to_draft(None).validate() {
            editor.form_error = Some(error.to_string());
            self.screen = Screen::Editor(editor);
            return;
        }

        // 2. Если выбрана новая обложка — копируем её в папку приложения.
        let mut imported = None;
        let mut poster_for_db = if editor.remove_poster {
            None
        } else {
            editor.original_poster.clone()
        };

        if let Some(source) = editor.pending_poster.clone() {
            let import_result = match &source {
                PendingPoster::File(path) => storage::import_poster(path, &self.paths),
                PendingPoster::Clipboard { image } => {
                    storage::import_poster_image(image, &self.paths)
                }
            };

            match import_result {
                Ok(poster) => {
                    poster_for_db = Some(poster.relative_path.clone());
                    imported = Some(poster);
                }
                Err(error) => {
                    editor.form_error = Some(format!("Не удалось добавить обложку: {error}"));
                    self.screen = Screen::Editor(editor);
                    return;
                }
            }
        }

        // 3. Сохраняем запись.
        let draft = editor.to_draft(poster_for_db);
        let film_id = editor.film_id;

        let save_result = match self.db.as_mut() {
            Some(db) => match film_id {
                Some(id) => db.update_film(id, &draft),
                None => db.insert_film(&draft).map(|_| ()),
            },
            None => Err(AppError::DatabaseUnavailable),
        };

        match save_result {
            Ok(()) => {
                // Новая обложка успешно сохранена — старый файл больше не нужен.
                let mut warning = None;
                if (imported.is_some() || editor.remove_poster)
                    && let Some(old_poster) = editor.original_poster.as_deref()
                    && let Err(error) = storage::delete_poster(&self.paths, old_poster)
                {
                    warning = Some(format!(
                        "Фильм сохранён, но старую обложку удалить не удалось: {error}"
                    ));
                }

                self.return_to_library();
                let reloaded = self.reload_films();
                self.reload_categories();

                if let Some(warning) = warning {
                    self.notify(warning, true);
                } else if reloaded {
                    self.notify(
                        if film_id.is_some() {
                            "Изменения сохранены"
                        } else {
                            "Фильм добавлен"
                        },
                        false,
                    );
                }
            }
            Err(error) => {
                // Откатываем только что скопированный файл, чтобы не оставлять мусор.
                if let Some(poster) = imported {
                    let _ = std::fs::remove_file(&poster.absolute_path);
                }

                editor.form_error = Some(format!("Не удалось сохранить фильм: {error}"));
                self.screen = Screen::Editor(editor);
            }
        }
    }

    fn ask_delete_by_id(&mut self, id: i64) {
        let result = match self.db.as_ref() {
            Some(db) => db.get_film(id),
            None => Err(AppError::DatabaseUnavailable),
        };

        match result {
            Ok(film) => self.confirm_delete = Some(film),
            Err(error) => {
                self.notify(format!("Не удалось открыть фильм: {error}"), true);
            }
        }
    }

    fn move_film(&mut self, film_id: i64, target_id: i64) {
        let result = match self.db.as_mut() {
            Some(db) => db.reorder_film(film_id, target_id),
            None => Err(AppError::DatabaseUnavailable),
        };

        match result {
            Ok(()) => {
                self.reload_films();
            }
            Err(error) => {
                self.notify(format!("Не удалось изменить порядок: {error}"), true);
            }
        }
    }

    fn delete_film(&mut self, id: i64) {
        let result = match self.db.as_mut() {
            Some(db) => db.delete_film(id),
            None => Err(AppError::DatabaseUnavailable),
        };

        self.confirm_delete = None;

        match result {
            Ok(poster_path) => {
                self.return_to_library();

                let mut warning = None;
                if let Some(stored_path) = poster_path
                    && let Err(error) = storage::delete_poster(&self.paths, &stored_path)
                {
                    warning = Some(format!(
                        "Фильм удалён, но обложку удалить не удалось: {error}"
                    ));
                }

                let reloaded = self.reload_films();

                if let Some(warning) = warning {
                    self.notify(warning, true);
                } else if reloaded {
                    self.notify("Фильм удалён", false);
                }
            }
            Err(error) => {
                self.notify(format!("Не удалось удалить фильм: {error}"), true);
            }
        }
    }

    fn notify(&mut self, message: impl Into<String>, is_error: bool) {
        self.toast = Some(Toast {
            message: message.into(),
            is_error,
            until: Instant::now() + Duration::from_secs(if is_error { 8 } else { 4 }),
        });
    }

    fn show_toast(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let now = Instant::now();

        if self.toast.as_ref().is_some_and(|toast| toast.until <= now) {
            self.toast = None;
        }

        let Some(toast) = &self.toast else {
            return;
        };

        ctx.request_repaint_after(toast.until.saturating_duration_since(now));

        let color = if toast.is_error {
            theme::DANGER
        } else {
            theme::SUCCESS
        };
        let mut dismiss = false;

        egui::Panel::bottom("toast_bar")
            .frame(
                egui::Frame::new()
                    .fill(theme::SURFACE)
                    .stroke(egui::Stroke::new(1.0, theme::BORDER))
                    .inner_margin(egui::Margin::symmetric(16, 8)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(color, &toast.message);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("×").clicked() {
                            dismiss = true;
                        }
                    });
                });
            });

        if dismiss {
            self.toast = None;
        }
    }

    fn show_delete_confirm(&mut self, ctx: &egui::Context) {
        let Some(film) = self.confirm_delete.clone() else {
            return;
        };

        let mut confirm = false;
        let mut cancel = false;

        let modal = egui::Modal::new(egui::Id::unique("confirm_delete_modal")).show(ctx, |ui| {
            ui.set_width(380.0);
            ui.heading("Удалить фильм?");
            ui.add_space(4.0);
            ui.label(RichText::new(format!("Удалить «{}»?", film.title)).strong());
            ui.label(
                RichText::new("Запись и файл обложки будут удалены безвозвратно.")
                    .small()
                    .color(theme::TEXT_MUTED),
            );
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::Button::new(RichText::new("Удалить").color(Color32::WHITE))
                            .fill(theme::DANGER),
                    )
                    .clicked()
                {
                    confirm = true;
                }
                if ui.button("Отмена").clicked() {
                    cancel = true;
                }
            });
        });

        if confirm {
            self.delete_film(film.id);
        } else if cancel || modal.should_close() {
            self.confirm_delete = None;
        }
    }

    fn show_settings(&mut self, ctx: &egui::Context) {
        if !self.settings_open {
            return;
        }

        let mut save = false;
        let mut cancel = false;

        let modal = egui::Modal::new(egui::Id::unique("settings_modal")).show(ctx, |ui| {
            ui.set_width(500.0);
            ui.heading("Настройки");
            ui.add_space(4.0);
            ui.label(
                RichText::new(
                    "Приложение полностью локальное. Ключ нужен только для \
                     автозаполнения данных из OMDb.",
                )
                .small()
                .color(theme::TEXT_MUTED),
            );
            ui.add_space(10.0);

            ui.label(RichText::new("Ключ OMDb API").strong());
            ui.add(
                egui::TextEdit::singleline(&mut self.settings_key)
                    .hint_text("Вставьте бесплатный ключ")
                    .desired_width(f32::INFINITY),
            );
            ui.hyperlink_to(
                "Получить бесплатный ключ на omdbapi.com",
                metadata::OMDB_KEY_HELP_URL,
            );
            ui.add_space(6.0);
            ui.label(
                RichText::new(
                    "Ключ хранится в library.db рядом с фильмографией. \
                     Пустое поле отключает автозаполнение.",
                )
                .small()
                .color(theme::TEXT_MUTED),
            );

            ui.add_space(14.0);
            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("Сохранить")
                                .strong()
                                .color(Color32::from_rgb(22, 20, 15)),
                        )
                        .fill(theme::ACCENT),
                    )
                    .clicked()
                {
                    save = true;
                }
                if ui.button("Отмена").clicked() {
                    cancel = true;
                }
            });
        });

        if save {
            self.save_settings();
        } else if cancel || modal.should_close() {
            self.settings_open = false;
        }
    }

    fn show_startup_error(&mut self, ui: &mut egui::Ui) {
        let error = self.startup_error.clone().unwrap_or_default();
        let db_path = self.paths.db_path.display().to_string();
        let mut retry = false;
        let mut quit = false;

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme::BG))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(120.0);
                    ui.heading("Не удалось открыть библиотеку");
                    ui.add_space(8.0);
                    ui.colored_label(theme::DANGER, &error);
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(format!("Файл базы данных: {db_path}"))
                            .small()
                            .color(theme::TEXT_MUTED),
                    );
                    ui.add_space(16.0);

                    ui.horizontal(|ui| {
                        if ui.button("Повторить").clicked() {
                            retry = true;
                        }
                        if ui.button("Выйти").clicked() {
                            quit = true;
                        }
                    });
                });
            });

        if retry {
            self.retry_startup();
        }
        if quit {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

impl eframe::App for FilmApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        if self.startup_error.is_some() {
            self.show_startup_error(ui);
            return;
        }

        self.poll_navigation(&ctx);

        if matches!(self.screen, Screen::Library)
            && ctx.input(|input| input.modifiers.command && input.key_pressed(egui::Key::N))
        {
            self.open_new_editor();
        }

        self.show_toast(ui);

        let mut filter_changed = false;

        let action = match &mut self.screen {
            Screen::Library => {
                let previous_filter = self.filter.clone();
                let library_action = library::show(
                    ui,
                    &self.films,
                    &mut self.filter,
                    &self.categories,
                    &mut self.library_state,
                    &self.paths,
                    &mut self.poster_cache,
                );

                filter_changed = self.filter != previous_filter;

                library_action.map(|action| match action {
                    LibraryAction::AddNew => AppAction::AddNew,
                    LibraryAction::Edit(id) => AppAction::Edit(id),
                    LibraryAction::Move { film_id, target_id } => {
                        AppAction::MoveFilm { film_id, target_id }
                    }
                    LibraryAction::Export => AppAction::Export,
                    LibraryAction::Import => AppAction::Import,
                    LibraryAction::Settings => AppAction::OpenSettings,
                })
            }
            Screen::Editor(editor) => {
                match editor::show(
                    ui,
                    editor,
                    &self.paths,
                    &self.categories,
                    &self.tag_suggestions,
                    self.metadata_job.is_some(),
                    &mut self.poster_cache,
                ) {
                    EditorAction::Save => Some(AppAction::SaveEditor),
                    EditorAction::Cancel => Some(AppAction::CancelEditor),
                    EditorAction::Delete(id) => Some(AppAction::AskDeleteById(id)),
                    EditorAction::Autofill => Some(AppAction::Autofill),
                    EditorAction::AddRelation {
                        source_id,
                        target_id,
                        kind,
                    } => Some(AppAction::AddRelation {
                        source_id,
                        target_id,
                        kind,
                    }),
                    EditorAction::RemoveRelation(id) => Some(AppAction::RemoveRelation(id)),
                    EditorAction::OpenRelated(id) => Some(AppAction::OpenRelated(id)),
                    EditorAction::None => None,
                }
            }
        };

        if filter_changed {
            // Новый фильтр — новый список, поэтому показываем его с начала.
            self.library_state.scroll_salt = self.alloc_scroll_salt();
            self.reload_films();
        }

        if let Some(action) = action {
            match action {
                AppAction::AddNew => self.open_new_editor(),
                AppAction::Edit(id) => self.open_editor(id),
                AppAction::AskDeleteById(id) => self.ask_delete_by_id(id),
                AppAction::MoveFilm { film_id, target_id } => self.move_film(film_id, target_id),
                AppAction::SaveEditor => self.save_editor(),
                AppAction::CancelEditor => self.return_to_library(),
                AppAction::Autofill => self.start_autofill(),
                AppAction::AddRelation {
                    source_id,
                    target_id,
                    kind,
                } => self.add_relation(source_id, target_id, kind),
                AppAction::RemoveRelation(id) => self.remove_relation(id),
                AppAction::OpenRelated(id) => self.open_editor(id),
                AppAction::Export => self.export_library(),
                AppAction::Import => self.import_library(),
                AppAction::OpenSettings => self.open_settings(),
            }
        }

        self.poll_metadata(&ctx);
        self.show_delete_confirm(&ctx);
        self.show_settings(&ctx);
    }
}

fn editor_query(editor: &EditorState) -> String {
    if editor.original_title.trim().is_empty() {
        editor.title.trim().to_owned()
    } else {
        editor.original_title.trim().to_owned()
    }
}

fn relation_candidates(films: &[Film], current_id: i64) -> Vec<(i64, String)> {
    let mut candidates: Vec<(i64, String)> = films
        .iter()
        .filter(|film| film.id != current_id)
        .map(|film| (film.id, film.title.clone()))
        .collect();
    candidates.sort_by_cached_key(|(_, title)| title.to_lowercase());
    candidates
}
