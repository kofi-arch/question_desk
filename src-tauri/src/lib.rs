mod ai;
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

#[tauri::command]
async fn draft_answer(app: tauri::AppHandle, id: String) -> Result<Question, String> {
    let key = std::env::var("ANTHROPIC_API_KEY")
        .ok()
        .filter(|k| !k.trim().is_empty())
        .ok_or_else(|| {
            "No API key found. Set ANTHROPIC_API_KEY in a .env file (see the README), then restart the app."
                .to_string()
        })?;
    let path = store_path(&app)?;

    // 1. load the question (lock released before any await)
    let q = {
        let _g = lock()?;
        store::find(&path, &id)?
    };

    // 2-3. call the API and parse the JSON reply
    let draft = ai::request_draft(key.trim(), &q.question, &q.context, &q.asker).await?;

    // 4-5. write it back and return the updated record
    let _g = lock()?;
    store::set_draft(&path, &id, draft)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Load ANTHROPIC_API_KEY from a local .env if present (searches parent folders too,
    // so it works from `npm run tauri dev`). Absence is fine.
    let _ = dotenvy::dotenv();

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            save_question,
            list_questions,
            delete_question,
            draft_answer
        ])
        .run(tauri::generate_context!())
        .expect("error while running Question Desk");
}
