//! Типы ошибок приложения.
//!
//! Все ошибки, которые доходят до UI, конвертируются в [`AppError`]
//! и показываются пользователю в виде сообщения, а не через `panic!`.

use std::path::PathBuf;

use thiserror::Error;

use crate::models::FilmValidationError;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Ошибка базы данных: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Ошибка файловой системы: {0}")]
    Io(#[from] std::io::Error),

    #[error("Не удалось прочитать изображение: {0}")]
    Image(#[from] image::ImageError),

    #[error("Не удалось определить каталог для данных приложения")]
    DataDirUnavailable,

    #[error("База данных недоступна")]
    DatabaseUnavailable,

    #[error("{0}")]
    Validation(#[from] FilmValidationError),

    #[error("Фильм не найден (id = {0})")]
    FilmNotFound(i64),

    #[error("Нельзя связать фильм с самим собой")]
    SelfRelation,

    #[error("Связь не найдена (id = {0})")]
    RelationNotFound(i64),

    #[error("Ошибка ZIP-архива: {0}")]
    Archive(String),

    #[error("{0}")]
    Transfer(String),

    #[error("Ошибка CSV: {0}")]
    Csv(#[from] csv::Error),

    #[error("Файл обложки не найден: {}", .0.display())]
    PosterNotFound(PathBuf),

    #[error("Неподдерживаемый формат изображения: {}", .0.display())]
    UnsupportedImageFormat(PathBuf),

    #[error("Изображение слишком большое: {0}×{1} px (максимум {2}×{2})")]
    ImageTooLarge(u32, u32, u32),

    #[error("Некорректный путь к обложке: {}", .0.display())]
    InvalidPosterPath(PathBuf),

    #[error("Изображение пустое (0×0)")]
    EmptyImage,

    #[error("Ошибка буфера обмена: {0}")]
    Clipboard(#[from] arboard::Error),

    #[error("В буфере обмена нет изображения. Скопируйте картинку и попробуйте снова.")]
    ClipboardNoImage,

    #[error("Не удалось прочитать изображение из буфера обмена")]
    ClipboardInvalidImage,
}
