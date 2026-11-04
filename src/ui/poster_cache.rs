//! Кэш текстур обложек.
//!
//! Файлы обложек после импорта неизменяемы, поэтому достаточно загружать
//! каждую текстуру один раз за сессию. Для карточек изображение уменьшается
//! до миниатюры, чтобы не держать в видеопамяти полноразмерные постеры.

use std::collections::HashMap;
use std::path::Path;

use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};

use crate::storage::AppPaths;

#[derive(Default)]
pub struct PosterCache {
    textures: HashMap<String, Option<TextureHandle>>,
}

impl PosterCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Текстура для пути, сохранённого в БД.
    pub fn texture(
        &mut self,
        ctx: &egui::Context,
        paths: &AppPaths,
        stored_path: &str,
        max_side: u32,
    ) -> Option<TextureHandle> {
        let key = format!("stored:{stored_path}");
        if let Some(cached) = self.textures.get(&key) {
            return cached.clone();
        }

        let path = paths.resolve_poster(stored_path);
        let texture = load_texture(ctx, &key, &path, max_side);
        self.textures.insert(key, texture.clone());
        texture
    }

    /// Текстура для произвольного файла (предпросмотр до сохранения).
    pub fn texture_from_path(
        &mut self,
        ctx: &egui::Context,
        path: &Path,
        max_side: u32,
    ) -> Option<TextureHandle> {
        let key = format!("file:{}", path.display());
        if let Some(cached) = self.textures.get(&key) {
            return cached.clone();
        }

        let texture = load_texture(ctx, &key, path, max_side);
        self.textures.insert(key, texture.clone());
        texture
    }
}

/// Текстура для изображения в памяти, например вставленного из буфера обмена.
pub fn texture_from_rgba(
    ctx: &egui::Context,
    key: &str,
    image: &image::RgbaImage,
    max_side: u32,
) -> Option<TextureHandle> {
    let thumbnail = image::imageops::thumbnail(image, max_side, max_side);
    let (width, height) = thumbnail.dimensions();

    if width == 0 || height == 0 {
        return None;
    }

    let color_image =
        ColorImage::from_rgba_unmultiplied([width as usize, height as usize], thumbnail.as_raw());
    Some(ctx.load_texture(key.to_owned(), color_image, TextureOptions::LINEAR))
}

fn load_texture(
    ctx: &egui::Context,
    key: &str,
    path: &Path,
    max_side: u32,
) -> Option<TextureHandle> {
    let image = image::open(path).ok()?;
    let thumbnail = image.thumbnail(max_side, max_side);
    let rgba = thumbnail.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];

    if size[0] == 0 || size[1] == 0 {
        return None;
    }

    let color_image = ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
    Some(ctx.load_texture(key.to_owned(), color_image, TextureOptions::LINEAR))
}
