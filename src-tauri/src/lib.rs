mod store;

use store::Question;
use tauri::Manager;

const STORE_FILE: &str = "questions.json";

fn store_path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Could not find the app data folder: {e}"))?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Could not create the app data folder {}: {e}", dir.display()))?;
    Ok(dir.join(STORE_FILE))
}

fn lock() -> Result<std::sync::MutexGuard<'static, ()>, String> {
    store::STORE_LOCK
        .lock()
        .map_err(|_| "Internal error: the question store is locked. Restart the app.".to_string())
}

#[tauri::command]
fn save_question(
    app: tauri::AppHandle,
    asker: String,
    context: String,
    question: String,
    tags: Vec<String>,
) -> Result<Question, String> {
    let path = store_path(&app)?;
    let _g = lock()?;
    store::add(&path, &asker, &context, &question, tags)
}

#[tauri::command]
fn list_questions(app: tauri::AppHandle) -> Result<Vec<Question>, String> {
    let path = store_path(&app)?;
    let _g = lock()?;
    store::list(&path)
}

#[tauri::command]
fn delete_question(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let path = store_path(&app)?;
    let _g = lock()?;
    store::remove(&path, &id)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            save_question,
            list_questions,
            delete_question
        ])
        .run(tauri::generate_context!())
        .expect("error while running Question Desk");
}
