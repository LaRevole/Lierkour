mod commands;

use commands::*;
use lierkour_core::{AppPaths, Database};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            // Portable mode: the launcher scripts (`run-portable.bat` on
            // Windows and `run-portable.sh` on Linux) point this variable at
            // the `data` folder next to the executable. Otherwise use the
            // platform's per-user application data directory.
            let data_dir = std::env::var_os(lierkour_core::storage::DATA_DIR_ENV)
                .or_else(|| std::env::var_os(lierkour_core::storage::LEGACY_DATA_DIR_ENV))
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    app.path()
                        .app_data_dir()
                        .expect("Failed to resolve app data directory")
                });

            let paths = AppPaths::from_data_dir(data_dir).expect("Failed to initialize app paths");

            // Seamless migration from legacy directory (v1.0/v1.1 MyFilmLibrary)
            if !paths.db_path.exists() {
                if let Ok(legacy) = AppPaths::discover()
                    && legacy.db_path.exists()
                    && legacy.db_path != paths.db_path
                {
                    let _ = fs::copy(&legacy.db_path, &paths.db_path);
                    if legacy.posters_dir.exists()
                        && let Ok(entries) = fs::read_dir(&legacy.posters_dir)
                    {
                        for entry in entries.flatten() {
                            let dest = paths.posters_dir.join(entry.file_name());
                            if !dest.exists() {
                                let _ = fs::copy(entry.path(), dest);
                            }
                        }
                    }
                    if legacy.screenshots_dir.exists()
                        && let Ok(entries) = fs::read_dir(&legacy.screenshots_dir)
                    {
                        for entry in entries.flatten() {
                            let dest = paths.screenshots_dir.join(entry.file_name());
                            if !dest.exists() {
                                let _ = fs::copy(entry.path(), dest);
                            }
                        }
                    }
                }
            }

            let db = Database::open(&paths.db_path).expect("Failed to open SQLite database");

            app.manage(AppState {
                db: Arc::new(Mutex::new(db)),
                paths,
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_all_films,
            get_filtered_films,
            get_seasons,
            detach_season,
            get_film,
            add_film,
            update_film,
            delete_film,
            get_categories,
            get_stats,
            get_theme,
            set_theme,
            get_setting,
            set_setting,
            import_poster_file,
            import_poster_from_base64,
            resolve_poster_asset,
            fetch_omdb_metadata,
            get_film_relations,
            add_film_relation,
            remove_film_relation,
            get_film_screenshots,
            resolve_screenshot_asset,
            add_film_screenshot,
            delete_film_screenshot,
            export_backup_zip,
            import_backup_zip,
            export_films_csv,
            export_films_markdown
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
