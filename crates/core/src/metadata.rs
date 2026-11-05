//! Необязательное автозаполнение метаданных через OMDb API.
//!
//! Приложение остаётся полностью локальным: сеть используется только когда
//! пользователь сам нажимает «Заполнить из OMDb» и указал бесплатный ключ.
//! Ключ хранится в таблице `settings` рядом с библиотекой.

use std::time::Duration;

use serde::Deserialize;
use thiserror::Error;

use crate::models::{MAX_RUNTIME_MINUTES, MAX_SEASONS, MAX_YEAR, MIN_YEAR};

pub const OMDB_API_KEY_SETTING: &str = "omdb_api_key";
pub const OMDB_KEY_HELP_URL: &str = "https://www.omdbapi.com/apikey.aspx";

const OMDB_ENDPOINT: &str = "https://www.omdbapi.com/";
const MAX_POSTER_BYTES: usize = 20 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum MetadataError {
    #[error(
        "не задан ключ OMDb API. Откройте «Настройки» и вставьте бесплатный ключ с omdbapi.com"
    )]
    MissingApiKey,
    #[error("не задано название для поиска")]
    EmptyQuery,
    #[error("фильм не найден в OMDb. Попробуйте оригинальное (английское) название")]
    NotFound,
    #[error("OMDb: {0}")]
    Api(String),
    #[error("сетевая ошибка: {0}")]
    Network(String),
    #[error("некорректный ответ OMDb: {0}")]
    Parse(String),
}

use serde::Serialize;

/// Данные, которые удалось получить из OMDb.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MovieMetadata {
    pub title: String,
    pub year: Option<u16>,
    pub director: String,
    pub genres: String,
    pub country: String,
    pub runtime_minutes: Option<u16>,
    pub description: String,
    pub imdb_id: String,
    pub total_seasons: u16,
    pub poster_url: Option<String>,
    /// Скачанные байты постера (может не быть, если загрузка не удалась).
    pub poster_bytes: Option<Vec<u8>>,
}

/// Ищет фильм по названию и, если нужно, по году выпуска.
///
/// Если точное совпадение не найдено, OMDb ищется через `s=`, после чего
/// метаданные запрашиваются по найденному `imdbID`.
pub fn fetch(
    api_key: &str,
    query: &str,
    year: Option<u16>,
) -> Result<MovieMetadata, MetadataError> {
    let api_key = api_key.trim();
    if api_key.is_empty() {
        return Err(MetadataError::MissingApiKey);
    }
    let query = query.trim();
    if query.is_empty() {
        return Err(MetadataError::EmptyQuery);
    }

    let agent = build_agent();

    match fetch_by_title(&agent, api_key, query, year) {
        Ok(metadata) => Ok(metadata),
        Err(MetadataError::NotFound) => {
            let Some(imdb_id) = search_first_id(&agent, api_key, query)? else {
                return Err(MetadataError::NotFound);
            };
            fetch_by_id(&agent, api_key, &imdb_id)
        }
        Err(error) => Err(error),
    }
}

fn build_agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(20)))
        .user_agent("MyFilmLibrary/0.1 (local desktop app)")
        .build()
        .into()
}

fn fetch_by_title(
    agent: &ureq::Agent,
    api_key: &str,
    title: &str,
    year: Option<u16>,
) -> Result<MovieMetadata, MetadataError> {
    let mut request = agent
        .get(OMDB_ENDPOINT)
        .query("apikey", api_key)
        .query("t", title)
        .query("plot", "full");
    if let Some(year) = year {
        request = request.query("y", year.to_string());
    }

    let mut response = request.call().map_err(map_ureq_error)?;
    let json: String = response
        .body_mut()
        .read_to_string()
        .map_err(|error| MetadataError::Parse(error.to_string()))?;
    let details: OmdbDetails =
        serde_json::from_str(&json).map_err(|error| MetadataError::Parse(error.to_string()))?;

    if details.is_success() {
        let mut metadata = metadata_from_details(details);
        metadata.poster_bytes = metadata
            .poster_url
            .as_deref()
            .and_then(|url| download_poster(agent, url).ok());
        Ok(metadata)
    } else {
        Err(error_from_omdb(details.error.as_deref()))
    }
}

fn fetch_by_id(
    agent: &ureq::Agent,
    api_key: &str,
    imdb_id: &str,
) -> Result<MovieMetadata, MetadataError> {
    let mut response = agent
        .get(OMDB_ENDPOINT)
        .query("apikey", api_key)
        .query("i", imdb_id)
        .query("plot", "full")
        .call()
        .map_err(map_ureq_error)?;
    let json: String = response
        .body_mut()
        .read_to_string()
        .map_err(|error| MetadataError::Parse(error.to_string()))?;
    let details: OmdbDetails =
        serde_json::from_str(&json).map_err(|error| MetadataError::Parse(error.to_string()))?;

    if details.is_success() {
        let mut metadata = metadata_from_details(details);
        metadata.poster_bytes = metadata
            .poster_url
            .as_deref()
            .and_then(|url| download_poster(agent, url).ok());
        Ok(metadata)
    } else {
        Err(error_from_omdb(details.error.as_deref()))
    }
}

fn search_first_id(
    agent: &ureq::Agent,
    api_key: &str,
    query: &str,
) -> Result<Option<String>, MetadataError> {
    let mut response = agent
        .get(OMDB_ENDPOINT)
        .query("apikey", api_key)
        .query("s", query)
        .call()
        .map_err(map_ureq_error)?;
    let json: String = response
        .body_mut()
        .read_to_string()
        .map_err(|error| MetadataError::Parse(error.to_string()))?;
    let parsed: OmdbSearch =
        serde_json::from_str(&json).map_err(|error| MetadataError::Parse(error.to_string()))?;

    if parsed.response.eq_ignore_ascii_case("true") {
        Ok(parsed
            .search
            .and_then(|mut results| {
                if results.is_empty() {
                    None
                } else {
                    Some(results.remove(0).imdb_id)
                }
            })
            .filter(|id| !id.trim().is_empty()))
    } else {
        match error_from_omdb(parsed.error.as_deref()) {
            MetadataError::NotFound => Ok(None),
            other => Err(other),
        }
    }
}

fn download_poster(agent: &ureq::Agent, url: &str) -> Result<Vec<u8>, MetadataError> {
    let mut response = agent.get(url).call().map_err(map_ureq_error)?;
    let bytes = response
        .body_mut()
        .read_to_vec()
        .map_err(|error| MetadataError::Network(error.to_string()))?;

    if bytes.is_empty() {
        return Err(MetadataError::Parse("пустой файл постера".to_owned()));
    }
    if bytes.len() > MAX_POSTER_BYTES {
        return Err(MetadataError::Parse(
            "файл постера слишком большой".to_owned(),
        ));
    }

    Ok(bytes)
}

fn map_ureq_error(error: ureq::Error) -> MetadataError {
    match error {
        ureq::Error::StatusCode(401) | ureq::Error::StatusCode(403) => {
            MetadataError::Api("ключ API отклонён. Проверьте ключ в «Настройках»".to_owned())
        }
        ureq::Error::StatusCode(code) => MetadataError::Api(format!("HTTP {code}")),
        ureq::Error::HostNotFound => {
            MetadataError::Network("не удалось найти сервер OMDb. Проверьте интернет".to_owned())
        }
        ureq::Error::Timeout(_) => {
            MetadataError::Network("сервер OMDb не ответил вовремя".to_owned())
        }
        other => MetadataError::Network(other.to_string()),
    }
}

fn error_from_omdb(message: Option<&str>) -> MetadataError {
    let message = message.unwrap_or("неизвестная ошибка").trim().to_owned();
    let lowercase = message.to_lowercase();

    if lowercase.contains("not found") {
        MetadataError::NotFound
    } else if lowercase.contains("invalid api key") {
        MetadataError::Api("ключ API отклонён. Проверьте ключ в «Настройках»".to_owned())
    } else if lowercase.contains("request limit") {
        MetadataError::Api("исчерпан дневной лимит запросов OMDb".to_owned())
    } else {
        MetadataError::Api(message)
    }
}

#[derive(Debug, Deserialize)]
struct OmdbDetails {
    #[serde(rename = "Response")]
    response: String,
    #[serde(rename = "Error")]
    error: Option<String>,
    #[serde(rename = "Title")]
    title: Option<String>,
    #[serde(rename = "Year")]
    year: Option<String>,
    #[serde(rename = "Runtime")]
    runtime: Option<String>,
    #[serde(rename = "Genre")]
    genre: Option<String>,
    #[serde(rename = "Director")]
    director: Option<String>,
    #[serde(rename = "Plot")]
    plot: Option<String>,
    #[serde(rename = "Poster")]
    poster: Option<String>,
    #[serde(rename = "Country")]
    country: Option<String>,
    #[serde(rename = "imdbID")]
    imdb_id: Option<String>,
    #[serde(rename = "totalSeasons")]
    total_seasons: Option<String>,
}

impl OmdbDetails {
    fn is_success(&self) -> bool {
        self.response.eq_ignore_ascii_case("true")
    }
}

#[derive(Debug, Deserialize)]
struct OmdbSearch {
    #[serde(rename = "Response")]
    response: String,
    #[serde(rename = "Error")]
    error: Option<String>,
    #[serde(rename = "Search")]
    search: Option<Vec<OmdbSearchItem>>,
}

#[derive(Debug, Deserialize)]
struct OmdbSearchItem {
    #[serde(rename = "imdbID")]
    imdb_id: String,
}

fn metadata_from_details(details: OmdbDetails) -> MovieMetadata {
    let poster_url = details
        .poster
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty() && value != "N/A");

    MovieMetadata {
        title: clean(details.title),
        year: details.year.as_deref().and_then(parse_year),
        director: clean(details.director),
        genres: translate_genres(details.genre.as_deref().unwrap_or_default()),
        country: clean(details.country),
        runtime_minutes: details.runtime.as_deref().and_then(parse_runtime),
        description: clean(details.plot),
        imdb_id: clean(details.imdb_id),
        total_seasons: details
            .total_seasons
            .as_deref()
            .and_then(|value| value.trim().parse::<u16>().ok())
            .unwrap_or(0)
            .min(MAX_SEASONS),
        poster_url,
        poster_bytes: None,
    }
}

fn clean(value: Option<String>) -> String {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| value != "N/A")
        .unwrap_or_default()
}

fn parse_year(value: &str) -> Option<u16> {
    let digits: String = value
        .chars()
        .skip_while(|ch| !ch.is_ascii_digit())
        .take_while(|ch| ch.is_ascii_digit())
        .take(4)
        .collect();
    digits
        .parse::<u16>()
        .ok()
        .filter(|year| (MIN_YEAR..=MAX_YEAR).contains(year))
}

fn parse_runtime(value: &str) -> Option<u16> {
    let digits: String = value
        .chars()
        .skip_while(|ch| !ch.is_ascii_digit())
        .take_while(|ch| ch.is_ascii_digit())
        .collect();
    digits
        .parse::<u16>()
        .ok()
        .filter(|minutes| (1..=MAX_RUNTIME_MINUTES).contains(minutes))
}

/// Переводит самые частые жанры OMDb, остальные оставляет как есть.
pub fn translate_genres(value: &str) -> String {
    value
        .split(',')
        .map(str::trim)
        .filter(|genre| !genre.is_empty())
        .map(translate_genre)
        .collect::<Vec<_>>()
        .join(", ")
}

fn translate_genre(genre: &str) -> String {
    let translated = match genre.to_ascii_lowercase().as_str() {
        "action" => "Боевик",
        "adventure" => "Приключения",
        "animation" => "Анимация",
        "biography" => "Биография",
        "comedy" => "Комедия",
        "crime" => "Криминал",
        "documentary" => "Документальный",
        "drama" => "Драма",
        "family" => "Семейный",
        "fantasy" => "Фэнтези",
        "film-noir" => "Нуар",
        "game-show" => "Игровое шоу",
        "history" => "История",
        "horror" => "Ужасы",
        "music" => "Музыка",
        "musical" => "Мюзикл",
        "mystery" => "Детектив",
        "news" => "Новости",
        "reality-tv" => "Реалити-шоу",
        "romance" => "Романтика",
        "sci-fi" => "Фантастика",
        "short" => "Короткометражка",
        "sport" => "Спорт",
        "talk-show" => "Ток-шоу",
        "thriller" => "Триллер",
        "war" => "Военный",
        "western" => "Вестерн",
        _ => return genre.to_owned(),
    };
    translated.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "Title": "The Matrix",
        "Year": "1999",
        "Rated": "R",
        "Runtime": "136 min",
        "Genre": "Action, Sci-Fi",
        "Director": "Lana Wachowski, Lilly Wachowski",
        "Plot": "A computer hacker learns the truth.",
        "Poster": "https://example.com/matrix.jpg",
        "Country": "United States",
        "imdbID": "tt0133093",
        "Response": "True"
    }"#;

    #[test]
    fn parses_omdb_details() {
        let details: OmdbDetails = serde_json::from_str(SAMPLE).unwrap();
        let metadata = metadata_from_details(details);

        assert_eq!(metadata.title, "The Matrix");
        assert_eq!(metadata.year, Some(1999));
        assert_eq!(metadata.runtime_minutes, Some(136));
        assert_eq!(metadata.genres, "Боевик, Фантастика");
        assert_eq!(metadata.director, "Lana Wachowski, Lilly Wachowski");
        assert_eq!(metadata.country, "United States");
        assert_eq!(metadata.imdb_id, "tt0133093");
        assert_eq!(
            metadata.poster_url.as_deref(),
            Some("https://example.com/matrix.jpg")
        );
    }

    #[test]
    fn parses_year_and_runtime_edges() {
        assert_eq!(parse_year("2010–2015"), Some(2010));
        assert_eq!(parse_year("N/A"), None);
        assert_eq!(parse_runtime("148 min"), Some(148));
        assert_eq!(parse_runtime("N/A"), None);
    }

    #[test]
    fn reports_missing_key_and_genres() {
        assert!(matches!(
            fetch("  ", "The Matrix", None),
            Err(MetadataError::MissingApiKey)
        ));
        assert_eq!(
            translate_genres("Drama, Mystery, Unknown"),
            "Драма, Детектив, Unknown"
        );
    }

    #[test]
    fn recognizes_omdb_errors() {
        assert!(matches!(
            error_from_omdb(Some("Movie not found!")),
            MetadataError::NotFound
        ));
        assert!(matches!(
            error_from_omdb(Some("Invalid API key!")),
            MetadataError::Api(_)
        ));
    }
}
