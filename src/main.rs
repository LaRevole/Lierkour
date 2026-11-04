#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
#![forbid(unsafe_code)]

//! Точка входа приложения «Моя Фильмография».
//!
//! Запуск: `cargo run --release`
//!
//! Ошибки открытия каталога данных возвращаются из `main` (без `panic!`).
//! Ошибки базы данных, которые возникают уже после старта, показываются
//! внутри окна приложения.

mod app;
mod db;
mod error;
mod metadata;
mod models;
mod nav;
mod storage;
mod transfer;
mod ui;

use crate::app::FilmApp;
use crate::storage::AppPaths;

fn main() -> anyhow::Result<()> {
    let paths = match AppPaths::discover() {
        Ok(paths) => paths,
        Err(error) => {
            let message = format!("Не удалось подготовить каталог данных приложения:\n{error}");
            report_fatal_error(&message);
            return Err(anyhow::anyhow!(message));
        }
    };

    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Моя Фильмография")
            .with_app_id("my-film-library")
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([860.0, 560.0]),
        ..Default::default()
    };

    if let Err(error) = eframe::run_native(
        "Моя Фильмография",
        native_options,
        Box::new(move |cc| Ok(Box::new(FilmApp::new(cc, paths)))),
    ) {
        let message = format!("Не удалось запустить графическое приложение:\n{error}");
        report_fatal_error(&message);
        return Err(anyhow::anyhow!(message));
    }

    Ok(())
}

/// Показывает нативную ошибку даже в release-сборке, где консоль скрыта.
fn report_fatal_error(message: &str) {
    eprintln!("{message}");
    let _ = rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title("Моя Фильмография")
        .set_description(message)
        .show();
}
