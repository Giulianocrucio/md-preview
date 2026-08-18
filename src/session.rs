use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentTab {
    pub id: u64,
    pub path: PathBuf,
    pub dirty: bool,
    pub missing: bool,
    pub edit_on_open: bool,
    pub preview: bool,
}

#[derive(Debug, Default)]
pub struct DocumentSession {
    pub tabs: Vec<DocumentTab>,
    pub active_id: Option<u64>,
    pub workspace_root: Option<PathBuf>,
    next_id: u64,
    last_pinned_active_id: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize)]
struct PersistedSession {
    version: u8,
    active: Option<usize>,
    tabs: Vec<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    workspace_root: Option<PathBuf>,
}

impl DocumentSession {
    pub fn load(path: &Path) -> Self {
        let Ok(raw) = fs::read(path) else {
            return Self::default();
        };
        let Ok(saved) = serde_json::from_slice::<PersistedSession>(&raw) else {
            return Self::default();
        };
        if saved.version != 1 {
            return Self::default();
        }

        let mut session = Self::default();
        for path in saved.tabs {
            session.open(path, false);
        }
        session.active_id = saved
            .active
            .and_then(|index| session.tabs.get(index))
            .map(|tab| tab.id)
            .or_else(|| session.tabs.last().map(|tab| tab.id));
        session.last_pinned_active_id = session.active_id;
        session.workspace_root = saved.workspace_root;
        session
    }

    pub fn open(&mut self, path: PathBuf, edit_on_open: bool) -> u64 {
        let path = normalize_path(path);
        if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.path == path) {
            tab.missing = !tab.path.exists();
            tab.edit_on_open |= edit_on_open;
            tab.preview = false;
            self.active_id = Some(tab.id);
            self.last_pinned_active_id = Some(tab.id);
            return tab.id;
        }

        self.next_id += 1;
        let id = self.next_id;
        self.tabs.push(DocumentTab {
            id,
            missing: !path.exists(),
            path,
            dirty: false,
            edit_on_open,
            preview: false,
        });
        self.active_id = Some(id);
        self.last_pinned_active_id = Some(id);
        id
    }

    pub fn open_preview(&mut self, path: PathBuf) -> u64 {
        if let Some(active) = self.active().filter(|tab| !tab.preview) {
            self.last_pinned_active_id = Some(active.id);
        }
        let path = normalize_path(path);
        if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.path == path) {
            tab.missing = !tab.path.exists();
            self.active_id = Some(tab.id);
            if !tab.preview {
                self.last_pinned_active_id = Some(tab.id);
            }
            return tab.id;
        }

        if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.preview && !tab.dirty) {
            tab.path = path;
            tab.missing = !tab.path.exists();
            tab.edit_on_open = false;
            let id = tab.id;
            self.active_id = Some(id);
            return id;
        }

        self.next_id += 1;
        let id = self.next_id;
        self.tabs.push(DocumentTab {
            id,
            missing: !path.exists(),
            path,
            dirty: false,
            edit_on_open: false,
            preview: true,
        });
        self.active_id = Some(id);
        id
    }

    pub fn pin(&mut self, id: u64) -> bool {
        let Some(tab) = self.get_mut(id) else {
            return false;
        };
        if !tab.preview {
            return false;
        }
        tab.preview = false;
        if self.active_id == Some(id) {
            self.last_pinned_active_id = Some(id);
        }
        true
    }

    pub fn set_active_dirty(&mut self, dirty: bool) -> bool {
        let Some(tab) = self.active_mut() else {
            return false;
        };
        tab.dirty = dirty;
        if dirty {
            tab.preview = false;
            self.last_pinned_active_id = Some(tab.id);
        }
        true
    }

    pub fn activate(&mut self, id: u64) -> bool {
        let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == id) else {
            return false;
        };
        tab.missing = !tab.path.exists();
        self.active_id = Some(id);
        if !tab.preview {
            self.last_pinned_active_id = Some(id);
        }
        true
    }

    pub fn close(&mut self, id: u64) -> bool {
        let Some(index) = self.tabs.iter().position(|tab| tab.id == id) else {
            return false;
        };
        let was_active = self.active_id == Some(id);
        self.tabs.remove(index);
        if was_active {
            self.active_id = self
                .tabs
                .get(index)
                .or_else(|| index.checked_sub(1).and_then(|i| self.tabs.get(i)))
                .map(|tab| tab.id);
        }
        if let Some(active) = self.active().filter(|tab| !tab.preview) {
            self.last_pinned_active_id = Some(active.id);
        } else if self.last_pinned_active_id == Some(id) {
            self.last_pinned_active_id = self
                .tabs
                .iter()
                .rev()
                .find(|tab| !tab.preview)
                .map(|tab| tab.id);
        }
        true
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let persisted_tabs = self
            .tabs
            .iter()
            .filter(|tab| !tab.preview)
            .collect::<Vec<_>>();
        let active_id = self
            .active_id
            .filter(|id| persisted_tabs.iter().any(|tab| tab.id == *id))
            .or_else(|| {
                self.last_pinned_active_id
                    .filter(|id| persisted_tabs.iter().any(|tab| tab.id == *id))
            });
        let active = active_id.and_then(|id| persisted_tabs.iter().position(|tab| tab.id == id));
        let saved = PersistedSession {
            version: 1,
            active,
            tabs: persisted_tabs
                .into_iter()
                .map(|tab| tab.path.clone())
                .collect(),
            workspace_root: self.workspace_root.clone(),
        };
        let body = serde_json::to_vec_pretty(&saved).map_err(io::Error::other)?;
        let temporary = path.with_extension("json.tmp");
        fs::write(&temporary, body)?;
        #[cfg(target_os = "windows")]
        if path.exists() {
            fs::remove_file(path)?;
        }
        fs::rename(temporary, path)
    }

    pub fn active(&self) -> Option<&DocumentTab> {
        let id = self.active_id?;
        self.tabs.iter().find(|tab| tab.id == id)
    }

    pub fn active_mut(&mut self) -> Option<&mut DocumentTab> {
        let id = self.active_id?;
        self.tabs.iter_mut().find(|tab| tab.id == id)
    }

    pub fn get_mut(&mut self, id: u64) -> Option<&mut DocumentTab> {
        self.tabs.iter_mut().find(|tab| tab.id == id)
    }

    pub fn relocate(&mut self, id: u64, path: PathBuf) -> bool {
        let path = normalize_path(path);
        if self.tabs.iter().any(|tab| tab.id != id && tab.path == path) {
            return false;
        }
        let Some(tab) = self.get_mut(id) else {
            return false;
        };
        tab.path = path;
        tab.missing = !tab.path.exists();
        tab.preview = false;
        if self.active_id == Some(id) {
            self.last_pinned_active_id = Some(id);
        }
        true
    }
}

fn normalize_path(path: PathBuf) -> PathBuf {
    let absolute = if path.is_absolute() {
        path
    } else {
        std::env::current_dir().unwrap_or_default().join(path)
    };
    fs::canonicalize(&absolute).unwrap_or_else(|_| {
        absolute
            .parent()
            .and_then(|parent| fs::canonicalize(parent).ok())
            .and_then(|parent| absolute.file_name().map(|name| parent.join(name)))
            .unwrap_or(absolute)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "md-preview-session-{name}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn opening_the_same_path_activates_without_duplicate() {
        let dir = temp_dir("dedupe");
        let file = dir.join("note.md");
        fs::write(&file, "# Note").unwrap();
        let mut session = DocumentSession::default();

        let first = session.open(file.clone(), false);
        let second = session.open(file.clone(), true);

        assert_eq!(first, second);
        assert_eq!(session.tabs.len(), 1);
        assert_eq!(session.active_id, Some(first));
        assert!(session.tabs[0].edit_on_open);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn preview_open_reuses_one_replaceable_tab() {
        let dir = temp_dir("preview_reuse");
        let pinned_path = dir.join("pinned.md");
        let first_preview_path = dir.join("first.md");
        let second_preview_path = dir.join("second.md");
        for path in [&pinned_path, &first_preview_path, &second_preview_path] {
            fs::write(path, "# Test").unwrap();
        }
        let mut session = DocumentSession::default();
        let pinned_id = session.open(pinned_path.clone(), false);

        let first_preview_id = session.open_preview(first_preview_path);
        let second_preview_id = session.open_preview(second_preview_path.clone());

        assert_eq!(first_preview_id, second_preview_id);
        assert_eq!(session.tabs.len(), 2);
        assert!(
            !session
                .tabs
                .iter()
                .find(|tab| tab.id == pinned_id)
                .unwrap()
                .preview
        );
        let preview = session
            .tabs
            .iter()
            .find(|tab| tab.id == second_preview_id)
            .unwrap();
        assert!(preview.preview);
        assert_eq!(preview.path, fs::canonicalize(second_preview_path).unwrap());
        assert_eq!(session.active_id, Some(second_preview_id));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn explicit_open_and_dirty_state_pin_a_preview() {
        let dir = temp_dir("preview_pin");
        let file = dir.join("preview.md");
        fs::write(&file, "# Test").unwrap();
        let mut session = DocumentSession::default();
        let id = session.open_preview(file.clone());

        assert!(session.tabs[0].preview);
        assert_eq!(session.open(file.clone(), false), id);
        assert!(!session.tabs[0].preview);
        assert_eq!(session.open_preview(file), id);
        assert!(!session.tabs[0].preview);

        let second = dir.join("second.md");
        fs::write(&second, "# Second").unwrap();
        let second_id = session.open_preview(second);
        assert!(session.active().unwrap().preview);
        assert!(session.pin(second_id));
        assert!(!session.active().unwrap().preview);

        let third = dir.join("third.md");
        fs::write(&third, "# Third").unwrap();
        session.open_preview(third);
        assert!(session.active().unwrap().preview);
        assert!(session.set_active_dirty(true));
        assert!(session.active().unwrap().dirty);
        assert!(!session.active().unwrap().preview);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn persisted_session_omits_transient_preview_tab() {
        let dir = temp_dir("preview_persist");
        let state_path = dir.join("session.json");
        let pinned = dir.join("pinned.md");
        let preview = dir.join("preview.md");
        fs::write(&pinned, "# Pinned").unwrap();
        fs::write(&preview, "# Preview").unwrap();
        let mut session = DocumentSession::default();
        session.open(pinned.clone(), false);
        session.open_preview(preview);

        session.save(&state_path).unwrap();
        let restored = DocumentSession::load(&state_path);

        assert_eq!(restored.tabs.len(), 1);
        assert_eq!(restored.tabs[0].path, fs::canonicalize(pinned).unwrap());
        assert!(!restored.tabs[0].preview);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn persisted_session_restores_the_last_active_pinned_tab_behind_a_preview() {
        let dir = temp_dir("preview_active_fallback");
        let state_path = dir.join("session.json");
        let first = dir.join("first.md");
        let second = dir.join("second.md");
        let preview = dir.join("preview.md");
        for path in [&first, &second, &preview] {
            fs::write(path, "# Test").unwrap();
        }

        let mut session = DocumentSession::default();
        let first_id = session.open(first.clone(), false);
        session.open(second, false);
        assert!(session.activate(first_id));
        session.open_preview(preview);
        session.save(&state_path).unwrap();

        let restored = DocumentSession::load(&state_path);
        assert_eq!(
            restored.active().map(|tab| &tab.path),
            Some(&fs::canonicalize(first).unwrap())
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn preview_fallback_tracks_a_pinned_tab_selected_after_closing_a_preview() {
        let dir = temp_dir("preview_close_fallback");
        let state_path = dir.join("session.json");
        let first = dir.join("first.md");
        let second = dir.join("second.md");
        let first_preview = dir.join("first-preview.md");
        let second_preview = dir.join("second-preview.md");
        for path in [&first, &second, &first_preview, &second_preview] {
            fs::write(path, "# Test").unwrap();
        }

        let mut session = DocumentSession::default();
        let first_id = session.open(first, false);
        let second_id = session.open(second.clone(), false);
        assert!(session.activate(first_id));
        let preview_id = session.open_preview(first_preview);
        assert!(session.close(preview_id));
        assert_eq!(session.active_id, Some(second_id));
        session.open_preview(second_preview);
        session.save(&state_path).unwrap();

        let restored = DocumentSession::load(&state_path);
        assert_eq!(
            restored.active().map(|tab| &tab.path),
            Some(&fs::canonicalize(second).unwrap())
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn closing_active_tab_selects_the_next_neighbor() {
        let dir = temp_dir("close");
        let mut session = DocumentSession::default();
        let first = session.open(dir.join("one.md"), false);
        let second = session.open(dir.join("two.md"), false);
        let third = session.open(dir.join("three.md"), false);
        assert!(session.activate(second));

        assert!(session.close(second));

        assert_eq!(session.active_id, Some(third));
        assert_eq!(
            session.tabs.iter().map(|tab| tab.id).collect::<Vec<_>>(),
            vec![first, third]
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn persisted_session_keeps_missing_tabs_and_active_order() {
        let dir = temp_dir("roundtrip");
        let state_path = dir.join("session.json");
        let existing = dir.join("existing.md");
        let missing = dir.join("missing.md");
        fs::write(&existing, "# Existing").unwrap();
        let mut session = DocumentSession::default();
        session.open(existing.clone(), false);
        session.open(missing.clone(), false);

        session.save(&state_path).unwrap();
        let restored = DocumentSession::load(&state_path);

        assert_eq!(restored.tabs.len(), 2);
        assert_eq!(restored.tabs[0].path, fs::canonicalize(existing).unwrap());
        assert_eq!(
            restored.tabs[1].path,
            fs::canonicalize(&dir)
                .unwrap()
                .join(missing.file_name().unwrap())
        );
        assert!(!restored.tabs[0].missing);
        assert!(restored.tabs[1].missing);
        assert_eq!(restored.active_id, restored.tabs.get(1).map(|tab| tab.id));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn persisted_session_preserves_workspace_root() {
        let dir = temp_dir("workspace_persist");
        let state_path = dir.join("session.json");
        let file = dir.join("doc.md");
        fs::write(&file, "# Test").unwrap();

        let mut session = DocumentSession {
            workspace_root: Some(dir.clone()),
            ..DocumentSession::default()
        };
        session.open(file, false);

        session.save(&state_path).unwrap();
        let restored = DocumentSession::load(&state_path);

        assert_eq!(restored.workspace_root, Some(dir.clone()));
        let _ = fs::remove_dir_all(dir);
    }
}
