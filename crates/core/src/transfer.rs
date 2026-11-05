//! Экспорт и импорт фильмографии в ZIP-архив.
//!
//! Внутри архива лежат `library.json` (фильмы и связи) и папка `posters/`
//! с копиями обложек. Импорт добавляет недостающие фильмы, пропускает
//! дубликаты и восстанавливает связи между ними.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use zip::CompressionMethod;
use zip::read::ZipArchive;
use zip::write::SimpleFileOptions;

use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::models::{Film, FilmRelation, FilmScreenshot, MediaType, NewFilm};
use crate::storage::{self, AppPaths};

const BUNDLE_VERSION: u32 = 1;
const BUNDLE_FILE: &str = "library.json";
const POSTERS_PREFIX: &str = "posters/";

#[derive(Debug, Serialize, Deserialize)]
struct LibraryBundle {
    version: u32,
    films: Vec<Film>,
    relations: Vec<FilmRelation>,
    #[serde(default)]
    screenshots: Vec<FilmScreenshot>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExportSummary {
    pub films: usize,
    pub relations: usize,
    pub posters: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImportSummary {
    pub added: usize,
    pub skipped: usize,
    pub relations_added: usize,
    pub posters_restored: usize,
    pub warnings: Vec<String>,
}

impl ImportSummary {
    /// Короткое сообщение для тоста.
    pub fn message(&self) -> String {
        let mut message = format!(
            "Импортировано: {} фильмов, пропущено дублей: {}, связей: {}",
            self.added, self.skipped, self.relations_added
        );
        if self.posters_restored > 0 {
            message.push_str(&format!(", обложек: {}", self.posters_restored));
        }
        if !self.warnings.is_empty() {
            message.push_str(&format!(". Предупреждения: {}", self.warnings.join("; ")));
        }
        message
    }
}

/// Сохраняет все фильмы, связи и обложки в ZIP-архив.
pub fn export_library(db: &Database, paths: &AppPaths, target: &Path) -> AppResult<ExportSummary> {
    let films = db.list_all_films()?;
    let relations = db.list_all_relations()?;
    let screenshots = db.list_all_screenshots()?;

    let bundle = LibraryBundle {
        version: BUNDLE_VERSION,
        films: films.clone(),
        relations: relations.clone(),
        screenshots: screenshots.clone(),
    };

    let file = fs::File::create(target)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);

    zip.start_file(BUNDLE_FILE, options)
        .map_err(|error| AppError::Archive(error.to_string()))?;
    let json = serde_json::to_vec_pretty(&bundle)
        .map_err(|error| AppError::Transfer(format!("Не удалось подготовить JSON: {error}")))?;
    zip.write_all(&json)?;

    let mut written_posters = HashSet::new();
    let mut posters = 0;

    for film in &films {
        let Some(stored_path) = film.poster_path.as_deref() else {
            continue;
        };
        let absolute = paths.resolve_poster(stored_path);
        if !absolute.is_file() {
            continue;
        }
        let Some(file_name) = absolute.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let entry_name = format!("{POSTERS_PREFIX}{file_name}");
        if !written_posters.insert(entry_name.clone()) {
            continue;
        }

        zip.start_file(entry_name, options)
            .map_err(|error| AppError::Archive(error.to_string()))?;
        let mut source = fs::File::open(&absolute)?;
        std::io::copy(&mut source, &mut zip)?;
        posters += 1;
    }

    let mut written_screenshots = HashSet::new();
    for shot in &screenshots {
        let absolute = paths.resolve_screenshot(&shot.image_path);
        if !absolute.is_file() {
            continue;
        }
        let Some(file_name) = absolute.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let entry_name = format!("screenshots/{file_name}");
        if !written_screenshots.insert(entry_name.clone()) {
            continue;
        }

        zip.start_file(entry_name, options)
            .map_err(|error| AppError::Archive(error.to_string()))?;
        let mut source = fs::File::open(&absolute)?;
        std::io::copy(&mut source, &mut zip)?;
    }

    zip.finish()
        .map_err(|error| AppError::Archive(error.to_string()))?;

    Ok(ExportSummary {
        films: films.len(),
        relations: relations.len(),
        posters,
    })
}

/// Добавляет фильмы и связи из ZIP-архива.
///
/// Совпадением считается тот же `imdb_id` либо одинаковые название и год.
/// Такие фильмы не дублируются, а импортируемые связи прикрепляются к уже
/// существующим записям.
pub fn import_library(
    db: &mut Database,
    paths: &AppPaths,
    source: &Path,
) -> AppResult<ImportSummary> {
    let file = fs::File::open(source)?;
    let mut zip = ZipArchive::new(file).map_err(|error| AppError::Archive(error.to_string()))?;

    let bundle: LibraryBundle = {
        let mut entry = zip
            .by_name(BUNDLE_FILE)
            .map_err(|_| AppError::Transfer("в архиве нет файла library.json".to_owned()))?;
        let mut json = String::new();
        entry.read_to_string(&mut json)?;
        serde_json::from_str(&json).map_err(|error| {
            AppError::Transfer(format!("Не удалось прочитать library.json: {error}"))
        })?
    };

    if bundle.version > BUNDLE_VERSION {
        return Err(AppError::Transfer(format!(
            "архив создан более новой версией приложения (формат {}). Обновите приложение",
            bundle.version
        )));
    }

    let existing_films = db.list_all_films()?;
    // (id, название, год, imdb_id, media_type) — для поиска дублей.
    let mut known: Vec<(i64, String, Option<u16>, String, MediaType)> = existing_films
        .iter()
        .map(|film| {
            (
                film.id,
                film.title.clone(),
                film.year,
                film.imdb_id.clone(),
                film.media_type,
            )
        })
        .collect();
    let existing_ids: Vec<i64> = existing_films.iter().map(|film| film.id).collect();

    let mut summary = ImportSummary::default();
    let mut id_map: HashMap<i64, i64> = HashMap::new();
    let mut imported_ids: Vec<i64> = Vec::new();

    for film in &bundle.films {
        if let Some(existing_id) = known
            .iter()
            .find(|(_, title, year, imdb_id, media_type)| {
                is_duplicate(film, title, *year, imdb_id, *media_type)
            })
            .map(|(id, _, _, _, _)| *id)
        {
            id_map.insert(film.id, existing_id);
            summary.skipped += 1;
            continue;
        }

        let mut imported_poster = None;
        let mut poster_path = None;

        if let Some(stored_path) = film.poster_path.as_deref() {
            match import_poster_from_zip(&mut zip, paths, stored_path) {
                Ok(Some(poster)) => {
                    poster_path = Some(poster.relative_path.clone());
                    imported_poster = Some(poster);
                }
                Ok(None) => {}
                Err(error) => summary.warnings.push(format!(
                    "«{}»: обложка не восстановлена ({error})",
                    film.title
                )),
            }
        }

        let draft = draft_from_export(film, poster_path);
        match db.insert_film(&draft) {
            Ok(new_id) => {
                id_map.insert(film.id, new_id);
                imported_ids.push(new_id);
                known.push((
                    new_id,
                    draft.title.clone(),
                    draft.year,
                    draft.imdb_id.clone(),
                    draft.media_type,
                ));
                summary.added += 1;
                if imported_poster.is_some() {
                    summary.posters_restored += 1;
                }
            }
            Err(error) => {
                // Не оставляем осиротевшую копию обложки.
                if let Some(poster) = imported_poster {
                    let _ = storage::delete_poster(paths, &poster.relative_path);
                }
                summary.warnings.push(format!("«{}»: {error}", film.title));
            }
        }
    }

    for relation in &bundle.relations {
        let (Some(&source_id), Some(&target_id)) = (
            id_map.get(&relation.source_id),
            id_map.get(&relation.target_id),
        ) else {
            continue;
        };
        if source_id == target_id {
            continue;
        }

        match db.add_relation(source_id, target_id, relation.kind) {
            Ok(true) => summary.relations_added += 1,
            Ok(false) => {}
            Err(error) => summary.warnings.push(format!(
                "Связь «{}» — «{}» не добавлена: {error}",
                relation.source_id, relation.target_id
            )),
        }
    }

    // Восстанавливаем кадры/скриншоты
    for screenshot in &bundle.screenshots {
        let Some(&new_film_id) = id_map.get(&screenshot.film_id) else {
            continue;
        };
        match import_screenshot_from_zip(&mut zip, paths, &screenshot.image_path) {
            Ok(Some(imported)) => {
                let _ =
                    db.add_screenshot(new_film_id, &imported.relative_path, &screenshot.caption);
            }
            Ok(None) => {}
            Err(error) => {
                summary
                    .warnings
                    .push(format!("Кадр не восстановлен: {error}"));
            }
        }
    }

    // Новые фильмы дописываем после существующих, сохраняя порядок архива.
    if !imported_ids.is_empty() {
        let mut order = existing_ids;
        order.extend(imported_ids);
        db.set_film_order(&order)?;
    }

    Ok(summary)
}

fn draft_from_export(film: &Film, poster_path: Option<String>) -> NewFilm {
    NewFilm {
        title: film.title.clone(),
        original_title: film.original_title.clone(),
        category: film.category.clone(),
        director: film.director.clone(),
        year: film.year,
        genres: film.genres.clone(),
        country: film.country.clone(),
        runtime_minutes: film.runtime_minutes,
        imdb_id: film.imdb_id.clone(),
        tags: film.tags.clone(),
        status: film.status,
        rating: film.rating,
        description: film.description.clone(),
        thoughts: film.thoughts.clone(),
        poster_path,
        seasons_total: film.seasons_total,
        watched_seasons: film.watched_seasons.clone(),
        quotes: film.quotes.clone(),
        parent_id: film.parent_id,
        media_type: film.media_type,
        platform: film.platform.clone(),
        launcher: film.launcher.clone(),
        playtime_hours: film.playtime_hours,
        completion_status: film.completion_status.clone(),
    }
}

fn is_duplicate(
    imported: &Film,
    existing_title: &str,
    existing_year: Option<u16>,
    existing_imdb_id: &str,
    existing_media_type: MediaType,
) -> bool {
    if imported.media_type != existing_media_type {
        return false;
    }
    let same_imdb = !imported.imdb_id.trim().is_empty()
        && imported.imdb_id.trim().to_lowercase() == existing_imdb_id.trim().to_lowercase();
    // to_lowercase, а не eq_ignore_ascii_case: названия бывают кириллическими.
    let same_title = imported.title.trim().to_lowercase() == existing_title.trim().to_lowercase()
        && imported.year == existing_year;

    same_imdb || same_title
}

fn import_poster_from_zip(
    zip: &mut ZipArchive<fs::File>,
    paths: &AppPaths,
    stored_path: &str,
) -> AppResult<Option<storage::ImportedPoster>> {
    let Some(file_name) = storage::poster_file_name(stored_path) else {
        return Ok(None);
    };
    let entry_name = format!("{POSTERS_PREFIX}{file_name}");

    let mut bytes = Vec::new();
    match zip.by_name(&entry_name) {
        Ok(mut entry) => {
            entry.read_to_end(&mut bytes)?;
        }
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(error) => return Err(AppError::Archive(error.to_string())),
    }

    if bytes.is_empty() {
        return Ok(None);
    }

    let image = image::load_from_memory(&bytes)?.to_rgba8();
    storage::validate_image_size(image.width(), image.height())?;
    Ok(Some(storage::import_poster_image(&image, paths)?))
}

fn import_screenshot_from_zip(
    zip: &mut ZipArchive<fs::File>,
    paths: &AppPaths,
    stored_path: &str,
) -> AppResult<Option<storage::ImportedPoster>> {
    let Some(file_name) = storage::poster_file_name(stored_path) else {
        return Ok(None);
    };
    let entry_name = format!("screenshots/{file_name}");

    let mut bytes = Vec::new();
    match zip.by_name(&entry_name) {
        Ok(mut entry) => {
            entry.read_to_end(&mut bytes)?;
        }
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(error) => return Err(AppError::Archive(error.to_string())),
    }

    if bytes.is_empty() {
        return Ok(None);
    }

    let image = image::load_from_memory(&bytes)?.to_rgba8();
    storage::validate_image_size(image.width(), image.height())?;
    Ok(Some(storage::import_screenshot_image(&image, paths)?))
}

/// Экспорт фильмографии в файл CSV (с BOM для корректного открытия в Excel).
pub fn export_to_csv(films: &[Film], target: &Path) -> AppResult<()> {
    let mut file = fs::File::create(target)?;
    // UTF-8 BOM, чтобы Excel корректно понимал кириллицу
    file.write_all(b"\xEF\xBB\xBF")?;

    let mut writer = csv::WriterBuilder::new()
        .has_headers(true)
        .from_writer(file);

    writer.write_record([
        "ID",
        "Тип медиа",
        "Название",
        "Оригинальное название",
        "Категория",
        "Год",
        "Режиссёр",
        "Жанры",
        "Страна",
        "Длительность (мин)",
        "Оценка",
        "Статус",
        "Теги",
        "IMDb ID",
        "Сезоны (всего)",
        "Просмотрено сезонов",
        "Цитаты",
        "Описание",
        "Мысли",
        "Платформа",
        "Лаунчер",
        "Время в игре (ч)",
        "Статус прохождения",
    ])?;

    for film in films {
        let seasons_str = film
            .watched_seasons
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(",");

        writer.write_record(&[
            film.id.to_string(),
            film.media_type.label().to_owned(),
            film.title.clone(),
            film.original_title.clone(),
            film.category.clone(),
            film.year.map(|y| y.to_string()).unwrap_or_default(),
            film.director.clone(),
            film.genres.clone(),
            film.country.clone(),
            film.runtime_minutes
                .map(|m| m.to_string())
                .unwrap_or_default(),
            film.rating.map(|r| r.to_string()).unwrap_or_default(),
            film.status.label().to_owned(),
            film.tags.clone(),
            film.imdb_id.clone(),
            film.seasons_total.to_string(),
            seasons_str,
            film.quotes.join(" | "),
            film.description.clone(),
            film.thoughts.clone(),
            film.platform.clone().unwrap_or_default(),
            film.launcher.clone().unwrap_or_default(),
            film.playtime_hours
                .map(|h| h.to_string())
                .unwrap_or_default(),
            film.completion_status.clone().unwrap_or_default(),
        ])?;
    }

    writer.flush()?;
    Ok(())
}

/// Экспорт фильмографии и каталога игр в красивый Markdown-файл.
pub fn export_to_markdown(films: &[Film], target: &Path) -> AppResult<()> {
    use std::fmt::Write as _;

    let mut content = String::new();
    let _ = writeln!(content, "# Моя медиатека\n");
    let _ = writeln!(
        content,
        "*Экспортировано записей: {}*\n\n---\n",
        films.len()
    );

    for film in films {
        let year_str = film.year.map(|y| format!(" ({y})")).unwrap_or_default();
        let _ = writeln!(content, "## {}{}\n", film.title, year_str);

        if !film.original_title.is_empty() {
            let _ = writeln!(
                content,
                "**Оригинальное название:** {}\n",
                film.original_title
            );
        }

        let rating_str = film
            .rating
            .map(|r| format!("{r}/10"))
            .unwrap_or_else(|| "без оценки".to_owned());

        let _ = writeln!(
            content,
            "**Тип:** {} | **Категория:** {} | **Статус:** {} | **Оценка:** {}\n",
            film.media_type.label(),
            film.category,
            film.status.label(),
            rating_str
        );

        if film.media_type == MediaType::Game {
            if let Some(ref platform) = film.platform {
                let _ = writeln!(content, "**Платформа:** {}\n", platform);
            }
            if let Some(ref launcher) = film.launcher {
                let _ = writeln!(content, "**Лаунчер:** {}\n", launcher);
            }
            if let Some(hours) = film.playtime_hours {
                let _ = writeln!(content, "**Время в игре:** {hours} ч.\n");
            }
            if let Some(ref status) = film.completion_status {
                let _ = writeln!(content, "**Статус прохождения:** {}\n", status);
            }
        } else {
            if !film.director.is_empty() {
                let _ = writeln!(content, "**Режиссёр:** {}\n", film.director);
            }
            if let Some(runtime) = film.runtime_minutes {
                let _ = writeln!(content, "**Длительность:** {runtime} мин.\n");
            }
            if film.has_seasons() {
                let _ = writeln!(
                    content,
                    "**Сезоны:** просмотрено {} из {}\n",
                    film.watched_season_count(),
                    film.seasons_total
                );
            }
        }
        if !film.genres.is_empty() {
            let _ = writeln!(content, "**Жанры:** {}\n", film.genres);
        }
        if !film.country.is_empty() {
            let _ = writeln!(content, "**Страна:** {}\n", film.country);
        }
        if let Some(runtime) = film.runtime_minutes {
            let _ = writeln!(content, "**Длительность:** {runtime} мин.\n");
        }
        if !film.tags.is_empty() {
            let _ = writeln!(content, "**Теги:** `{}`\n", film.tags);
        }
        if !film.imdb_id.is_empty() {
            let _ = writeln!(
                content,
                "**IMDb:** [{0}](https://www.imdb.com/title/{0}/)\n",
                film.imdb_id
            );
        }
        if film.has_seasons() {
            let _ = writeln!(
                content,
                "**Сезоны:** просмотрено {} из {}\n",
                film.watched_season_count(),
                film.seasons_total
            );
        }
        if !film.description.is_empty() {
            let _ = writeln!(content, "### Описание\n{}\n", film.description);
        }
        if !film.thoughts.is_empty() {
            let _ = writeln!(content, "### Впечатления\n{}\n", film.thoughts);
        }
        if !film.quotes.is_empty() {
            let _ = writeln!(content, "### Цитаты\n");
            for quote in &film.quotes {
                let _ = writeln!(content, "> {}\n", quote);
            }
        }

        let _ = writeln!(content, "---\n");
    }

    fs::write(target, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{FilmStatus, RelationKind};

    fn temp_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "mfl-transfer-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    fn draft(title: &str) -> NewFilm {
        NewFilm {
            title: title.to_owned(),
            year: Some(2021),
            genres: "Фантастика, Драма".to_owned(),
            country: "США".to_owned(),
            runtime_minutes: Some(155),
            imdb_id: format!("tt{}", title.len()),
            tags: "пересмотр".to_owned(),
            ..NewFilm::default()
        }
    }

    #[test]
    fn exports_and_imports_films_and_relations() {
        let dir = temp_dir("roundtrip");
        let paths = AppPaths::from_data_dir(dir.join("data")).unwrap();
        let archive = dir.join("backup.zip");
        let mut source = Database::open(&paths.db_path).unwrap();

        let poster_image = image::RgbaImage::from_pixel(4, 6, image::Rgba([40, 80, 120, 255]));
        let poster = storage::import_poster_image(&poster_image, &paths).unwrap();

        let mut dune = draft("Дюна");
        dune.poster_path = Some(poster.relative_path.clone());

        let first = source.insert_film(&dune).unwrap();
        let second = source.insert_film(&draft("Дюна: Часть вторая")).unwrap();
        source
            .add_relation(second, first, RelationKind::Sequel)
            .unwrap();

        let exported = export_library(&source, &paths, &archive).unwrap();
        assert_eq!(exported.films, 2);
        assert_eq!(exported.relations, 1);
        assert_eq!(exported.posters, 1);

        let import_paths = AppPaths::from_data_dir(dir.join("imported")).unwrap();
        let mut target = Database::open(&import_paths.db_path).unwrap();
        let summary = import_library(&mut target, &import_paths, &archive).unwrap();

        assert_eq!(summary.added, 2);
        assert_eq!(summary.skipped, 0);
        assert_eq!(summary.relations_added, 1);
        assert_eq!(summary.posters_restored, 1);

        let films = target.list_all_films().unwrap();
        assert_eq!(films.len(), 2);
        // Экспорт сохраняет порядок отображения, поэтому проверяем по названию.
        let dune = films
            .iter()
            .find(|film| film.title == "Дюна")
            .expect("импортированная «Дюна»");
        assert_eq!(dune.year, Some(2021));
        assert_eq!(dune.genres, "Фантастика, Драма");
        assert_eq!(dune.tags, "пересмотр");
        assert!(
            import_paths
                .resolve_poster(dune.poster_path.as_deref().unwrap())
                .exists()
        );
        assert!(films.iter().any(|film| film.title == "Дюна: Часть вторая"));

        let relations = target.list_all_relations().unwrap();
        assert_eq!(relations.len(), 1);
        assert_eq!(relations[0].kind, RelationKind::Sequel);

        // Повторный импорт не должен плодить дубликаты.
        let second_import = import_library(&mut target, &import_paths, &archive).unwrap();
        assert_eq!(second_import.added, 0);
        assert_eq!(second_import.skipped, 2);
        assert_eq!(target.list_all_films().unwrap().len(), 2);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn imports_poster_from_zip_with_windows_separators() {
        // В старых архивах путь в library.json мог быть записан с `\`.
        // Имя файла должно извлекаться независимо от ОС, создавшей архив.
        let dir = temp_dir("zip-win-path");
        let paths = AppPaths::from_data_dir(dir.join("data")).unwrap();
        let archive = dir.join("legacy.zip");

        let image = image::RgbaImage::from_pixel(3, 3, image::Rgba([10, 90, 200, 255]));
        let mut png = Vec::new();
        image
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();

        {
            let file = fs::File::create(&archive).unwrap();
            let mut writer = zip::ZipWriter::new(file);
            let options =
                SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
            writer.start_file("posters/legacy.png", options).unwrap();
            writer.write_all(&png).unwrap();
            writer.finish().unwrap();
        }

        let mut zip = ZipArchive::new(fs::File::open(&archive).unwrap()).unwrap();
        let imported = import_poster_from_zip(&mut zip, &paths, r"posters\legacy.png")
            .unwrap()
            .expect("обложка должна восстановиться");

        assert!(imported.relative_path.starts_with("posters/"));
        assert!(imported.absolute_path.exists());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn reads_old_bundle_without_tags_field() {
        // Архивы, созданные до появления тегов, не содержат поле `tags`.
        // Оно должно читаться как пустая строка, а не ломать импорт.
        let json = r#"{
            "version": 1,
            "films": [{
                "id": 7,
                "title": "Дюна",
                "original_title": "Dune",
                "category": "Фильмы",
                "director": "Дени Вильнёв",
                "year": 2021,
                "genres": "Фантастика",
                "country": "США",
                "runtime_minutes": 155,
                "imdb_id": "tt1160419",
                "status": "watched",
                "rating": 9,
                "description": "",
                "thoughts": "",
                "poster_path": null,
                "seasons_total": 0,
                "watched_seasons": []
            }],
            "relations": []
        }"#;

        let bundle: LibraryBundle = serde_json::from_str(json).unwrap();
        assert_eq!(bundle.films.len(), 1);
        assert_eq!(bundle.films[0].tags, "");
        assert!(bundle.films[0].tags_list().is_empty());
    }

    #[test]
    fn detects_duplicates_by_imdb_or_title_and_year() {
        let film = Film {
            id: 1,
            title: "Дюна".to_owned(),
            original_title: "Dune".to_owned(),
            category: "Фильмы".to_owned(),
            director: String::new(),
            year: Some(2021),
            genres: String::new(),
            country: String::new(),
            runtime_minutes: None,
            imdb_id: "tt1160419".to_owned(),
            tags: String::new(),
            status: FilmStatus::Watched,
            rating: None,
            description: String::new(),
            thoughts: String::new(),
            poster_path: None,
            seasons_total: 0,
            watched_seasons: Vec::new(),
            quotes: Vec::new(),
            parent_id: None,
            media_type: MediaType::Movie,
            platform: None,
            launcher: None,
            playtime_hours: None,
            completion_status: None,
        };

        assert!(is_duplicate(
            &film,
            "дюна",
            Some(2021),
            "",
            MediaType::Movie
        ));
        assert!(is_duplicate(
            &film,
            "другое",
            Some(1999),
            "tt1160419",
            MediaType::Movie
        ));
        assert!(!is_duplicate(
            &film,
            "Дюна",
            Some(1984),
            "",
            MediaType::Movie
        ));
        assert!(!is_duplicate(
            &film,
            "дюна",
            Some(2021),
            "",
            MediaType::Game
        ));
    }

    #[test]
    fn exports_to_csv_and_markdown() {
        let dir = temp_dir("csv-md");
        fs::create_dir_all(&dir).unwrap();
        let csv_path = dir.join("test.csv");
        let md_path = dir.join("test.md");

        let film = Film {
            id: 1,
            title: "Матрица".to_owned(),
            original_title: "The Matrix".to_owned(),
            category: "Фильмы".to_owned(),
            director: "Лана Вачовски, Лилли Вачовски".to_owned(),
            year: Some(1999),
            genres: "Фантастика, Боевик".to_owned(),
            country: "США".to_owned(),
            runtime_minutes: Some(136),
            imdb_id: "tt0133093".to_owned(),
            tags: "киберпанк, классика".to_owned(),
            status: FilmStatus::Watched,
            rating: Some(10),
            description: "Мир — это иллюзия.".to_owned(),
            thoughts: "Шедевр.".to_owned(),
            poster_path: None,
            seasons_total: 0,
            watched_seasons: Vec::new(),
            quotes: vec!["Ложки нет.".to_owned()],
            parent_id: None,
            media_type: MediaType::Movie,
            platform: None,
            launcher: None,
            playtime_hours: None,
            completion_status: None,
        };

        export_to_csv(std::slice::from_ref(&film), &csv_path).unwrap();
        assert!(csv_path.exists());
        let csv_content = fs::read(&csv_path).unwrap();
        assert!(csv_content.starts_with(b"\xEF\xBB\xBF")); // BOM check
        let csv_str = String::from_utf8_lossy(&csv_content);
        assert!(csv_str.contains("Матрица"));
        assert!(csv_str.contains("Ложки нет."));
        assert!(csv_str.contains("Тип медиа"));

        export_to_markdown(&[film], &md_path).unwrap();
        assert!(md_path.exists());
        let md_str = fs::read_to_string(&md_path).unwrap();
        assert!(md_str.contains("# Моя медиатека"));
        assert!(md_str.contains("## Матрица (1999)"));
        assert!(md_str.contains("> Ложки нет."));

        let _ = fs::remove_dir_all(&dir);
    }
}
