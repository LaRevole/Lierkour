//! Работа с локальными путями и файлами обложек.
//!
//! Данные хранятся в `%APPDATA%/<AppName>` на Windows и
//! `~/.local/share/<AppName>` на Linux. Если системный каталог недоступен,
//! используется папка `data/` рядом с исполняемым файлом.
//!
//! В БД записывается относительный путь (`posters/<file>`), а абсолютный
//! вычисляется через [`AppPaths::resolve_poster`]. Это позволяет переносить
//! папку с данными между ОС и пользователями.

use std::borrow::Cow;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use image::ImageFormat;

use crate::error::{AppError, AppResult};

const APP_DIR_NAME: &str = "MyFilmLibrary";
pub const DATA_DIR_ENV: &str = "MY_FILM_LIBRARY_DATA_DIR";
const MAX_IMAGE_DIMENSION: u32 = 8_000;
pub const POSTER_THUMB_WIDTH: u32 = 300;
pub const POSTER_THUMB_HEIGHT: u32 = 450;

/// Все пути, которые нужны приложению во время работы.
#[derive(Debug, Clone)]
pub struct AppPaths {
    pub data_dir: PathBuf,
    pub db_path: PathBuf,
    pub posters_dir: PathBuf,
    pub screenshots_dir: PathBuf,
}

impl AppPaths {
    /// Определяет каталог данных и создаёт необходимые папки.
    pub fn discover() -> AppResult<Self> {
        // Явный каталог полезен для переносимой сборки, тестов и отладки.
        if let Some(custom_dir) = std::env::var_os(DATA_DIR_ENV).filter(|value| !value.is_empty()) {
            return Self::from_data_dir(PathBuf::from(custom_dir));
        }

        let data_dir = dirs::data_dir()
            .map(|dir| dir.join(APP_DIR_NAME))
            .or_else(exe_side_data_dir)
            .ok_or(AppError::DataDirUnavailable)?;

        Self::from_data_dir(data_dir)
    }

    /// Полезно для тестов и для переносимой сборки с явно заданным каталогом.
    pub fn from_data_dir(data_dir: PathBuf) -> AppResult<Self> {
        let posters_dir = data_dir.join("posters");
        fs::create_dir_all(&posters_dir)?;
        let screenshots_dir = data_dir.join("screenshots");
        fs::create_dir_all(&screenshots_dir)?;

        Ok(Self {
            db_path: data_dir.join("library.db"),
            data_dir,
            posters_dir,
            screenshots_dir,
        })
    }

    /// Преобразует путь из БД в абсолютный путь на текущей машине.
    ///
    /// Понимает и `/`, и `\`, чтобы библиотека, созданная в Windows,
    /// корректно открывалась в Linux (и наоборот). Абсолютный путь Windows
    /// (`C:/...`), открытый в Linux, сводится к имени файла в `posters/`:
    /// сама папка данных уже перенесена вместе с библиотекой.
    pub fn resolve_poster(&self, stored_path: &str) -> PathBuf {
        let normalized = stored_path.replace('\\', "/");
        let path = Path::new(&normalized);

        if path.is_absolute() {
            return path.to_path_buf();
        }

        if is_windows_drive_path(&normalized)
            && let Some(file_name) = poster_file_name(&normalized)
        {
            return self.posters_dir.join(file_name);
        }

        self.data_dir.join(path)
    }

    /// Путь к миниатюре обложки (если она существует), иначе оригинал.
    pub fn resolve_poster_thumbnail(&self, stored_path: &str) -> PathBuf {
        if let Some(file_name) = poster_file_name(stored_path) {
            let thumb = self.posters_dir.join(format!("thumb_{file_name}"));
            if thumb.is_file() {
                return thumb;
            }
        }
        self.resolve_poster(stored_path)
    }

    /// Преобразует путь к кадру в абсолютный путь.
    pub fn resolve_screenshot(&self, stored_path: &str) -> PathBuf {
        let normalized = stored_path.replace('\\', "/");
        let path = Path::new(&normalized);

        if path.is_absolute() {
            return path.to_path_buf();
        }

        self.data_dir.join(path)
    }
}

/// Имя файла обложки из пути, сохранённого в БД или ZIP-архиве.
///
/// Работает с разделителями обеих ОС, поэтому архив, созданный
/// в Windows, можно импортировать в Linux и наоборот.
pub fn poster_file_name(stored_path: &str) -> Option<&str> {
    let trimmed = stored_path.trim_end_matches(['/', '\\']);
    let name = trimmed.rsplit(['/', '\\']).next()?;
    if name.is_empty() || name == "." || name == ".." {
        None
    } else {
        Some(name)
    }
}

/// Путь вида `C:/Users/...` (после нормализации разделителей).
fn is_windows_drive_path(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'/' | b'\\')
}

/// Превращает текст из буфера обмена в путь к файлу.
///
/// Windows Explorer копирует обычный путь (иногда в кавычках), а файловые
/// менеджеры Linux — список `file://` URI (иногда с заголовком
/// `x-special/gnome-copied-files` и строкой `copy`). Предпочитаем первую
/// URI-строку, иначе берём первую содержательную строку, и декодируем
/// percent-последовательности вроде `%20`.
pub fn path_from_clipboard_text(text: &str) -> Option<PathBuf> {
    let mut first_line = None;
    let mut uri_line = None;

    for raw in text.lines() {
        let line = raw.trim().trim_matches('"').trim();
        if line.is_empty() || is_clipboard_metadata(line) {
            continue;
        }
        first_line.get_or_insert(line);
        if line.starts_with("file://") {
            uri_line = Some(line);
            break;
        }
    }

    let line = uri_line.or(first_line)?;
    let raw = if let Some(rest) = line.strip_prefix("file://") {
        let rest = rest.strip_prefix("localhost").unwrap_or(rest);
        Cow::Owned(percent_decode(rest))
    } else {
        Cow::Borrowed(line)
    };

    let raw = strip_windows_uri_slash(&raw);
    if raw.is_empty() {
        None
    } else {
        Some(PathBuf::from(raw))
    }
}

/// Служебные строки, которые файловые менеджеры добавляют к списку URI.
fn is_clipboard_metadata(line: &str) -> bool {
    matches!(line, "copy" | "cut" | "x-special/gnome-copied-files")
}

/// Убирает ведущий `/` у `file:///C:/...`, чтобы на Windows получился
/// нормальный абсолютный путь с буквой диска.
fn strip_windows_uri_slash(path: &str) -> &str {
    if let Some(rest) = path.strip_prefix('/') {
        let bytes = rest.as_bytes();
        if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
            return rest;
        }
    }
    path
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let (Some(high), Some(low)) =
                (hex_value(bytes[index + 1]), hex_value(bytes[index + 2]))
        {
            decoded.push((high << 4) | low);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }

    String::from_utf8_lossy(&decoded).into_owned()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn exe_side_data_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("data")))
}

/// Результат импорта обложки.
#[derive(Debug, Clone)]
pub struct ImportedPoster {
    /// Путь, который сохраняется в SQLite (`posters/<name>.<ext>`).
    pub relative_path: String,
    /// Абсолютный путь к созданной копии (нужен для отката при ошибке БД).
    pub absolute_path: PathBuf,
}

/// Проверяет, что файл действительно является изображением,
/// и возвращает определённый формат.
pub fn validate_poster(source: &Path) -> AppResult<ImageFormat> {
    if !source.is_file() {
        return Err(AppError::PosterNotFound(source.to_path_buf()));
    }

    let reader = image::ImageReader::open(source)?.with_guessed_format()?;
    let format = reader
        .format()
        .ok_or_else(|| AppError::UnsupportedImageFormat(source.to_path_buf()))?;

    let (width, height) = reader.into_dimensions()?;
    if width == 0 || height == 0 {
        return Err(AppError::UnsupportedImageFormat(source.to_path_buf()));
    }
    validate_image_size(width, height)?;

    Ok(format)
}

/// Проверяет размеры уже декодированного изображения (например, из буфера обмена).
pub fn validate_image_size(width: u32, height: u32) -> AppResult<()> {
    if width == 0 || height == 0 {
        return Err(AppError::EmptyImage);
    }
    if width > MAX_IMAGE_DIMENSION || height > MAX_IMAGE_DIMENSION {
        return Err(AppError::ImageTooLarge(width, height, MAX_IMAGE_DIMENSION));
    }
    Ok(())
}

/// Валидирует изображение и копирует его в локальную папку обложек.
///
/// Имя файла генерируется заново, поэтому оригинал можно удалить сразу
/// после сохранения. Расширение берётся из реального формата, а не из
/// имени исходного файла.
pub fn import_poster(source: &Path, paths: &AppPaths) -> AppResult<ImportedPoster> {
    let format = validate_poster(source)?;

    let extension = format.extensions_str().first().copied().unwrap_or("img");

    let readable_stem: String = source
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("poster")
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '-')
        .take(40)
        .collect();
    let stem = if readable_stem.is_empty() {
        "poster".to_owned()
    } else {
        readable_stem
    };

    let file_name = format!("{stem}-{}.{extension}", unique_suffix());
    let destination = paths.posters_dir.join(&file_name);

    if let Err(error) = fs::copy(source, &destination) {
        // Не оставляем частично скопированный файл.
        let _ = fs::remove_file(&destination);
        return Err(error.into());
    }

    // Генерируем уменьшенную копию (300x450) для сетки
    if let Ok(img) = image::open(&destination) {
        let thumb = img.thumbnail(POSTER_THUMB_WIDTH, POSTER_THUMB_HEIGHT);
        let thumb_dest = paths.posters_dir.join(format!("thumb_{file_name}"));
        let _ = thumb.save(&thumb_dest);
    }

    Ok(imported_poster(destination))
}

/// Сохраняет изображение, полученное из буфера обмена, как PNG-обложку.
///
/// В отличие от [`import_poster`] исходного файла нет: изображение уже
/// декодировано в память, поэтому сразу пишем его в папку приложения.
pub fn import_poster_image(
    image: &image::RgbaImage,
    paths: &AppPaths,
) -> AppResult<ImportedPoster> {
    let (width, height) = image.dimensions();
    validate_image_size(width, height)?;

    let suffix = unique_suffix();
    let file_name = format!("poster-{suffix}.png");
    let destination = paths.posters_dir.join(&file_name);

    if let Err(error) = image::save_buffer_with_format(
        &destination,
        image.as_raw(),
        width,
        height,
        image::ColorType::Rgba8,
        ImageFormat::Png,
    ) {
        let _ = fs::remove_file(&destination);
        return Err(error.into());
    }

    // Генерируем уменьшенную копию (300x450) для сетки
    let dynamic = image::DynamicImage::ImageRgba8(image.clone());
    let thumb = dynamic.thumbnail(POSTER_THUMB_WIDTH, POSTER_THUMB_HEIGHT);
    let thumb_dest = paths.posters_dir.join(format!("thumb_{file_name}"));
    let _ = thumb.save(&thumb_dest);

    Ok(imported_poster(destination))
}

pub fn import_poster_bytes(bytes: &[u8], paths: &AppPaths) -> AppResult<ImportedPoster> {
    let img = image::load_from_memory(bytes)?.to_rgba8();
    import_poster_image(&img, paths)
}

fn imported_poster(destination: PathBuf) -> ImportedPoster {
    let relative_path = format!(
        "posters/{}",
        destination
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
    );

    ImportedPoster {
        relative_path,
        absolute_path: destination,
    }
}

/// Удаляет копию обложки, но только если она лежит внутри папки приложения.
///
/// Ошибка удаления не должна ломать сохранение фильма, поэтому вызывающий
/// код обычно лишь показывает предупреждение.
pub fn delete_poster(paths: &AppPaths, stored_path: &str) -> AppResult<()> {
    let path = paths.resolve_poster(stored_path);
    if path.parent() != Some(paths.posters_dir.as_path()) {
        return Err(AppError::InvalidPosterPath(path));
    }

    if let Some(name) = poster_file_name(stored_path) {
        let thumb = paths.posters_dir.join(format!("thumb_{name}"));
        let _ = fs::remove_file(thumb);
    }

    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

/// Импортирует файл скриншота/кадра в хранилище приложения.
pub fn import_screenshot(source: &Path, paths: &AppPaths) -> AppResult<ImportedPoster> {
    let format = validate_poster(source)?;
    let extension = format.extensions_str().first().copied().unwrap_or("png");
    let file_name = format!("screenshot-{}.{extension}", unique_suffix());
    let destination = paths.screenshots_dir.join(&file_name);

    if let Err(error) = fs::copy(source, &destination) {
        let _ = fs::remove_file(&destination);
        return Err(error.into());
    }

    Ok(ImportedPoster {
        relative_path: format!("screenshots/{file_name}"),
        absolute_path: destination,
    })
}

/// Сохраняет изображение скриншота/кадра из буфера обмена.
pub fn import_screenshot_image(
    image: &image::RgbaImage,
    paths: &AppPaths,
) -> AppResult<ImportedPoster> {
    let (width, height) = image.dimensions();
    validate_image_size(width, height)?;

    let file_name = format!("screenshot-{}.png", unique_suffix());
    let destination = paths.screenshots_dir.join(&file_name);

    if let Err(error) = image::save_buffer_with_format(
        &destination,
        image.as_raw(),
        width,
        height,
        image::ColorType::Rgba8,
        ImageFormat::Png,
    ) {
        let _ = fs::remove_file(&destination);
        return Err(error.into());
    }

    Ok(ImportedPoster {
        relative_path: format!("screenshots/{file_name}"),
        absolute_path: destination,
    })
}

/// Удаляет файл кадра/скриншота.
pub fn delete_screenshot_file(paths: &AppPaths, stored_path: &str) -> AppResult<()> {
    let path = paths.resolve_screenshot(stored_path);
    if path.parent() != Some(paths.screenshots_dir.as_path()) {
        return Err(AppError::InvalidPosterPath(path));
    }

    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn unique_suffix() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);

    format!("{nanos:x}{counter:x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_missing_and_non_image_files() {
        let dir = std::env::temp_dir().join(format!("mfl-poster-test-{}", unique_suffix()));
        fs::create_dir_all(&dir).unwrap();

        let missing = dir.join("missing.png");
        assert!(matches!(
            validate_poster(&missing),
            Err(AppError::PosterNotFound(_))
        ));

        let text = dir.join("not-an-image.jpg");
        fs::write(&text, b"definitely not an image").unwrap();
        assert!(validate_poster(&text).is_err());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_poster_supports_relative_and_absolute_paths() {
        let dir = std::env::temp_dir().join(format!("mfl-paths-test-{}", unique_suffix()));
        let paths = AppPaths::from_data_dir(dir.clone()).unwrap();

        assert_eq!(
            paths.resolve_poster("posters/a.jpg"),
            dir.join("posters").join("a.jpg")
        );

        let absolute = dir.join("elsewhere.jpg");
        assert_eq!(paths.resolve_poster(absolute.to_str().unwrap()), absolute);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_poster_accepts_backslash_separators() {
        let dir = std::env::temp_dir().join(format!("mfl-paths-win-test-{}", unique_suffix()));
        let paths = AppPaths::from_data_dir(dir.clone()).unwrap();

        assert_eq!(
            paths.resolve_poster(r"posters\a.jpg"),
            dir.join("posters").join("a.jpg")
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(not(windows))]
    #[test]
    fn resolve_poster_maps_legacy_windows_path_into_posters_dir() {
        let dir = std::env::temp_dir().join(format!("mfl-paths-legacy-test-{}", unique_suffix()));
        let paths = AppPaths::from_data_dir(dir.clone()).unwrap();

        assert_eq!(
            paths.resolve_poster(r"C:\Users\old\AppData\Roaming\MyFilmLibrary\posters\a.jpg"),
            dir.join("posters").join("a.jpg")
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn poster_file_name_handles_both_separators() {
        assert_eq!(poster_file_name("posters/a.jpg"), Some("a.jpg"));
        assert_eq!(poster_file_name(r"posters\a.jpg"), Some("a.jpg"));
        assert_eq!(poster_file_name(r"C:\data\posters\a.jpg"), Some("a.jpg"));
        assert_eq!(
            poster_file_name("/data/posters/poster-1.png"),
            Some("poster-1.png")
        );
        assert_eq!(poster_file_name("a.jpg/"), Some("a.jpg"));
        assert_eq!(poster_file_name(""), None);
        assert_eq!(poster_file_name(".."), None);
    }

    #[test]
    fn clipboard_text_accepts_quoted_paths() {
        assert_eq!(
            path_from_clipboard_text("  \"pictures/poster one.jpg\"\n"),
            Some(PathBuf::from("pictures/poster one.jpg"))
        );
        assert_eq!(
            path_from_clipboard_text("pictures/poster.jpg"),
            Some(PathBuf::from("pictures/poster.jpg"))
        );
        assert_eq!(path_from_clipboard_text("   \n\t\n"), None);
    }

    #[test]
    fn clipboard_text_decodes_file_uris() {
        assert_eq!(
            path_from_clipboard_text("file:///home/me/My%20Poster.jpg"),
            Some(PathBuf::from("/home/me/My Poster.jpg"))
        );
        assert_eq!(
            path_from_clipboard_text("file://localhost/home/me/poster.jpg"),
            Some(PathBuf::from("/home/me/poster.jpg"))
        );
        assert_eq!(
            path_from_clipboard_text("file:///tmp/%D0%BF%D0%BE%D1%81%D1%82%D0%B5%D1%80.jpg"),
            Some(PathBuf::from("/tmp/постер.jpg"))
        );
    }

    #[test]
    fn clipboard_text_takes_first_line_of_uri_list() {
        let text = "file:///tmp/first%20poster.jpg\nfile:///tmp/second.jpg\n";
        assert_eq!(
            path_from_clipboard_text(text),
            Some(PathBuf::from("/tmp/first poster.jpg"))
        );
    }

    #[test]
    fn clipboard_text_skips_gnome_copied_files_header() {
        let text = "x-special/gnome-copied-files\ncopy\nfile:///home/me/poster.jpg\n";
        assert_eq!(
            path_from_clipboard_text(text),
            Some(PathBuf::from("/home/me/poster.jpg"))
        );
    }

    #[test]
    fn clipboard_text_keeps_windows_drive_paths() {
        assert_eq!(
            path_from_clipboard_text("file:///C:/Users/me/Poster.jpg"),
            Some(PathBuf::from("C:/Users/me/Poster.jpg"))
        );
        assert_eq!(
            strip_windows_uri_slash("/home/me/poster.jpg"),
            "/home/me/poster.jpg"
        );
    }

    #[test]
    fn imports_and_deletes_managed_poster() {
        let dir = std::env::temp_dir().join(format!("mfl-import-test-{}", unique_suffix()));
        let paths = AppPaths::from_data_dir(dir.clone()).unwrap();

        let source = dir.join("source.png");
        image::RgbaImage::from_pixel(4, 4, image::Rgba([20, 40, 60, 255]))
            .save(&source)
            .unwrap();

        let imported = import_poster(&source, &paths).unwrap();
        assert!(imported.relative_path.starts_with("posters/"));
        assert!(imported.absolute_path.exists());
        assert_eq!(
            imported.absolute_path.parent(),
            Some(paths.posters_dir.as_path())
        );

        delete_poster(&paths, &imported.relative_path).unwrap();
        assert!(!imported.absolute_path.exists());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn imports_clipboard_image_as_png() {
        let dir = std::env::temp_dir().join(format!("mfl-clipboard-test-{}", unique_suffix()));
        let paths = AppPaths::from_data_dir(dir.clone()).unwrap();

        let image = image::RgbaImage::from_pixel(3, 2, image::Rgba([10, 20, 30, 255]));
        let imported = import_poster_image(&image, &paths).unwrap();

        assert!(imported.relative_path.starts_with("posters/"));
        assert!(imported.relative_path.ends_with(".png"));
        assert!(imported.absolute_path.exists());

        let decoded = image::open(&imported.absolute_path).unwrap().to_rgba8();
        assert_eq!(decoded.dimensions(), (3, 2));
        assert_eq!(decoded.get_pixel(0, 0).0, [10, 20, 30, 255]);

        delete_poster(&paths, &imported.relative_path).unwrap();
        assert!(!imported.absolute_path.exists());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_empty_image_dimensions() {
        assert!(matches!(
            validate_image_size(0, 10),
            Err(AppError::EmptyImage)
        ));
        assert!(matches!(
            validate_image_size(MAX_IMAGE_DIMENSION + 1, 10),
            Err(AppError::ImageTooLarge(_, _, _))
        ));
        assert!(validate_image_size(10, 10).is_ok());
    }

    #[test]
    fn refuses_to_delete_files_outside_posters_dir() {
        let dir = std::env::temp_dir().join(format!("mfl-guard-test-{}", unique_suffix()));
        let paths = AppPaths::from_data_dir(dir.clone()).unwrap();

        let outside = dir.join("library.db");
        fs::write(&outside, b"do not delete").unwrap();

        assert!(matches!(
            delete_poster(&paths, outside.to_str().unwrap()),
            Err(AppError::InvalidPosterPath(_))
        ));
        assert!(outside.exists());

        let _ = fs::remove_dir_all(&dir);
    }
}
