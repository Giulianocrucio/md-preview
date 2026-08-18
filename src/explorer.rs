use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub relative_path: String,
    pub is_dir: bool,
    pub is_supported: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<FileEntry>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceTree {
    pub root_name: String,
    pub root_path: PathBuf,
    pub entries: Vec<FileEntry>,
}

/// Normalizes path by stripping Windows verbatim prefix (`\\?\`) if present.
fn normalize_path_buf(path: PathBuf) -> PathBuf {
    let path_str = path.to_string_lossy();
    if let Some(stripped) = path_str.strip_prefix(r"\\?\") {
        PathBuf::from(stripped)
    } else {
        path
    }
}

/// Checks if a file path has a supported Markdown or plain text document extension.
fn is_supported_document_extension(path: &Path) -> bool {
    path.extension()
        .map(|extension| {
            matches!(
                extension.to_string_lossy().to_ascii_lowercase().as_str(),
                "md" | "markdown" | "mdown" | "mkd" | "txt"
            )
        })
        .unwrap_or(false)
}

/// Checks if a directory should be ignored during workspace traversal.
fn is_ignored_directory(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | "node_modules"
            | "target"
            | ".vscode"
            | ".idea"
            | "dist"
            | "build"
            | "__pycache__"
            | ".obsidian"
            | ".gradle"
            | ".cargo"
    )
}

/// Checks if a file should be ignored during workspace traversal.
fn is_ignored_file(name: &str) -> bool {
    name.starts_with('.') || name == "Thumbs.db" || name == "desktop.ini"
}

/// Recursively scans a workspace directory up to `max_depth` levels.
/// Only Markdown and supported text documents (and folders containing them) are returned.
pub fn scan_directory(root: &Path, max_depth: usize) -> WorkspaceTree {
    let canonical_root = fs::canonicalize(root)
        .map(normalize_path_buf)
        .unwrap_or_else(|_| root.to_path_buf());

    let root_name = canonical_root
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "workspace".to_string());

    let entries = scan_dir_recursive(&canonical_root, &canonical_root, 0, max_depth);

    WorkspaceTree {
        root_name,
        root_path: canonical_root,
        entries,
    }
}

fn scan_dir_recursive(
    root: &Path,
    current_dir: &Path,
    current_depth: usize,
    max_depth: usize,
) -> Vec<FileEntry> {
    if current_depth > max_depth {
        return Vec::new();
    }

    let Ok(read_dir) = fs::read_dir(current_dir) else {
        return Vec::new();
    };

    let mut entries = Vec::new();

    for item in read_dir.flatten() {
        let path = normalize_path_buf(item.path());
        let Ok(file_type) = item.file_type() else {
            continue;
        };

        let file_name = item.file_name().to_string_lossy().to_string();

        if file_type.is_dir() {
            if is_ignored_directory(&file_name) || file_name.starts_with('.') {
                continue;
            }

            let relative = path
                .strip_prefix(root)
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_else(|_| file_name.clone());

            let children = if current_depth < max_depth {
                scan_dir_recursive(root, &path, current_depth + 1, max_depth)
            } else {
                Vec::new()
            };

            // Only include directories that contain at least one Markdown document
            if !children.is_empty() {
                entries.push(FileEntry {
                    name: file_name,
                    path,
                    relative_path: relative,
                    is_dir: true,
                    is_supported: false,
                    children: Some(children),
                });
            }
        } else if file_type.is_file() {
            if is_ignored_file(&file_name) {
                continue;
            }

            // Strictly include only supported Markdown and text documents
            if !is_supported_document_extension(&path) {
                continue;
            }

            let relative = path
                .strip_prefix(root)
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_else(|_| file_name.clone());

            entries.push(FileEntry {
                name: file_name,
                path,
                relative_path: relative,
                is_dir: false,
                is_supported: true,
                children: None,
            });
        }
    }

    // Sort: directories first (alphabetical case-insensitive), then files (alphabetical case-insensitive)
    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    entries
}

/// Finds the default document to open in a workspace directory (e.g. README.md, index.md, or first .md).
pub fn find_default_document(root: &Path) -> Option<PathBuf> {
    let canonical = fs::canonicalize(root)
        .map(normalize_path_buf)
        .unwrap_or_else(|_| root.to_path_buf());

    // Priority file names in the root directory
    let priority_names = [
        "README.md",
        "readme.md",
        "Readme.md",
        "README.markdown",
        "index.md",
        "INDEX.md",
        "README.txt",
        "readme.txt",
    ];

    for name in priority_names {
        let candidate = canonical.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    // If no priority file is found, search for the first supported document directly in the root directory
    if let Ok(read_dir) = fs::read_dir(&canonical) {
        let mut supported_files = Vec::new();
        for item in read_dir.flatten() {
            let path = normalize_path_buf(item.path());
            if path.is_file() && is_supported_document_extension(&path) {
                supported_files.push(path);
            }
        }
        supported_files.sort_by(|a, b| {
            a.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase()
                .cmp(
                    &b.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_lowercase(),
                )
        });
        if let Some(first) = supported_files.into_iter().next() {
            return Some(first);
        }
    }

    // Next, check top-level subdirectories for priority files (e.g. docs/README.md)
    if let Ok(read_dir) = fs::read_dir(&canonical) {
        for item in read_dir.flatten() {
            if let Ok(ft) = item.file_type() {
                if ft.is_dir() {
                    let sub_name = item.file_name().to_string_lossy().to_string();
                    if is_ignored_directory(&sub_name) || sub_name.starts_with('.') {
                        continue;
                    }
                    for name in priority_names {
                        let candidate = normalize_path_buf(item.path().join(name));
                        if candidate.is_file() {
                            return Some(candidate);
                        }
                    }
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_test_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("md-preview-exp-{name}-{unique}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_scan_directory_hierarchy_and_sorting() {
        let dir = temp_test_dir("hierarchy");
        let docs = dir.join("docs");
        let src = dir.join("src");
        fs::create_dir_all(&docs).unwrap();
        fs::create_dir_all(&src).unwrap();

        fs::write(dir.join("README.md"), "# Hello").unwrap();
        fs::write(dir.join("about.txt"), "About").unwrap();
        fs::write(docs.join("guide.md"), "# Guide").unwrap();
        fs::write(src.join("main.rs"), "fn main() {}").unwrap();

        let tree = scan_directory(&dir, 4);

        // Only docs, about.txt, and README.md are included (src and main.rs are omitted because src has no Markdown files)
        assert_eq!(tree.entries.len(), 3);
        // Directories first
        assert!(tree.entries[0].is_dir);
        assert_eq!(tree.entries[0].name, "docs");
        // Files after
        assert!(!tree.entries[1].is_dir);
        assert!(!tree.entries[2].is_dir);

        // Supported check
        let readme_entry = tree.entries.iter().find(|e| e.name == "README.md").unwrap();
        assert!(readme_entry.is_supported);

        let docs_entry = &tree.entries[0];
        assert!(docs_entry.children.is_some());
        let docs_children = docs_entry.children.as_ref().unwrap();
        assert_eq!(docs_children.len(), 1);
        assert_eq!(docs_children[0].name, "guide.md");
        assert!(docs_children[0].is_supported);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_scan_ignores_build_and_hidden_dirs() {
        let dir = temp_test_dir("ignores");
        let git_dir = dir.join(".git");
        let target_dir = dir.join("target");
        let node_modules = dir.join("node_modules");

        fs::create_dir_all(&git_dir).unwrap();
        fs::create_dir_all(&target_dir).unwrap();
        fs::create_dir_all(&node_modules).unwrap();

        fs::write(git_dir.join("config"), "git config").unwrap();
        fs::write(dir.join(".DS_Store"), "junk").unwrap();
        fs::write(dir.join("valid.md"), "# Valid").unwrap();

        let tree = scan_directory(&dir, 3);
        assert_eq!(tree.entries.len(), 1);
        assert_eq!(tree.entries[0].name, "valid.md");

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_find_default_document_priority() {
        let dir = temp_test_dir("default_doc");
        let sub = dir.join("guides");
        fs::create_dir_all(&sub).unwrap();

        fs::write(dir.join("zebra.md"), "# Zebra").unwrap();
        fs::write(dir.join("alpha.md"), "# Alpha").unwrap();

        // Alpha is chosen alphabetically if no README is present
        let found = find_default_document(&dir).unwrap();
        assert_eq!(found.file_name().unwrap(), "alpha.md");

        // If README.md is added, it takes precedence over alpha.md
        fs::write(dir.join("README.md"), "# Readme").unwrap();
        let found_readme = find_default_document(&dir).unwrap();
        assert_eq!(found_readme.file_name().unwrap(), "README.md");

        let _ = fs::remove_dir_all(dir);
    }
}
