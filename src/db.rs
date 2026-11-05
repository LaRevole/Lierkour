//! Слой доступа к SQLite.
//!
//! Здесь нет UI-логики: только открытие соединения, миграции и CRUD.
//! `rusqlite` с feature `bundled` компилирует SQLite внутрь бинарника,
//! поэтому приложение не зависит от системной библиотеки.

use std::fs;
use std::path::Path;
use std::time::Duration;

use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::error::{AppError, AppResult};
use crate::models::{
    AppTheme, DEFAULT_CATEGORIES, DashboardStats, Film, FilmFilter, FilmRelation, FilmScreenshot,
    FilmStatus, NewFilm, RelatedFilm, RelationKind, parse_quotes, parse_watched_seasons,
    split_comma_list,
};

/// Список миграций: (версия, SQL). Применяются по порядку в транзакции.
///
/// Чтобы добавить изменение схемы, создайте `migrations/003_*.sql`
/// и допишите пару в этот массив. Уже применённые миграции редактировать нельзя.
const MIGRATIONS: &[(i64, &str)] = &[
    (1, include_str!("../migrations/001_init.sql")),
    (2, include_str!("../migrations/002_categories.sql")),
    (3, include_str!("../migrations/003_series_and_order.sql")),
    (
        4,
        include_str!("../migrations/004_metadata_relations_export.sql"),
    ),
    (5, include_str!("../migrations/005_tags.sql")),
    (
        6,
        include_str!("../migrations/006_screenshots_and_quotes.sql"),
    ),
];

pub struct Database {
    conn: Connection,
}

impl Database {
    /// Открывает (или создаёт) файл БД и применяет миграции.
    pub fn open(path: impl AsRef<Path>) -> AppResult<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)?;
        Self::from_connection(conn)
    }

    fn from_connection(mut conn: Connection) -> AppResult<Self> {
        conn.busy_timeout(Duration::from_secs(5))?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        // WAL заметно снижает вероятность блокировок при чтении во время записи.
        // Для in-memory БД SQLite просто проигнорирует этот режим.
        let _ = conn.pragma_update(None, "journal_mode", "WAL");
        conn.pragma_update(None, "synchronous", "NORMAL")?;

        Self::migrate(&mut conn)?;

        Ok(Self { conn })
    }

    fn migrate(conn: &mut Connection) -> AppResult<()> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                 version    INTEGER PRIMARY KEY,
                 applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
             );",
        )?;

        let current_version: i64 = conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )?;

        for (version, sql) in MIGRATIONS {
            if *version <= current_version {
                continue;
            }

            let tx = conn.transaction()?;
            tx.execute_batch(sql)?;
            tx.execute(
                "INSERT INTO schema_migrations (version) VALUES (?1)",
                [*version],
            )?;
            tx.commit()?;
        }

        Ok(())
    }

    /// Возвращает фильмы с учётом фильтра по статусу/категории и поиска по названию.
    ///
    /// Поиск выполняется в Rust через Unicode-`to_lowercase`, потому что
    /// встроенный SQLite `LIKE ... COLLATE NOCASE` корректно работает
    /// только с ASCII. Для локальной библиотеки это не создаёт проблем.
    pub fn list_films(&self, filter: &FilmFilter) -> AppResult<Vec<Film>> {
        let mut statement = self.conn.prepare(
            "SELECT id, title, original_title, category, director, year, genres, country,
                    runtime_minutes, imdb_id, tags, status, rating, description, thoughts,
                    poster_path, seasons_total, watched_seasons, quotes
             FROM films
             WHERE (?1 IS NULL OR status = ?1)
               AND (?2 IS NULL OR category = ?2)
             ORDER BY sort_order ASC, updated_at DESC, id DESC",
        )?;

        let rows = statement.query_map(
            params![
                filter.status.map(FilmStatus::as_db_str),
                filter.category.as_deref(),
            ],
            film_from_row,
        )?;

        let mut films = rows.collect::<rusqlite::Result<Vec<_>>>()?;

        let needle = filter.search.trim().to_lowercase();
        if !needle.is_empty() {
            films.retain(|film| {
                film.title.to_lowercase().contains(&needle)
                    || film.original_title.to_lowercase().contains(&needle)
                    || film.director.to_lowercase().contains(&needle)
                    || film.genres.to_lowercase().contains(&needle)
                    || film.country.to_lowercase().contains(&needle)
                    || film.imdb_id.to_lowercase().contains(&needle)
                    || film.tags.to_lowercase().contains(&needle)
                    || film
                        .year
                        .is_some_and(|year| year.to_string().contains(&needle))
            });
        }

        Ok(films)
    }

    /// Все фильмы в сохранённом порядке (без фильтров и поиска).
    pub fn list_all_films(&self) -> AppResult<Vec<Film>> {
        self.list_films(&FilmFilter::default())
    }

    pub fn get_film(&self, id: i64) -> AppResult<Film> {
        self.conn
            .query_row(
                "SELECT id, title, original_title, category, director, year, genres, country,
                        runtime_minutes, imdb_id, tags, status, rating, description, thoughts,
                        poster_path, seasons_total, watched_seasons, quotes
                 FROM films WHERE id = ?1",
                [id],
                film_from_row,
            )
            .optional()?
            .ok_or(AppError::FilmNotFound(id))
    }

    /// Список категорий: предопределённые плюс все категории из сохранённых
    /// фильмов (включая добавленные пользователем).
    pub fn list_categories(&self) -> AppResult<Vec<String>> {
        let mut categories: Vec<String> = DEFAULT_CATEGORIES
            .iter()
            .map(|category| (*category).to_owned())
            .collect();

        let mut statement = self
            .conn
            .prepare("SELECT DISTINCT category FROM films ORDER BY category COLLATE NOCASE")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;

        for category in rows {
            let category = category?;
            let normalized = category.to_lowercase();
            if !categories
                .iter()
                .any(|known| known.to_lowercase() == normalized)
            {
                categories.push(category);
            }
        }

        Ok(categories)
    }

    /// Вставляет фильм и возвращает его новый `id`.
    pub fn insert_film(&mut self, draft: &NewFilm) -> AppResult<i64> {
        draft.validate()?;

        self.conn.execute(
            "INSERT INTO films (
                 title, original_title, category, director, year, genres, country,
                 runtime_minutes, imdb_id, tags, status, rating, description, thoughts,
                 poster_path, seasons_total, watched_seasons, quotes, sort_order
             )
             VALUES (
                 ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18,
                 (SELECT COALESCE(MIN(sort_order), 0) - 1 FROM films)
             )",
            params![
                draft.title.trim(),
                draft.original_title.trim(),
                draft.category.trim(),
                draft.director.trim(),
                draft.year,
                draft.genres_db(),
                draft.country.trim(),
                draft.runtime_minutes,
                draft.imdb_id_db(),
                draft.tags_db(),
                draft.status,
                draft.rating,
                draft.description.trim(),
                draft.thoughts.trim(),
                draft.poster_path,
                draft.seasons_total,
                draft.watched_seasons_db(),
                draft.quotes_db(),
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    pub fn update_film(&mut self, id: i64, draft: &NewFilm) -> AppResult<()> {
        draft.validate()?;

        let changed = self.conn.execute(
            "UPDATE films
             SET title = ?2,
                 original_title = ?3,
                 category = ?4,
                 director = ?5,
                 year = ?6,
                 genres = ?7,
                 country = ?8,
                 runtime_minutes = ?9,
                 imdb_id = ?10,
                 tags = ?11,
                 status = ?12,
                 rating = ?13,
                 description = ?14,
                 thoughts = ?15,
                 poster_path = ?16,
                 seasons_total = ?17,
                 watched_seasons = ?18,
                 quotes = ?19,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?1",
            params![
                id,
                draft.title.trim(),
                draft.original_title.trim(),
                draft.category.trim(),
                draft.director.trim(),
                draft.year,
                draft.genres_db(),
                draft.country.trim(),
                draft.runtime_minutes,
                draft.imdb_id_db(),
                draft.tags_db(),
                draft.status,
                draft.rating,
                draft.description.trim(),
                draft.thoughts.trim(),
                draft.poster_path,
                draft.seasons_total,
                draft.watched_seasons_db(),
                draft.quotes_db(),
            ],
        )?;

        if changed == 0 {
            return Err(AppError::FilmNotFound(id));
        }

        Ok(())
    }

    /// Перемещает карточку `film_id` на место `target_id` и сохраняет порядок.
    ///
    /// Позиции перенумеровываются целиком, поэтому в БД не накапливаются
    /// одинаковые `sort_order` после серии перетаскиваний.
    pub fn reorder_film(&mut self, film_id: i64, target_id: i64) -> AppResult<()> {
        if film_id == target_id {
            return Ok(());
        }

        let mut ids = {
            let mut statement = self.conn.prepare(
                "SELECT id FROM films ORDER BY sort_order ASC, updated_at DESC, id DESC",
            )?;
            let rows = statement.query_map([], |row| row.get::<_, i64>(0))?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
        };

        let from_index = ids
            .iter()
            .position(|id| *id == film_id)
            .ok_or(AppError::FilmNotFound(film_id))?;
        // Индекс цели запоминаем до удаления: так перетаскиваемая карточка
        // занимает место цели, а остальные сдвигаются в нужную сторону.
        let target_index = ids
            .iter()
            .position(|id| *id == target_id)
            .ok_or(AppError::FilmNotFound(target_id))?;
        ids.remove(from_index);
        ids.insert(target_index.min(ids.len()), film_id);

        self.set_film_order(&ids)
    }

    /// Присваивает фильмам позиции в указанном порядке.
    ///
    /// Используется перетаскиванием карточек и импортом фильмографии.
    pub fn set_film_order(&mut self, ids: &[i64]) -> AppResult<()> {
        let tx = self.conn.transaction()?;
        for (position, id) in ids.iter().enumerate() {
            tx.execute(
                "UPDATE films SET sort_order = ?2 WHERE id = ?1",
                params![id, position as i64],
            )?;
        }
        tx.commit()?;

        Ok(())
    }

    fn film_exists(&self, id: i64) -> AppResult<bool> {
        Ok(self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM films WHERE id = ?1)",
            [id],
            |row| row.get(0),
        )?)
    }

    /// Добавляет направленную связь. Возвращает `false`, если она уже была.
    pub fn add_relation(
        &mut self,
        source_id: i64,
        target_id: i64,
        kind: RelationKind,
    ) -> AppResult<bool> {
        if source_id == target_id {
            return Err(AppError::SelfRelation);
        }
        if !self.film_exists(source_id)? {
            return Err(AppError::FilmNotFound(source_id));
        }
        if !self.film_exists(target_id)? {
            return Err(AppError::FilmNotFound(target_id));
        }

        let changed = self.conn.execute(
            "INSERT OR IGNORE INTO film_relations (source_id, target_id, kind)
             VALUES (?1, ?2, ?3)",
            params![source_id, target_id, kind],
        )?;

        Ok(changed > 0)
    }

    pub fn remove_relation(&mut self, relation_id: i64) -> AppResult<()> {
        let changed = self
            .conn
            .execute("DELETE FROM film_relations WHERE id = ?1", [relation_id])?;

        if changed == 0 {
            return Err(AppError::RelationNotFound(relation_id));
        }

        Ok(())
    }

    /// Все связи фильма с подписями со стороны этого фильма.
    pub fn relations_for(&self, film_id: i64) -> AppResult<Vec<RelatedFilm>> {
        let mut statement = self.conn.prepare(
            "SELECT r.id AS relation_id,
                    (r.source_id = ?1) AS outgoing,
                    r.kind AS relation_kind,
                    f.id, f.title, f.original_title, f.category, f.director, f.year,
                    f.genres, f.country, f.runtime_minutes, f.imdb_id, f.tags, f.status, f.rating,
                    f.description, f.thoughts, f.poster_path, f.seasons_total,
                    f.watched_seasons, f.quotes
             FROM film_relations r
             JOIN films f
               ON f.id = CASE WHEN r.source_id = ?1 THEN r.target_id ELSE r.source_id END
             WHERE r.source_id = ?1 OR r.target_id = ?1
             ORDER BY r.kind, f.title COLLATE NOCASE",
        )?;

        let rows = statement.query_map([film_id], |row| {
            let outgoing: i64 = row.get("outgoing")?;
            Ok(RelatedFilm {
                relation_id: row.get("relation_id")?,
                film: film_from_row(row)?,
                kind: row.get("relation_kind")?,
                outgoing: outgoing != 0,
            })
        })?;

        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Все связи библиотеки — для экспорта и импорта.
    pub fn list_all_relations(&self) -> AppResult<Vec<FilmRelation>> {
        let mut statement = self
            .conn
            .prepare("SELECT id, source_id, target_id, kind FROM film_relations ORDER BY id")?;
        let rows = statement.query_map([], |row| {
            Ok(FilmRelation {
                id: row.get("id")?,
                source_id: row.get("source_id")?,
                target_id: row.get("target_id")?,
                kind: row.get("kind")?,
            })
        })?;

        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn get_setting(&self, key: &str) -> AppResult<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
                row.get(0)
            })
            .optional()?)
    }

    pub fn set_setting(&mut self, key: &str, value: &str) -> AppResult<()> {
        self.conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    /// Удаляет фильм и возвращает путь к его обложке (если она была),
    /// чтобы вызывающий код мог удалить файл.
    pub fn delete_film(&mut self, id: i64) -> AppResult<Option<String>> {
        let poster_path: Option<String> = self
            .conn
            .query_row("SELECT poster_path FROM films WHERE id = ?1", [id], |row| {
                row.get(0)
            })
            .optional()?
            .ok_or(AppError::FilmNotFound(id))?;

        self.conn.execute("DELETE FROM films WHERE id = ?1", [id])?;

        Ok(poster_path)
    }

    // --- Скриншоты и кадры ---

    pub fn list_screenshots(&self, film_id: i64) -> AppResult<Vec<FilmScreenshot>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, film_id, image_path, caption, position
             FROM film_screenshots
             WHERE film_id = ?1
             ORDER BY position ASC, id ASC",
        )?;
        let rows = stmt.query_map([film_id], |row| {
            Ok(FilmScreenshot {
                id: row.get(0)?,
                film_id: row.get(1)?,
                image_path: row.get(2)?,
                caption: row.get(3)?,
                position: row.get(4)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn list_all_screenshots(&self) -> AppResult<Vec<FilmScreenshot>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, film_id, image_path, caption, position
             FROM film_screenshots
             ORDER BY film_id ASC, position ASC, id ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(FilmScreenshot {
                id: row.get(0)?,
                film_id: row.get(1)?,
                image_path: row.get(2)?,
                caption: row.get(3)?,
                position: row.get(4)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn add_screenshot(
        &mut self,
        film_id: i64,
        image_path: &str,
        caption: &str,
    ) -> AppResult<i64> {
        let next_pos: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(position), -1) + 1 FROM film_screenshots WHERE film_id = ?1",
            [film_id],
            |r| r.get(0),
        )?;

        self.conn.execute(
            "INSERT INTO film_screenshots (film_id, image_path, caption, position)
             VALUES (?1, ?2, ?3, ?4)",
            params![film_id, image_path.trim(), caption.trim(), next_pos],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    pub fn delete_screenshot(&mut self, id: i64) -> AppResult<Option<String>> {
        let path: Option<String> = self
            .conn
            .query_row(
                "SELECT image_path FROM film_screenshots WHERE id = ?1",
                [id],
                |r| r.get(0),
            )
            .optional()?;

        if path.is_some() {
            self.conn
                .execute("DELETE FROM film_screenshots WHERE id = ?1", [id])?;
        }

        Ok(path)
    }

    // --- Настройки и Тема ---

    pub fn get_theme(&self) -> AppResult<AppTheme> {
        match self.get_setting("theme")? {
            Some(value) => Ok(AppTheme::from_str_opt(&value).unwrap_or_default()),
            None => Ok(AppTheme::default()),
        }
    }

    pub fn set_theme(&mut self, theme: AppTheme) -> AppResult<()> {
        self.set_setting("theme", theme.as_str())
    }

    // --- Статистика библиотеки (Dashboard) ---

    pub fn get_dashboard_stats(&self) -> AppResult<DashboardStats> {
        let total_films: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM films", [], |r| r.get(0))?;

        let watched_films: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM films WHERE status = 'watched'",
            [],
            |r| r.get(0),
        )?;

        let want_to_watch: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM films WHERE status = 'want_to_watch'",
            [],
            |r| r.get(0),
        )?;

        let watching: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM films WHERE status = 'watching'",
            [],
            |r| r.get(0),
        )?;

        let dropped: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM films WHERE status = 'dropped'",
            [],
            |r| r.get(0),
        )?;

        let average_rating: Option<f64> = self
            .conn
            .query_row(
                "SELECT AVG(rating) FROM films WHERE rating IS NOT NULL",
                [],
                |r| r.get(0),
            )
            .optional()?
            .flatten();

        let total_runtime_minutes: i64 = self
            .conn
            .query_row(
                "SELECT COALESCE(SUM(runtime_minutes), 0) FROM films WHERE status = 'watched' AND runtime_minutes IS NOT NULL",
                [],
                |r| r.get(0),
            )?;

        // Топ жанров
        let mut genre_counts = std::collections::HashMap::<String, usize>::new();
        let mut stmt = self
            .conn
            .prepare("SELECT genres FROM films WHERE genres != ''")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        for g_str in rows.flatten() {
            for genre in split_comma_list(&g_str) {
                *genre_counts.entry(genre.to_owned()).or_insert(0) += 1;
            }
        }
        let mut top_genres: Vec<(String, usize)> = genre_counts.into_iter().collect();
        top_genres.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        top_genres.truncate(5);

        // Топ режиссёров
        let mut director_counts = std::collections::HashMap::<String, usize>::new();
        let mut stmt = self
            .conn
            .prepare("SELECT director FROM films WHERE director != ''")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        for d in rows.flatten() {
            let trimmed = d.trim();
            if !trimmed.is_empty() {
                *director_counts.entry(trimmed.to_owned()).or_insert(0) += 1;
            }
        }
        let mut top_directors: Vec<(String, usize)> = director_counts.into_iter().collect();
        top_directors.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        top_directors.truncate(5);

        Ok(DashboardStats {
            total_films: total_films.max(0) as usize,
            watched_films: watched_films.max(0) as usize,
            want_to_watch: want_to_watch.max(0) as usize,
            watching: watching.max(0) as usize,
            dropped: dropped.max(0) as usize,
            average_rating,
            total_runtime_minutes: total_runtime_minutes.max(0) as u64,
            top_genres,
            top_directors,
        })
    }
}

fn film_from_row(row: &Row<'_>) -> rusqlite::Result<Film> {
    let seasons_total: u16 = row.get("seasons_total")?;
    let watched_seasons: String = row.get("watched_seasons")?;
    let quotes_str: String = row.get::<_, Option<String>>("quotes")?.unwrap_or_default();

    Ok(Film {
        id: row.get("id")?,
        title: row.get("title")?,
        original_title: row.get("original_title")?,
        category: row.get("category")?,
        director: row.get("director")?,
        year: row.get("year")?,
        genres: row.get("genres")?,
        country: row.get("country")?,
        runtime_minutes: row.get("runtime_minutes")?,
        imdb_id: row.get("imdb_id")?,
        tags: row.get("tags")?,
        status: row.get("status")?,
        rating: row.get("rating")?,
        description: row.get("description")?,
        thoughts: row.get("thoughts")?,
        poster_path: row.get("poster_path")?,
        seasons_total,
        watched_seasons: parse_watched_seasons(&watched_seasons, seasons_total),
        quotes: parse_quotes(&quotes_str),
    })
}

// Позволяет читать/писать FilmStatus напрямую через rusqlite.
impl FromSql for FilmStatus {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let value = value.as_str()?;
        FilmStatus::from_db_str(value).ok_or(FromSqlError::InvalidType)
    }
}

impl ToSql for FilmStatus {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.as_db_str()))
    }
}

impl FromSql for RelationKind {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let value = value.as_str()?;
        RelationKind::from_db_str(value).ok_or(FromSqlError::InvalidType)
    }
}

impl ToSql for RelationKind {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.as_db_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::FilmValidationError;

    fn test_db() -> Database {
        Database::from_connection(Connection::open_in_memory().unwrap()).unwrap()
    }

    fn draft(title: &str, status: FilmStatus) -> NewFilm {
        NewFilm {
            title: title.to_owned(),
            status,
            rating: Some(8),
            description: "Описание".to_owned(),
            thoughts: "Мысли".to_owned(),
            ..NewFilm::default()
        }
    }

    #[test]
    fn insert_list_update_delete() {
        let mut db = test_db();

        let id = db
            .insert_film(&draft("Интерстеллар", FilmStatus::WantToWatch))
            .unwrap();

        let films = db.list_films(&FilmFilter::default()).unwrap();
        assert_eq!(films.len(), 1);
        assert_eq!(films[0].id, id);
        assert_eq!(films[0].title, "Интерстеллар");
        assert_eq!(films[0].category, crate::models::DEFAULT_CATEGORY);
        assert_eq!(films[0].rating, Some(8));

        let mut updated = draft("Интерстеллар", FilmStatus::Watched);
        updated.original_title = "Interstellar".to_owned();
        updated.category = "Сериалы".to_owned();
        updated.director = "Кристофер Нолан".to_owned();
        updated.rating = Some(10);
        updated.poster_path = Some("posters/interstellar.jpg".to_owned());
        updated.seasons_total = 3;
        updated.watched_seasons = vec![3, 1, 1];
        db.update_film(id, &updated).unwrap();

        let film = db.get_film(id).unwrap();
        assert_eq!(film.original_title, "Interstellar");
        assert_eq!(film.category, "Сериалы");
        assert_eq!(film.director, "Кристофер Нолан");
        assert_eq!(film.status, FilmStatus::Watched);
        assert_eq!(film.rating, Some(10));
        assert_eq!(film.seasons_total, 3);
        assert_eq!(film.watched_seasons, vec![1, 3]);
        assert!(film.watched_season_count() < film.seasons_total as usize);
        assert_eq!(
            film.poster_path.as_deref(),
            Some("posters/interstellar.jpg")
        );

        let poster = db.delete_film(id).unwrap();
        assert_eq!(poster.as_deref(), Some("posters/interstellar.jpg"));
        assert!(matches!(db.get_film(id), Err(AppError::FilmNotFound(_))));
    }

    #[test]
    fn filters_by_status_and_unicode_title() {
        let mut db = test_db();
        db.insert_film(&draft("Дюна", FilmStatus::Watched)).unwrap();
        db.insert_film(&draft("Дюна: Часть вторая", FilmStatus::Watching))
            .unwrap();
        db.insert_film(&draft("Arrival", FilmStatus::Watched))
            .unwrap();

        let mut original = draft("Прибытие", FilmStatus::WantToWatch);
        original.original_title = "Arrival".to_owned();
        original.director = "Дени Вильнёв".to_owned();
        original.year = Some(2016);
        original.genres = "Фантастика, Драма".to_owned();
        original.country = "Канада".to_owned();
        original.tags = "умное кино, на пересмотр".to_owned();
        db.insert_film(&original).unwrap();

        let by_director = db
            .list_films(&FilmFilter {
                search: "вильнёв".to_owned(),
                status: None,
                category: None,
            })
            .unwrap();
        assert_eq!(by_director.len(), 1);
        assert_eq!(by_director[0].title, "Прибытие");

        let by_original = db
            .list_films(&FilmFilter {
                search: "arrival".to_owned(),
                status: None,
                category: None,
            })
            .unwrap();
        assert_eq!(by_original.len(), 2);

        for query in ["канада", "2016", "фантастика", "пересмотр"] {
            let found = db
                .list_films(&FilmFilter {
                    search: query.to_owned(),
                    status: None,
                    category: None,
                })
                .unwrap();
            assert_eq!(found.len(), 1, "поиск по «{query}»");
            assert_eq!(found[0].title, "Прибытие");
        }

        let by_status = db
            .list_films(&FilmFilter {
                search: String::new(),
                status: Some(FilmStatus::Watched),
                category: None,
            })
            .unwrap();
        assert_eq!(by_status.len(), 2);

        // Поиск должен быть регистронезависимым и для кириллицы.
        let by_search = db
            .list_films(&FilmFilter {
                search: "дюна".to_owned(),
                status: None,
                category: None,
            })
            .unwrap();
        assert_eq!(by_search.len(), 2);
        assert!(by_search.iter().all(|film| film.title.starts_with("Дюна")));
    }

    #[test]
    fn filters_by_category_and_lists_custom_categories() {
        let mut db = test_db();

        let mut anime = draft("Ван-Пис", FilmStatus::Watching);
        anime.category = "Аниме".to_owned();
        db.insert_film(&anime).unwrap();

        let mut custom = draft("Своё видео", FilmStatus::Watched);
        custom.category = "Домашнее видео".to_owned();
        db.insert_film(&custom).unwrap();

        let by_category = db
            .list_films(&FilmFilter {
                search: String::new(),
                status: None,
                category: Some("Аниме".to_owned()),
            })
            .unwrap();
        assert_eq!(by_category.len(), 1);
        assert_eq!(by_category[0].title, "Ван-Пис");

        let categories = db.list_categories().unwrap();
        for (index, expected) in DEFAULT_CATEGORIES.iter().enumerate() {
            assert_eq!(categories[index].as_str(), *expected);
        }
        assert!(
            categories
                .iter()
                .any(|category| category == "Домашнее видео")
        );
    }

    #[test]
    fn migration_adds_category_to_existing_films() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("../migrations/001_init.sql"))
            .unwrap();
        conn.execute_batch(
            "CREATE TABLE schema_migrations (
                 version    INTEGER PRIMARY KEY,
                 applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
             );
             INSERT INTO schema_migrations (version) VALUES (1);",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO films (title, status, description, thoughts)
             VALUES ('Старый фильм', 'watched', '', '')",
            [],
        )
        .unwrap();

        let db = Database::from_connection(conn).unwrap();
        let films = db.list_films(&FilmFilter::default()).unwrap();

        assert_eq!(films.len(), 1);
        assert_eq!(films[0].title, "Старый фильм");
        assert_eq!(films[0].category, crate::models::DEFAULT_CATEGORY);
        assert_eq!(films[0].original_title, "");
        assert_eq!(films[0].director, "");
        assert_eq!(films[0].seasons_total, 0);
        assert!(films[0].watched_seasons.is_empty());
        assert_eq!(films[0].year, None);
        assert_eq!(films[0].genres, "");
        assert_eq!(films[0].country, "");
        assert_eq!(films[0].runtime_minutes, None);
        assert_eq!(films[0].imdb_id, "");
        assert_eq!(films[0].tags, "");
    }

    #[test]
    fn reorders_films_and_keeps_new_on_top() {
        let mut db = test_db();

        let first = db
            .insert_film(&draft("Первый", FilmStatus::Watched))
            .unwrap();
        let second = db
            .insert_film(&draft("Второй", FilmStatus::Watched))
            .unwrap();
        let third = db
            .insert_film(&draft("Третий", FilmStatus::Watched))
            .unwrap();

        // Каждый новый фильм добавляется в начало.
        let ids = |db: &Database| {
            db.list_films(&FilmFilter::default())
                .unwrap()
                .into_iter()
                .map(|film| film.id)
                .collect::<Vec<_>>()
        };
        assert_eq!(ids(&db), vec![third, second, first]);

        // Перетаскиваем первый фильм на место второго.
        db.reorder_film(first, second).unwrap();
        assert_eq!(ids(&db), vec![third, first, second]);

        // Перетаскиваем третий в конец.
        db.reorder_film(third, second).unwrap();
        assert_eq!(ids(&db), vec![first, second, third]);

        assert!(matches!(
            db.reorder_film(999, first),
            Err(AppError::FilmNotFound(999))
        ));
    }

    #[test]
    fn stores_extended_metadata_and_relations() {
        let mut db = test_db();

        let mut first = draft("Дюна", FilmStatus::Watched);
        first.year = Some(2021);
        first.genres = "Фантастика,  Приключения, фантастика".to_owned();
        first.country = "США".to_owned();
        first.runtime_minutes = Some(155);
        first.imdb_id = "TT1160419".to_owned();
        first.tags = "эпик,  На пересмотр, эпик".to_owned();
        let first_id = db.insert_film(&first).unwrap();

        let mut second = draft("Дюна: Часть вторая", FilmStatus::Watching);
        second.year = Some(2024);
        second.imdb_id = "tt15239678".to_owned();
        let second_id = db.insert_film(&second).unwrap();

        let stored = db.get_film(first_id).unwrap();
        assert_eq!(stored.year, Some(2021));
        assert_eq!(stored.genres, "Фантастика, Приключения");
        assert_eq!(stored.country, "США");
        assert_eq!(stored.runtime_minutes, Some(155));
        assert_eq!(stored.imdb_id, "tt1160419");
        assert_eq!(stored.tags, "эпик, На пересмотр");
        assert_eq!(stored.tags_list(), vec!["эпик", "На пересмотр"]);

        assert!(
            db.add_relation(second_id, first_id, RelationKind::Sequel)
                .unwrap()
        );

        let from_first = db.relations_for(first_id).unwrap();
        assert_eq!(from_first.len(), 1);
        assert_eq!(from_first[0].film.id, second_id);
        assert!(!from_first[0].outgoing);
        assert_eq!(from_first[0].label(), "Приквел");

        let from_second = db.relations_for(second_id).unwrap();
        assert_eq!(from_second[0].label(), "Сиквел");
        assert_eq!(
            db.list_all_relations().unwrap(),
            vec![FilmRelation {
                id: 1,
                source_id: second_id,
                target_id: first_id,
                kind: RelationKind::Sequel,
            }]
        );

        // Повторная связь и связь с самим собой отклоняются/игнорируются.
        assert!(
            !db.add_relation(second_id, first_id, RelationKind::Sequel)
                .unwrap()
        );
        assert!(matches!(
            db.add_relation(first_id, first_id, RelationKind::Related),
            Err(AppError::SelfRelation)
        ));
        assert!(matches!(
            db.add_relation(999, first_id, RelationKind::Related),
            Err(AppError::FilmNotFound(999))
        ));

        // Удаление фильма каскадно удаляет его связи.
        db.delete_film(first_id).unwrap();
        assert!(db.relations_for(second_id).unwrap().is_empty());
    }

    #[test]
    fn settings_are_stored_and_updated() {
        let mut db = test_db();
        assert_eq!(db.get_setting("omdb_api_key").unwrap(), None);

        db.set_setting("omdb_api_key", "abc").unwrap();
        assert_eq!(
            db.get_setting("omdb_api_key").unwrap().as_deref(),
            Some("abc")
        );

        db.set_setting("omdb_api_key", "").unwrap();
        assert_eq!(db.get_setting("omdb_api_key").unwrap().as_deref(), Some(""));
    }

    #[test]
    fn validation_errors_do_not_reach_sqlite() {
        let mut db = test_db();
        let mut invalid = draft("   ", FilmStatus::WantToWatch);

        assert!(matches!(
            db.insert_film(&invalid),
            Err(AppError::Validation(FilmValidationError::EmptyTitle))
        ));

        invalid.title = "OK".to_owned();
        invalid.rating = Some(42);
        assert!(matches!(
            db.insert_film(&invalid),
            Err(AppError::Validation(FilmValidationError::RatingOutOfRange))
        ));

        assert!(db.list_films(&FilmFilter::default()).unwrap().is_empty());
    }

    #[test]
    fn migrations_create_schema_and_are_idempotent() {
        let path = std::env::temp_dir().join(format!(
            "mfl-migration-test-{}-{}.db",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        {
            let db = Database::open(&path).unwrap();
            assert_eq!(
                db.conn
                    .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                        row.get::<_, i64>(0)
                    })
                    .unwrap(),
                MIGRATIONS.len() as i64
            );
        }

        {
            // Повторное открытие не должно пытаться применить миграции заново.
            let db = Database::open(&path).unwrap();
            assert_eq!(
                db.conn
                    .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                        row.get::<_, i64>(0)
                    })
                    .unwrap(),
                MIGRATIONS.len() as i64
            );
        }

        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("db-wal"));
        let _ = fs::remove_file(path.with_extension("db-shm"));
    }

    #[test]
    fn quotes_and_screenshots_crud() {
        let mut db = test_db();
        let mut new_film = draft("Матрица", FilmStatus::Watched);
        new_film.quotes = vec![
            "Ложки нет.".to_owned(),
            "Следуй за белым кроликом.".to_owned(),
        ];
        let film_id = db.insert_film(&new_film).unwrap();

        let film = db.get_film(film_id).unwrap();
        assert_eq!(film.quotes.len(), 2);
        assert_eq!(film.quotes[0], "Ложки нет.");
        assert_eq!(film.quotes[1], "Следуй за белым кроликом.");

        // Скриншоты
        let shot1 = db
            .add_screenshot(film_id, "screenshots/shot1.png", "Нео уклоняется от пуль")
            .unwrap();
        let shot2 = db
            .add_screenshot(film_id, "screenshots/shot2.png", "Красная таблетка")
            .unwrap();

        let shots = db.list_screenshots(film_id).unwrap();
        assert_eq!(shots.len(), 2);
        assert_eq!(shots[0].id, shot1);
        assert_eq!(shots[0].caption, "Нео уклоняется от пуль");
        assert_eq!(shots[1].id, shot2);

        // Удаление скриншота
        let deleted_path = db.delete_screenshot(shot1).unwrap();
        assert_eq!(deleted_path, Some("screenshots/shot1.png".to_owned()));
        assert_eq!(db.list_screenshots(film_id).unwrap().len(), 1);
    }

    #[test]
    fn dashboard_stats_and_theme() {
        let mut db = test_db();

        assert_eq!(db.get_theme().unwrap(), AppTheme::Dark);
        db.set_theme(AppTheme::Light).unwrap();
        assert_eq!(db.get_theme().unwrap(), AppTheme::Light);

        let mut f1 = draft("Фильм 1", FilmStatus::Watched);
        f1.director = "Кристофер Нолан".to_owned();
        f1.genres = "Фантастика, Боевик".to_owned();
        f1.rating = Some(9);
        f1.runtime_minutes = Some(150);
        db.insert_film(&f1).unwrap();

        let mut f2 = draft("Фильм 2", FilmStatus::Watched);
        f2.director = "Кристофер Нолан".to_owned();
        f2.genres = "Фантастика, Драма".to_owned();
        f2.rating = Some(7);
        f2.runtime_minutes = Some(120);
        db.insert_film(&f2).unwrap();

        let mut f3 = draft("Фильм 3", FilmStatus::WantToWatch);
        f3.director = "Дени Вильнёв".to_owned();
        f3.genres = "Фантастика".to_owned();
        db.insert_film(&f3).unwrap();

        let stats = db.get_dashboard_stats().unwrap();
        assert_eq!(stats.total_films, 3);
        assert_eq!(stats.watched_films, 2);
        assert_eq!(stats.want_to_watch, 1);
        assert_eq!(stats.total_runtime_minutes, 270);
        assert_eq!(stats.average_rating, Some(8.0));
        assert_eq!(stats.top_genres[0].0, "Фантастика");
        assert_eq!(stats.top_genres[0].1, 3);
        assert_eq!(stats.top_directors[0].0, "Кристофер Нолан");
        assert_eq!(stats.top_directors[0].1, 2);
    }
}
