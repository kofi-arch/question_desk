//! Local persistence: one JSON file, no database.
//! Deliberately free of any Tauri types so it can be unit-tested on its own.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::Mutex;

/// Serialises every read-modify-write of the file (commands can run concurrently).
/// Never hold this guard across an `.await`.
pub static STORE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Draft {
    pub draft: String,
    pub verify: Vec<String>,
    pub model: String,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Question {
    pub id: String,
    pub asked_at: String,
    pub asker: String,
    pub context: String,
    pub question: String,
    pub tags: Vec<String>,
    pub draft: Option<Draft>,
}

/// Read all records. A missing file is an empty store; a corrupt file is an error
/// (we never silently overwrite data we could not read).
pub fn load(path: &Path) -> Result<Vec<Question>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(path)
        .map_err(|e| format!("Could not read {}: {e}", path.display()))?;
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(&raw).map_err(|e| {
        format!(
            "The saved questions file ({}) is damaged and was left untouched: {e}",
            path.display()
        )
    })
}

/// Write atomically: temp file in the same folder, then rename over the original.
pub fn save_all(path: &Path, items: &[Question]) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)
            .map_err(|e| format!("Could not create data folder {}: {e}", dir.display()))?;
    }
    let json = serde_json::to_string_pretty(items)
        .map_err(|e| format!("Could not encode questions: {e}"))?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, json).map_err(|e| format!("Could not write {}: {e}", tmp.display()))?;
    fs::rename(&tmp, path).map_err(|e| format!("Could not save {}: {e}", path.display()))
}

fn sort_newest_first(items: &mut [Question]) {
    // ISO-8601 UTC strings sort correctly as text.
    items.sort_by(|a, b| b.asked_at.cmp(&a.asked_at).then_with(|| b.id.cmp(&a.id)));
}

pub fn list(path: &Path) -> Result<Vec<Question>, String> {
    let mut items = load(path)?;
    sort_newest_first(&mut items);
    Ok(items)
}

pub fn add(
    path: &Path,
    asker: &str,
    context: &str,
    question: &str,
    tags: Vec<String>,
) -> Result<Question, String> {
    let question = question.trim();
    if question.is_empty() {
        return Err("Please type the question before saving.".into());
    }
    let mut items = load(path)?;
    let now = chrono::Utc::now();
    let base = format!("q_{}", now.format("%Y%m%d_%H%M%S"));
    let mut id = base.clone();
    let mut n = 2;
    while items.iter().any(|q| q.id == id) {
        id = format!("{base}_{n}");
        n += 1;
    }
    let tags = tags
        .into_iter()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect();
    let record = Question {
        id,
        asked_at: now.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        asker: asker.trim().to_string(),
        context: context.trim().to_string(),
        question: question.to_string(),
        tags,
        draft: None,
    };
    items.push(record.clone());
    save_all(path, &items)?;
    Ok(record)
}

pub fn remove(path: &Path, id: &str) -> Result<(), String> {
    let mut items = load(path)?;
    let before = items.len();
    items.retain(|q| q.id != id);
    if items.len() == before {
        return Err("That question no longer exists.".into());
    }
    save_all(path, &items)
}

pub fn find(path: &Path, id: &str) -> Result<Question, String> {
    load(path)?
        .into_iter()
        .find(|q| q.id == id)
        .ok_or_else(|| "That question no longer exists.".to_string())
}

pub fn set_draft(path: &Path, id: &str, draft: Draft) -> Result<Question, String> {
    let mut items = load(path)?;
    let q = items
        .iter_mut()
        .find(|q| q.id == id)
        .ok_or_else(|| "That question was deleted while the draft was being written.".to_string())?;
    q.draft = Some(draft);
    let updated = q.clone();
    save_all(path, &items)?;
    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_path(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("qd_test_{}_{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir.join("questions.json")
    }

    #[test]
    fn missing_file_is_empty() {
        assert!(list(&tmp_path("missing")).unwrap().is_empty());
    }

    #[test]
    fn add_list_delete_roundtrip() {
        let p = tmp_path("roundtrip");
        let a = add(&p, "youth group", "after session", "Why suffering?", vec![" suffering ".into(), "".into()]).unwrap();
        let b = add(&p, "", "", "Is the Bible reliable?", vec![]).unwrap();
        assert_ne!(a.id, b.id, "ids must be unique even within one second");
        assert_eq!(a.tags, vec!["suffering"]);
        assert_eq!(list(&p).unwrap().len(), 2);
        remove(&p, &a.id).unwrap();
        assert_eq!(list(&p).unwrap().len(), 1);
        assert!(remove(&p, &a.id).is_err());
    }

    #[test]
    fn empty_question_rejected() {
        assert!(add(&tmp_path("empty"), "x", "y", "   ", vec![]).is_err());
    }

    #[test]
    fn draft_persists() {
        let p = tmp_path("draft");
        let q = add(&p, "", "", "Q?", vec![]).unwrap();
        let d = Draft { draft: "hi".into(), verify: vec!["a".into()], model: "m".into(), created_at: "t".into() };
        let updated = set_draft(&p, &q.id, d.clone()).unwrap();
        assert_eq!(updated.draft, Some(d.clone()));
        assert_eq!(find(&p, &q.id).unwrap().draft, Some(d));
    }

    #[test]
    fn corrupt_file_is_not_overwritten() {
        let p = tmp_path("corrupt");
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, "{ not json").unwrap();
        assert!(add(&p, "", "", "Q?", vec![]).is_err());
        assert_eq!(fs::read_to_string(&p).unwrap(), "{ not json");
    }
}
