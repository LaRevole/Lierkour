//! Доменные модели приложения.
//!
//! Модуль намеренно не зависит от egui и rusqlite: это чистые типы,
//! которые удобно тестировать и переиспользовать.

use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Категории, которые предлагаются пользователю по умолчанию.
/// Свои категории добавляются в редакторе и сохраняются вместе с фильмом.
pub const DEFAULT_CATEGORIES: [&str; 6] = [
    "Фильмы",
    "Сериалы",
    "Мультфильмы",
    "Аниме",
    "Документальные",
    "Другое",
];

pub const DEFAULT_CATEGORY: &str = DEFAULT_CATEGORIES[0];

/// Разумный верхний предел для количества сезонов.
pub const MAX_SEASONS: u16 = 200;
/// Границы года выпуска, которые принимает валидация.
pub const MIN_YEAR: u16 = 1888;
pub const MAX_YEAR: u16 = 2100;
/// Верхний предел длительности фильма в минутах.
pub const MAX_RUNTIME_MINUTES: u16 = 1440;

/// Статус просмотра фильма.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilmStatus {
    #[default]
    WantToWatch,
    Watching,
    Watched,
    Dropped,
}

impl FilmStatus {
    /// Все статусы в порядке отображения в интерфейсе.
    pub const ALL: [Self; 4] = [
        Self::WantToWatch,
        Self::Watching,
        Self::Watched,
        Self::Dropped,
    ];

    /// Значение, которое хранится в SQLite.
    pub const fn as_db_str(self) -> &'static str {
        match self {
            Self::WantToWatch => "want_to_watch",
            Self::Watching => "watching",
            Self::Watched => "watched",
            Self::Dropped => "dropped",
        }
    }

    /// Подпись для интерфейса.
    pub const fn label(self) -> &'static str {
        match self {
            Self::WantToWatch => "Хочу посмотреть",
            Self::Watching => "Смотрю",
            Self::Watched => "Просмотрено",
            Self::Dropped => "Брошено",
        }
    }

    pub fn from_db_str(value: &str) -> Option<Self> {
        match value {
            "want_to_watch" => Some(Self::WantToWatch),
            "watching" => Some(Self::Watching),
            "watched" => Some(Self::Watched),
            "dropped" => Some(Self::Dropped),
            _ => None,
        }
    }
}

impl fmt::Display for FilmStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Тип связи между двумя фильмами.
///
/// Связь направленная: `source` — это `kind` по отношению к `target`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationKind {
    Sequel,
    Prequel,
    Remake,
    SpinOff,
    Related,
}

impl RelationKind {
    pub const ALL: [Self; 5] = [
        Self::Sequel,
        Self::Prequel,
        Self::Remake,
        Self::SpinOff,
        Self::Related,
    ];

    pub const fn as_db_str(self) -> &'static str {
        match self {
            Self::Sequel => "sequel",
            Self::Prequel => "prequel",
            Self::Remake => "remake",
            Self::SpinOff => "spin_off",
            Self::Related => "related",
        }
    }

    /// Подпись со стороны `source`.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Sequel => "Сиквел",
            Self::Prequel => "Приквел",
            Self::Remake => "Ремейк",
            Self::SpinOff => "Ответвление",
            Self::Related => "Связано",
        }
    }

    /// Подпись со стороны `target`: «этот фильм является … для source».
    pub const fn inverse_label(self) -> &'static str {
        match self {
            Self::Sequel => "Приквел",
            Self::Prequel => "Сиквел",
            Self::Remake => "Оригинал",
            Self::SpinOff => "Основной фильм",
            Self::Related => "Связано",
        }
    }

    pub fn from_db_str(value: &str) -> Option<Self> {
        match value {
            "sequel" => Some(Self::Sequel),
            "prequel" => Some(Self::Prequel),
            "remake" => Some(Self::Remake),
            "spin_off" => Some(Self::SpinOff),
            "related" => Some(Self::Related),
            _ => None,
        }
    }
}

impl fmt::Display for RelationKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Строка таблицы `film_relations`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilmRelation {
    pub id: i64,
    pub source_id: i64,
    pub target_id: i64,
    pub kind: RelationKind,
}

/// Кадр или скриншот фильма.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilmScreenshot {
    pub id: i64,
    pub film_id: i64,
    pub image_path: String,
    #[serde(default)]
    pub caption: String,
    pub position: i64,
}

/// Черновик нового скриншота при добавлении к фильму.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct NewScreenshot {
    pub image_path: String,
    pub caption: String,
    pub position: i64,
}

/// Тема оформления интерфейса.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppTheme {
    #[default]
    Dark,
    Light,
}

impl AppTheme {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }

    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s {
            "dark" => Some(Self::Dark),
            "light" => Some(Self::Light),
            _ => None,
        }
    }
}

/// Сводная статистика библиотеки для страницы профиля (Dashboard).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DashboardStats {
    pub total_films: usize,
    pub watched_films: usize,
    pub want_to_watch: usize,
    pub watching: usize,
    pub dropped: usize,
    pub average_rating: Option<f64>,
    pub total_runtime_minutes: u64,
    pub top_genres: Vec<(String, usize)>,
    pub top_directors: Vec<(String, usize)>,
    pub total_seasons: usize,
    pub watched_seasons: usize,
}

/// Связь в том виде, в котором её видно со страницы конкретного фильма.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelatedFilm {
    pub relation_id: i64,
    pub film: Film,
    pub kind: RelationKind,
    /// `true`, если текущий фильм — `source`, `false` — `target`.
    pub outgoing: bool,
}

impl RelatedFilm {
    pub const fn label(&self) -> &'static str {
        if self.outgoing {
            self.kind.label()
        } else {
            self.kind.inverse_label()
        }
    }
}

/// Фильм, прочитанный из базы данных.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Film {
    pub id: i64,
    pub title: String,
    /// Название на языке оригинала. Пустая строка — не заполнено.
    pub original_title: String,
    pub category: String,
    pub director: String,
    /// Год выпуска. `None` — не указан.
    pub year: Option<u16>,
    /// Жанры через запятую.
    pub genres: String,
    pub country: String,
    /// Длительность в минутах.
    pub runtime_minutes: Option<u16>,
    /// Идентификатор IMDb (`tt...`), если известен.
    pub imdb_id: String,
    /// Пользовательские теги через запятую. Поле добавлено в миграции 005,
    /// поэтому старые архивы без него читаются с пустым значением.
    #[serde(default)]
    pub tags: String,
    pub status: FilmStatus,
    /// `None` — фильм ещё не оценён. Иначе 1..=10.
    pub rating: Option<u8>,
    pub description: String,
    pub thoughts: String,
    /// Путь к копии обложки. Обычно относительный (`posters/...`),
    /// разрешается через [`crate::storage::AppPaths::resolve_poster`].
    pub poster_path: Option<String>,
    /// 0 — сезоны не указаны.
    pub seasons_total: u16,
    /// Отсортированные уникальные номера просмотренных сезонов (1..=seasons_total).
    pub watched_seasons: Vec<u16>,
    /// Цитаты и любимые моменты. Поле добавлено в миграции 006,
    /// старые архивы без него читаются с пустым списком.
    #[serde(default)]
    pub quotes: Vec<String>,
    #[serde(default)]
    pub parent_id: Option<i64>,
}

impl Film {
    /// Человекочитаемая оценка для карточки.
    pub fn rating_label(&self) -> String {
        match self.rating {
            Some(value) => format!("{value}/10"),
            None => "Без оценки".to_owned(),
        }
    }

    pub fn watched_season_count(&self) -> usize {
        self.watched_seasons.len()
    }

    pub fn has_seasons(&self) -> bool {
        self.seasons_total > 0
    }

    /// Жанры, разобранные из строки `"Фантастика, Приключения"`.
    pub fn genres_list(&self) -> Vec<&str> {
        split_comma_list(&self.genres)
    }

    /// Пользовательские теги, разобранные из строки.
    pub fn tags_list(&self) -> Vec<&str> {
        split_comma_list(&self.tags)
    }

    pub fn runtime_label(&self) -> Option<String> {
        self.runtime_minutes.map(|minutes| format!("{minutes} мин"))
    }
}

/// Подробная информация о сезоне сериала (подготовка почвы для v1.2.0).
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SeasonDetails {
    pub season_number: u16,
    pub title: Option<String>,
    pub episodes_total: u16,
    pub episodes_watched: u16,
    pub air_year: Option<u16>,
    pub overview: Option<String>,
}

#[allow(dead_code)]
impl SeasonDetails {
    pub fn new(season_number: u16, episodes_total: u16) -> Self {
        Self {
            season_number,
            title: None,
            episodes_total,
            episodes_watched: 0,
            air_year: None,
            overview: None,
        }
    }

    pub fn is_completed(&self) -> bool {
        self.episodes_total > 0 && self.episodes_watched >= self.episodes_total
    }

    pub fn progress_percent(&self) -> u32 {
        if self.episodes_total == 0 {
            0
        } else {
            ((self.episodes_watched as f64 / self.episodes_total as f64) * 100.0) as u32
        }
    }
}

/// Разбирает список через запятую: обрезает пробелы и выбрасывает пустые.
pub fn split_comma_list(value: &str) -> Vec<&str> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .collect()
}

/// Нормализует список через запятую: без пустых элементов и дублей
/// (регистр при сравнении не важен, исходное написание сохраняется).
pub fn normalize_comma_list(value: &str) -> String {
    let mut items: Vec<String> = Vec::new();
    for item in split_comma_list(value) {
        let lowercase = item.to_lowercase();
        if items.iter().any(|known| known.to_lowercase() == lowercase) {
            continue;
        }
        items.push(item.to_owned());
    }
    items.join(", ")
}

/// Данные, необходимые для создания или обновления фильма.
///
/// Отделён от [`Film`], потому что у нового фильма ещё нет `id`,
/// а обложка уже должна быть импортирована в локальную папку.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewFilm {
    pub title: String,
    pub original_title: String,
    pub category: String,
    pub director: String,
    pub year: Option<u16>,
    pub genres: String,
    pub country: String,
    pub runtime_minutes: Option<u16>,
    pub imdb_id: String,
    pub tags: String,
    pub status: FilmStatus,
    pub rating: Option<u8>,
    pub description: String,
    pub thoughts: String,
    pub poster_path: Option<String>,
    pub seasons_total: u16,
    pub watched_seasons: Vec<u16>,
    pub quotes: Vec<String>,
    #[serde(default)]
    pub parent_id: Option<i64>,
}

impl Default for NewFilm {
    fn default() -> Self {
        Self {
            title: String::new(),
            original_title: String::new(),
            category: DEFAULT_CATEGORY.to_owned(),
            director: String::new(),
            year: None,
            genres: String::new(),
            country: String::new(),
            runtime_minutes: None,
            imdb_id: String::new(),
            tags: String::new(),
            status: FilmStatus::WantToWatch,
            rating: None,
            description: String::new(),
            thoughts: String::new(),
            poster_path: None,
            seasons_total: 0,
            watched_seasons: Vec::new(),
            quotes: Vec::new(),
            parent_id: None,
        }
    }
}

impl NewFilm {
    pub const MAX_TITLE_CHARS: usize = 200;
    pub const MAX_ORIGINAL_TITLE_CHARS: usize = 300;
    pub const MAX_CATEGORY_CHARS: usize = 60;
    pub const MAX_DIRECTOR_CHARS: usize = 200;
    pub const MAX_GENRES_CHARS: usize = 300;
    pub const MAX_COUNTRY_CHARS: usize = 120;
    pub const MAX_IMDB_ID_CHARS: usize = 20;
    pub const MAX_TAGS_CHARS: usize = 300;

    /// Нормализованная строка жанров: без пустых элементов и дублей.
    pub fn genres_db(&self) -> String {
        normalize_comma_list(&self.genres)
    }

    /// Нормализованная строка тегов: без пустых элементов и дублей.
    pub fn tags_db(&self) -> String {
        normalize_comma_list(&self.tags)
    }

    pub fn imdb_id_db(&self) -> String {
        self.imdb_id.trim().to_lowercase()
    }

    /// Нормализованный список просмотренных сезонов: без дублей, по возрастанию.
    pub fn normalized_watched_seasons(&self) -> Vec<u16> {
        let mut seasons = self.watched_seasons.clone();
        seasons.sort_unstable();
        seasons.dedup();
        seasons
    }

    /// Строка для хранения в SQLite: `"1,2,5"`.
    pub fn watched_seasons_db(&self) -> String {
        season_list_to_db(&self.normalized_watched_seasons())
    }

    /// JSON-строка со списком цитат для хранения в SQLite.
    pub fn quotes_db(&self) -> String {
        let cleaned: Vec<&str> = self
            .quotes
            .iter()
            .map(|q| q.trim())
            .filter(|q| !q.is_empty())
            .collect();
        serde_json::to_string(&cleaned).unwrap_or_else(|_| "[]".to_owned())
    }

    /// Проверка данных до обращения к БД.
    ///
    /// SQLite-ограничения остаются второй линией защиты, но пользователю
    /// нужно показывать понятную ошибку ещё до сохранения.
    pub fn validate(&self) -> Result<(), FilmValidationError> {
        let title = self.title.trim();
        if title.is_empty() {
            return Err(FilmValidationError::EmptyTitle);
        }
        if title.chars().count() > Self::MAX_TITLE_CHARS {
            return Err(FilmValidationError::TitleTooLong(Self::MAX_TITLE_CHARS));
        }

        let category = self.category.trim();
        if category.is_empty() {
            return Err(FilmValidationError::EmptyCategory);
        }
        if category.chars().count() > Self::MAX_CATEGORY_CHARS {
            return Err(FilmValidationError::CategoryTooLong(
                Self::MAX_CATEGORY_CHARS,
            ));
        }

        if self.original_title.trim().chars().count() > Self::MAX_ORIGINAL_TITLE_CHARS {
            return Err(FilmValidationError::OriginalTitleTooLong(
                Self::MAX_ORIGINAL_TITLE_CHARS,
            ));
        }
        if self.director.trim().chars().count() > Self::MAX_DIRECTOR_CHARS {
            return Err(FilmValidationError::DirectorTooLong(
                Self::MAX_DIRECTOR_CHARS,
            ));
        }
        if self.genres.trim().chars().count() > Self::MAX_GENRES_CHARS {
            return Err(FilmValidationError::GenresTooLong(Self::MAX_GENRES_CHARS));
        }
        if self.country.trim().chars().count() > Self::MAX_COUNTRY_CHARS {
            return Err(FilmValidationError::CountryTooLong(Self::MAX_COUNTRY_CHARS));
        }
        if self.imdb_id.trim().chars().count() > Self::MAX_IMDB_ID_CHARS {
            return Err(FilmValidationError::ImdbIdTooLong(Self::MAX_IMDB_ID_CHARS));
        }
        if self.tags.trim().chars().count() > Self::MAX_TAGS_CHARS {
            return Err(FilmValidationError::TagsTooLong(Self::MAX_TAGS_CHARS));
        }

        if let Some(year) = self.year
            && !(MIN_YEAR..=MAX_YEAR).contains(&year)
        {
            return Err(FilmValidationError::YearOutOfRange(MIN_YEAR, MAX_YEAR));
        }
        if let Some(runtime) = self.runtime_minutes
            && !(1..=MAX_RUNTIME_MINUTES).contains(&runtime)
        {
            return Err(FilmValidationError::RuntimeOutOfRange(MAX_RUNTIME_MINUTES));
        }

        if self.seasons_total > MAX_SEASONS {
            return Err(FilmValidationError::TooManySeasons(MAX_SEASONS));
        }
        if let Some(season) = self
            .normalized_watched_seasons()
            .into_iter()
            .find(|season| *season == 0 || *season > self.seasons_total)
        {
            return Err(FilmValidationError::SeasonOutOfRange(
                season,
                self.seasons_total,
            ));
        }

        if let Some(rating) = self.rating
            && !(1..=10).contains(&rating)
        {
            return Err(FilmValidationError::RatingOutOfRange);
        }
        Ok(())
    }
}

/// Разбирает строку из БД (`"1,2,5"`) и отбрасывает некорректные номера.
pub fn parse_watched_seasons(value: &str, seasons_total: u16) -> Vec<u16> {
    let mut seasons: Vec<u16> = value
        .split(',')
        .filter_map(|part| part.trim().parse::<u16>().ok())
        .filter(|season| *season >= 1 && *season <= seasons_total)
        .collect();
    seasons.sort_unstable();
    seasons.dedup();
    seasons
}

/// Сериализует номера сезонов для хранения в SQLite.
pub fn season_list_to_db(seasons: &[u16]) -> String {
    seasons
        .iter()
        .map(u16::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

/// Разбирает список цитат из JSON-массива строк или текстовых строк.
pub fn parse_quotes(value: &str) -> Vec<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    if let Ok(quotes) = serde_json::from_str::<Vec<String>>(trimmed) {
        return quotes
            .into_iter()
            .map(|q| q.trim().to_owned())
            .filter(|q| !q.is_empty())
            .collect();
    }
    // Fallback для строк с разделителем переноса строки:
    trimmed
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum FilmValidationError {
    #[error("Название фильма не может быть пустым")]
    EmptyTitle,
    #[error("Название слишком длинное: максимум {0} символов")]
    TitleTooLong(usize),
    #[error("Категория не может быть пустой")]
    EmptyCategory,
    #[error("Название категории слишком длинное: максимум {0} символов")]
    CategoryTooLong(usize),
    #[error("Оригинальное название слишком длинное: максимум {0} символов")]
    OriginalTitleTooLong(usize),
    #[error("Имя режиссёра слишком длинное: максимум {0} символов")]
    DirectorTooLong(usize),
    #[error("Список жанров слишком длинный: максимум {0} символов")]
    GenresTooLong(usize),
    #[error("Название страны слишком длинное: максимум {0} символов")]
    CountryTooLong(usize),
    #[error("Идентификатор IMDb слишком длинный: максимум {0} символов")]
    ImdbIdTooLong(usize),
    #[error("Список тегов слишком длинный: максимум {0} символов")]
    TagsTooLong(usize),
    #[error("Год должен быть в диапазоне {0}..={1}")]
    YearOutOfRange(u16, u16),
    #[error("Длительность должна быть от 1 до {0} минут")]
    RuntimeOutOfRange(u16),
    #[error("Слишком много сезонов: максимум {0}")]
    TooManySeasons(u16),
    #[error("Сезон {0} вне диапазона 1..={1}")]
    SeasonOutOfRange(u16, u16),
    #[error("Оценка должна быть числом от 1 до 10")]
    RatingOutOfRange,
}

/// Параметры поиска и фильтрации списка.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilmFilter {
    pub search: String,
    pub status: Option<FilmStatus>,
    pub category: Option<String>,
}

impl FilmFilter {
    pub fn is_active(&self) -> bool {
        !self.search.trim().is_empty() || self.status.is_some() || self.category.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_title_and_rating() {
        let mut film = NewFilm::default();
        assert_eq!(film.validate(), Err(FilmValidationError::EmptyTitle));

        film.title = "  Интерстеллар  ".to_owned();
        film.rating = Some(10);
        assert!(film.validate().is_ok());

        film.rating = Some(11);
        assert_eq!(film.validate(), Err(FilmValidationError::RatingOutOfRange));

        film.rating = Some(0);
        assert_eq!(film.validate(), Err(FilmValidationError::RatingOutOfRange));

        film.rating = None;
        film.title = "x".repeat(NewFilm::MAX_TITLE_CHARS + 1);
        assert_eq!(
            film.validate(),
            Err(FilmValidationError::TitleTooLong(NewFilm::MAX_TITLE_CHARS))
        );

        film.title = "OK".to_owned();
        film.category.clear();
        assert_eq!(film.validate(), Err(FilmValidationError::EmptyCategory));

        film.category = "я".repeat(NewFilm::MAX_CATEGORY_CHARS + 1);
        assert_eq!(
            film.validate(),
            Err(FilmValidationError::CategoryTooLong(
                NewFilm::MAX_CATEGORY_CHARS
            ))
        );
    }

    #[test]
    fn seasons_are_normalized_and_validated() {
        let mut film = NewFilm {
            title: "Сериал".to_owned(),
            seasons_total: 3,
            watched_seasons: vec![3, 1, 1],
            ..NewFilm::default()
        };

        assert!(film.validate().is_ok());
        assert_eq!(film.normalized_watched_seasons(), vec![1, 3]);
        assert_eq!(film.watched_seasons_db(), "1,3");

        film.watched_seasons = vec![4];
        assert_eq!(
            film.validate(),
            Err(FilmValidationError::SeasonOutOfRange(4, 3))
        );

        assert_eq!(parse_watched_seasons("1,2,2,0,9", 3), vec![1, 2]);
        assert_eq!(season_list_to_db(&[5, 2, 2, 1]), "5,2,2,1");
    }

    #[test]
    fn validates_extended_metadata() {
        let mut film = NewFilm {
            title: "Дюна".to_owned(),
            year: Some(2021),
            genres: "Фантастика, Приключения".to_owned(),
            country: "США".to_owned(),
            runtime_minutes: Some(155),
            imdb_id: "tt1160419".to_owned(),
            tags: "эпик, на пересмотр".to_owned(),
            ..NewFilm::default()
        };
        assert!(film.validate().is_ok());
        assert_eq!(film.genres_db(), "Фантастика, Приключения");
        assert_eq!(film.tags_db(), "эпик, на пересмотр");

        film.genres = "Драма, драма, , Драма".to_owned();
        assert_eq!(film.genres_db(), "Драма");

        film.tags = "Пересмотр, пересмотр, , С друзьями".to_owned();
        assert_eq!(film.tags_db(), "Пересмотр, С друзьями");

        film.tags = "т".repeat(NewFilm::MAX_TAGS_CHARS + 1);
        assert_eq!(
            film.validate(),
            Err(FilmValidationError::TagsTooLong(NewFilm::MAX_TAGS_CHARS))
        );
        film.tags = "ок".to_owned();

        film.year = Some(1800);
        assert_eq!(
            film.validate(),
            Err(FilmValidationError::YearOutOfRange(MIN_YEAR, MAX_YEAR))
        );
        film.year = Some(2021);

        film.runtime_minutes = Some(0);
        assert_eq!(
            film.validate(),
            Err(FilmValidationError::RuntimeOutOfRange(MAX_RUNTIME_MINUTES))
        );
        film.runtime_minutes = Some(120);

        film.genres = "ж".repeat(NewFilm::MAX_GENRES_CHARS + 1);
        assert_eq!(
            film.validate(),
            Err(FilmValidationError::GenresTooLong(
                NewFilm::MAX_GENRES_CHARS
            ))
        );
    }

    #[test]
    fn relation_labels_are_symmetric() {
        assert_eq!(RelationKind::Sequel.label(), "Сиквел");
        assert_eq!(RelationKind::Sequel.inverse_label(), "Приквел");
        assert_eq!(RelationKind::Remake.inverse_label(), "Оригинал");
        for kind in RelationKind::ALL {
            assert_eq!(RelationKind::from_db_str(kind.as_db_str()), Some(kind));
        }
    }

    #[test]
    fn status_roundtrip() {
        for status in FilmStatus::ALL {
            assert_eq!(FilmStatus::from_db_str(status.as_db_str()), Some(status));
        }
        assert_eq!(FilmStatus::from_db_str("unknown"), None);
    }

    #[test]
    fn quotes_serialization_and_parsing() {
        let quotes = vec!["Цитата 1".to_string(), "Цитата 2".to_string()];
        let json = serde_json::to_string(&quotes).unwrap();
        assert_eq!(parse_quotes(&json), quotes);
        assert_eq!(
            parse_quotes("Цитата A\nЦитата B"),
            vec!["Цитата A", "Цитата B"]
        );
        assert!(parse_quotes("").is_empty());
    }

    #[test]
    fn season_details_progress_and_completion() {
        let mut season = SeasonDetails::new(1, 10);
        assert_eq!(season.progress_percent(), 0);
        assert!(!season.is_completed());

        season.episodes_watched = 5;
        assert_eq!(season.progress_percent(), 50);
        assert!(!season.is_completed());

        season.episodes_watched = 10;
        assert_eq!(season.progress_percent(), 100);
        assert!(season.is_completed());
    }
}
