//! Кэш текстур обложек с асинхронной фоновой загрузкой.
//!
//! Изображения декодируются и масштабируются в отдельном фоновом потоке,
//! чтобы UI-поток никогда не блокировался и вертикальная прокрутка
//! оставалась идеально плавной (60-144 FPS).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};

use crate::storage::AppPaths;

struct LoadResult {
    key: String,
    image: Option<ColorImage>,
}

pub struct PosterCache {
    textures: HashMap<String, Option<TextureHandle>>,
    pending: HashSet<String>,
    req_tx: Sender<(String, PathBuf, u32)>,
    res_rx: Receiver<LoadResult>,
}

impl Default for PosterCache {
    fn default() -> Self {
        Self::new()
    }
}

impl PosterCache {
    pub fn new() -> Self {
        let (req_tx, req_rx) = mpsc::channel::<(String, PathBuf, u32)>();
        let (res_tx, res_rx) = mpsc::channel::<LoadResult>();

        thread::spawn(move || {
            while let Ok((key, path, max_side)) = req_rx.recv() {
                let image = load_color_image(&path, max_side);
                let _ = res_tx.send(LoadResult { key, image });
            }
        });

        Self {
            textures: HashMap::new(),
            pending: HashSet::new(),
            req_tx,
            res_rx,
        }
    }

    /// Проверяет поступление декодированных изображений из фонового потока.
    pub fn poll(&mut self, ctx: &egui::Context) {
        let mut updated = false;
        while let Ok(res) = self.res_rx.try_recv() {
            self.pending.remove(&res.key);
            let handle = res
                .image
                .map(|img| ctx.load_texture(res.key.clone(), img, TextureOptions::LINEAR));
            self.textures.insert(res.key, handle);
            updated = true;
        }
        if updated {
            ctx.request_repaint();
        }
    }

    /// Текстура для пути, сохранённого в БД.
    pub fn texture(
        &mut self,
        ctx: &egui::Context,
        paths: &AppPaths,
        stored_path: &str,
        max_side: u32,
    ) -> Option<TextureHandle> {
        self.poll(ctx);
        let key = format!("stored:{stored_path}");
        if let Some(cached) = self.textures.get(&key) {
            return cached.clone();
        }

        if !self.pending.contains(&key) {
            self.pending.insert(key.clone());
            let path = paths.resolve_poster(stored_path);
            let _ = self.req_tx.send((key, path, max_side));
        }

        None
    }

    /// Текстура для миниатюры обложки в сетке библиотеки.
    pub fn thumbnail_texture(
        &mut self,
        ctx: &egui::Context,
        paths: &AppPaths,
        stored_path: &str,
        max_side: u32,
    ) -> Option<TextureHandle> {
        self.poll(ctx);
        let key = format!("thumb:{stored_path}");
        if let Some(cached) = self.textures.get(&key) {
            return cached.clone();
        }

        if !self.pending.contains(&key) {
            self.pending.insert(key.clone());
            let path = paths.resolve_poster_thumbnail(stored_path);
            let _ = self.req_tx.send((key, path, max_side));
        }

        None
    }

    /// Текстура для кадра/скриншота.
    pub fn screenshot_texture(
        &mut self,
        ctx: &egui::Context,
        paths: &AppPaths,
        stored_path: &str,
        max_side: u32,
    ) -> Option<TextureHandle> {
        self.poll(ctx);
        let key = format!("shot:{stored_path}");
        if let Some(cached) = self.textures.get(&key) {
            return cached.clone();
        }

        if !self.pending.contains(&key) {
            self.pending.insert(key.clone());
            let path = paths.resolve_screenshot(stored_path);
            let _ = self.req_tx.send((key, path, max_side));
        }

        None
    }

    /// Текстура для произвольного файла (предпросмотр до сохранения).
    pub fn texture_from_path(
        &mut self,
        ctx: &egui::Context,
        path: &Path,
        max_side: u32,
    ) -> Option<TextureHandle> {
        self.poll(ctx);
        let key = format!("file:{}", path.display());
        if let Some(cached) = self.textures.get(&key) {
            return cached.clone();
        }

        if !self.pending.contains(&key) {
            self.pending.insert(key.clone());
            let _ = self.req_tx.send((key, path.to_path_buf(), max_side));
        }

        None
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

fn load_color_image(path: &Path, max_side: u32) -> Option<ColorImage> {
    let image = image::open(path).ok()?;
    let thumbnail = image.thumbnail(max_side, max_side);
    let rgba = thumbnail.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];

    if size[0] == 0 || size[1] == 0 {
        return None;
    }

    Some(ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()))
}
