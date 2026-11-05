use myfilmlibrary_core::metadata::MovieMetadata;
use myfilmlibrary_core::transfer::{ExportSummary, ImportSummary};
use myfilmlibrary_core::{
    AppPaths, AppTheme, DashboardStats, Database, Film, FilmFilter, FilmScreenshot, NewFilm,
    RelatedFilm, RelationKind,
};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tauri::State;

pub struct AppState {
    pub db: Arc<Mutex<Database>>,
    pub paths: AppPaths,
}

// --------------------------- FILMS & CRUD ---------------------------

#[tauri::command]
pub async fn get_all_films(state: State<'_, AppState>) -> Result<Vec<Film>, String> {
    let db = Arc::clone(&state.db);
    tokio::task::spawn_blocking(move || {
        let db_lock = db.lock().map_err(|e| e.to_string())?;
        db_lock.list_all_films().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("Execution error: {e}"))?
}

#[tauri::command]
pub async fn get_filtered_films(
    filter: FilmFilter,
    state: State<'_, AppState>,
) -> Result<Vec<Film>, String> {
    let db = Arc::clone(&state.db);
    tokio::task::spawn_blocking(move || {
        let db_lock = db.lock().map_err(|e| e.to_string())?;
        db_lock.list_root_films(&filter).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("Execution error: {e}"))?
}

#[tauri::command]
pub async fn get_seasons(parent_id: i64, state: State<'_, AppState>) -> Result<Vec<Film>, String> {
    let db = Arc::clone(&state.db);
    tokio::task::spawn_blocking(move || {
        let db_lock = db.lock().map_err(|e| e.to_string())?;
        db_lock.get_seasons(parent_id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("Execution error: {e}"))?
}

#[tauri::command]
pub async fn detach_season(season_id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let db = Arc::clone(&state.db);
    tokio::task::spawn_blocking(move || {
        let mut db_lock = db.lock().map_err(|e| e.to_string())?;
        db_lock.detach_season(season_id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("Execution error: {e}"))?
}

#[tauri::command]
pub async fn get_film(id: i64, state: State<'_, AppState>) -> Result<Film, String> {
    let db = Arc::clone(&state.db);
    tokio::task::spawn_blocking(move || {
        let db_lock = db.lock().map_err(|e| e.to_string())?;
        db_lock.get_film(id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("Execution error: {e}"))?
}

#[tauri::command]
pub async fn add_film(film: NewFilm, state: State<'_, AppState>) -> Result<Film, String> {
    let db = Arc::clone(&state.db);
    tokio::task::spawn_blocking(move || {
        let mut db_lock = db.lock().map_err(|e| e.to_string())?;
        let new_id = db_lock.insert_film(&film).map_err(|e| e.to_string())?;
        db_lock.get_film(new_id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("Execution error: {e}"))?
}

#[tauri::command]
pub async fn update_film(
    id: i64,
    film: NewFilm,
    state: State<'_, AppState>,
) -> Result<Film, String> {
    let db = Arc::clone(&state.db);
    tokio::task::spawn_blocking(move || {
        let mut db_lock = db.lock().map_err(|e| e.to_string())?;
        db_lock.update_film(id, &film).map_err(|e| e.to_string())?;
        db_lock.get_film(id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("Execution error: {e}"))?
}

#[tauri::command]
pub async fn delete_film(id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let db = Arc::clone(&state.db);
    let paths = state.paths.clone();
    tokio::task::spawn_blocking(move || {
        let mut db_lock = db.lock().map_err(|e| e.to_string())?;
        let poster_to_delete = db_lock.delete_film(id).map_err(|e| e.to_string())?;
        if let Some(poster) = poster_to_delete {
            let _ = myfilmlibrary_core::storage::delete_poster(&paths, &poster);
        }
        Ok(())
    })
    .await
    .map_err(|e| format!("Execution error: {e}"))?
}

#[tauri::command]
pub async fn get_categories(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let db = Arc::clone(&state.db);
    tokio::task::spawn_blocking(move || {
        let db_lock = db.lock().map_err(|e| e.to_string())?;
        db_lock.list_categories().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("Execution error: {e}"))?
}

#[tauri::command]
pub async fn get_stats(state: State<'_, AppState>) -> Result<DashboardStats, String> {
    let db = Arc::clone(&state.db);
    tokio::task::spawn_blocking(move || {
        let db_lock = db.lock().map_err(|e| e.to_string())?;
        db_lock.get_dashboard_stats().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("Execution error: {e}"))?
}

// --------------------------- THEME & SETTINGS ---------------------------

#[tauri::command]
pub async fn get_theme(state: State<'_, AppState>) -> Result<String, String> {
    let db = Arc::clone(&state.db);
    tokio::task::spawn_blocking(move || {
        let db_lock = db.lock().map_err(|e| e.to_string())?;
        let theme = db_lock.get_theme().map_err(|e| e.to_string())?;
        Ok(theme.as_str().to_string())
    })
    .await
    .map_err(|e| format!("Execution error: {e}"))?
}

#[tauri::command]
pub async fn set_theme(theme_str: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = Arc::clone(&state.db);
    tokio::task::spawn_blocking(move || {
        let theme = AppTheme::from_str_opt(&theme_str).unwrap_or(AppTheme::Dark);
        let mut db_lock = db.lock().map_err(|e| e.to_string())?;
        db_lock.set_theme(theme).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("Execution error: {e}"))?
}

#[tauri::command]
pub async fn get_setting(
    key: String,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let db = Arc::clone(&state.db);
    tokio::task::spawn_blocking(move || {
        let db_lock = db.lock().map_err(|e| e.to_string())?;
        db_lock.get_setting(&key).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("Execution error: {e}"))?
}

#[tauri::command]
pub async fn set_setting(
    key: String,
    value: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = Arc::clone(&state.db);
    tokio::task::spawn_blocking(move || {
        let mut db_lock = db.lock().map_err(|e| e.to_string())?;
        db_lock.set_setting(&key, &value).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("Execution error: {e}"))?
}

// --------------------------- IMAGES & POSTERS ---------------------------

#[tauri::command]
pub async fn import_poster_file(
    source_path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let paths = state.paths.clone();
    tokio::task::spawn_blocking(move || {
        let p = Path::new(&source_path);
        let imported =
            myfilmlibrary_core::storage::import_poster(p, &paths).map_err(|e| e.to_string())?;
        Ok(imported.relative_path)
    })
    .await
    .map_err(|e| format!("Execution error: {e}"))?
}

#[tauri::command]
pub async fn resolve_poster_asset(
    stored_path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let paths = state.paths.clone();
    let abs_path = paths.resolve_poster(&stored_path);
    if !abs_path.exists() {
        return Err("Poster file does not exist".to_string());
    }
    // Читаем байты постера и возвращаем data:image/png;base64,... для мгновенного отображения в WebView
    let bytes = std::fs::read(&abs_path).map_err(|e| e.to_string())?;
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:image/jpeg;base64,{b64}"))
}

#[tauri::command]
pub async fn import_poster_from_base64(
    base64_data: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let paths = state.paths.clone();
    tokio::task::spawn_blocking(move || {
        use base64::Engine;
        // Отрезаем data:image/...;base64, если передано с data url
        let clean_base64 = if let Some(idx) = base64_data.find(",") {
            &base64_data[idx + 1..]
        } else {
            &base64_data
        };

        let bytes = base64::engine::general_purpose::STANDARD
            .decode(clean_base64)
            .map_err(|e| format!("Base64 decode error: {e}"))?;

        let imported = myfilmlibrary_core::storage::import_poster_bytes(&bytes, &paths)
            .map_err(|e| e.to_string())?;

        Ok(imported.relative_path)
    })
    .await
    .map_err(|e| format!("Execution error: {e}"))?
}

// --------------------------- OMDB METADATA ---------------------------

#[tauri::command]
pub async fn fetch_omdb_metadata(
    title: String,
    year: Option<u16>,
    state: State<'_, AppState>,
) -> Result<MovieMetadata, String> {
    let db = Arc::clone(&state.db);
    let api_key = {
        let db_lock = db.lock().map_err(|e| e.to_string())?;
        db_lock
            .get_setting("omdb_api_key")
            .map_err(|e| e.to_string())?
    };

    let key = match api_key {
        Some(k) if !k.trim().is_empty() => k,
        _ => return Err("OMDb API key is not configured in Settings".to_string()),
    };

    tokio::task::spawn_blocking(move || {
        myfilmlibrary_core::metadata::fetch(&key, &title, year).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("Execution error: {e}"))?
}

// --------------------------- SCREENSHOTS & RELATIONS ---------------------------

#[tauri::command]
pub async fn get_film_relations(
    film_id: i64,
    state: State<'_, AppState>,
) -> Result<Vec<RelatedFilm>, String> {
    let db = Arc::clone(&state.db);
    tokio::task::spawn_blocking(move || {
        let db_lock = db.lock().map_err(|e| e.to_string())?;
        db_lock.relations_for(film_id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("Execution error: {e}"))?
}

#[tauri::command]
pub async fn add_film_relation(
    source_id: i64,
    target_id: i64,
    kind: RelationKind,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let db = Arc::clone(&state.db);
    tokio::task::spawn_blocking(move || {
        let mut db_lock = db.lock().map_err(|e| e.to_string())?;
        db_lock
            .add_relation(source_id, target_id, kind)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("Execution error: {e}"))?
}

#[tauri::command]
pub async fn remove_film_relation(
    relation_id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = Arc::clone(&state.db);
    tokio::task::spawn_blocking(move || {
        let mut db_lock = db.lock().map_err(|e| e.to_string())?;
        db_lock
            .remove_relation(relation_id)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("Execution error: {e}"))?
}

#[tauri::command]
pub async fn get_film_screenshots(
    film_id: i64,
    state: State<'_, AppState>,
) -> Result<Vec<FilmScreenshot>, String> {
    let db = Arc::clone(&state.db);
    tokio::task::spawn_blocking(move || {
        let db_lock = db.lock().map_err(|e| e.to_string())?;
        db_lock.list_screenshots(film_id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("Execution error: {e}"))?
}

#[tauri::command]
pub async fn resolve_screenshot_asset(
    stored_path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let paths = state.paths.clone();
    let abs_path = paths.resolve_screenshot(&stored_path);
    if !abs_path.exists() {
        return Err("Screenshot file does not exist".to_string());
    }
    let bytes = std::fs::read(&abs_path).map_err(|e| e.to_string())?;
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:image/jpeg;base64,{b64}"))
}

#[tauri::command]
pub async fn add_film_screenshot(
    film_id: i64,
    source_path: String,
    caption: String,
    state: State<'_, AppState>,
) -> Result<FilmScreenshot, String> {
    let db = Arc::clone(&state.db);
    let paths = state.paths.clone();
    tokio::task::spawn_blocking(move || {
        let p = Path::new(&source_path);
        let imported =
            myfilmlibrary_core::storage::import_screenshot(p, &paths).map_err(|e| e.to_string())?;

        let mut db_lock = db.lock().map_err(|e| e.to_string())?;
        let shot_id = db_lock
            .add_screenshot(film_id, &imported.relative_path, &caption)
            .map_err(|e| e.to_string())?;

        let shots = db_lock
            .list_screenshots(film_id)
            .map_err(|e| e.to_string())?;
        shots
            .into_iter()
            .find(|s| s.id == shot_id)
            .ok_or_else(|| "Failed to find newly added screenshot".to_string())
    })
    .await
    .map_err(|e| format!("Execution error: {e}"))?
}

#[tauri::command]
pub async fn delete_film_screenshot(
    screenshot_id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = Arc::clone(&state.db);
    let paths = state.paths.clone();
    tokio::task::spawn_blocking(move || {
        let mut db_lock = db.lock().map_err(|e| e.to_string())?;
        let deleted_path = db_lock
            .delete_screenshot(screenshot_id)
            .map_err(|e| e.to_string())?;

        if let Some(path) = deleted_path {
            let _ = myfilmlibrary_core::storage::delete_screenshot_file(&paths, &path);
        }
        Ok(())
    })
    .await
    .map_err(|e| format!("Execution error: {e}"))?
}

// --------------------------- IMPORT / EXPORT ---------------------------

#[tauri::command]
pub async fn export_backup_zip(
    target_path: String,
    state: State<'_, AppState>,
) -> Result<ExportSummary, String> {
    let db = Arc::clone(&state.db);
    let paths = state.paths.clone();
    tokio::task::spawn_blocking(move || {
        let db_lock = db.lock().map_err(|e| e.to_string())?;
        let p = Path::new(&target_path);
        myfilmlibrary_core::transfer::export_library(&db_lock, &paths, p).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("Execution error: {e}"))?
}

#[tauri::command]
pub async fn import_backup_zip(
    source_path: String,
    state: State<'_, AppState>,
) -> Result<ImportSummary, String> {
    let db = Arc::clone(&state.db);
    let paths = state.paths.clone();
    tokio::task::spawn_blocking(move || {
        let mut db_lock = db.lock().map_err(|e| e.to_string())?;
        let p = Path::new(&source_path);
        myfilmlibrary_core::transfer::import_library(&mut db_lock, &paths, p)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("Execution error: {e}"))?
}

#[tauri::command]
pub async fn export_films_csv(
    target_path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = Arc::clone(&state.db);
    tokio::task::spawn_blocking(move || {
        let db_lock = db.lock().map_err(|e| e.to_string())?;
        let films = db_lock.list_all_films().map_err(|e| e.to_string())?;
        let p = Path::new(&target_path);
        myfilmlibrary_core::transfer::export_to_csv(&films, p).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("Execution error: {e}"))?
}

#[tauri::command]
pub async fn export_films_markdown(
    target_path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = Arc::clone(&state.db);
    tokio::task::spawn_blocking(move || {
        let db_lock = db.lock().map_err(|e| e.to_string())?;
        let films = db_lock.list_all_films().map_err(|e| e.to_string())?;
        let p = Path::new(&target_path);
        myfilmlibrary_core::transfer::export_to_markdown(&films, p).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("Execution error: {e}"))?
}
