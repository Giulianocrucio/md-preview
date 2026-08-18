use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

const TEXT_SNIFF_LIMIT: u64 = 8 * 1024;

pub const COMMON_TEXT_EXTENSIONS: &[&str] = &[
    "md", "markdown", "mdown", "mkd", "txt", "text", "log", "json", "jsonc", "yaml", "yml", "toml",
    "xml", "csv", "tsv", "ini", "cfg", "conf", "html", "htm", "css", "js", "jsx", "ts", "tsx",
    "rs", "py", "java", "c", "h", "cpp", "hpp", "cs", "go", "rb", "php", "sh", "ps1", "sql",
];

pub fn is_markdown_document(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "md" | "markdown" | "mdown" | "mkd"
            )
        })
        .unwrap_or(false)
}

pub fn is_primary_document(path: &Path) -> bool {
    is_markdown_document(path)
        || path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.eq_ignore_ascii_case("txt"))
            .unwrap_or(false)
}

pub fn is_supported_document(path: &Path) -> bool {
    looks_like_utf8_text(path).unwrap_or(false)
}

fn looks_like_utf8_text(path: &Path) -> io::Result<bool> {
    let mut sample = Vec::new();
    File::open(path)?
        .take(TEXT_SNIFF_LIMIT)
        .read_to_end(&mut sample)?;

    if sample
        .iter()
        .any(|byte| matches!(byte, 0..=8 | 11 | 12 | 14..=31 | 127))
    {
        return Ok(false);
    }

    Ok(match std::str::from_utf8(&sample) {
        Ok(_) => true,
        Err(error) => error.error_len().is_none(),
    })
}

pub fn syntax_language(path: &Path) -> Option<String> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    if extension
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '+' | '-' | '_'))
    {
        Some(extension)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_test_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("md-preview-document-{name}-{unique}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn supports_markdown_and_other_utf8_text_files() {
        let dir = temp_test_dir("text");
        let markdown = dir.join("README.MD");
        let json = dir.join("data.json");
        let extensionless = dir.join("LICENSE");
        let text = dir.join("notes.TXT");
        fs::write(&markdown, "# Hello").unwrap();
        fs::write(&json, r#"{"enabled": true}"#).unwrap();
        fs::write(&extensionless, "MIT License").unwrap();
        fs::write(&text, "Notes").unwrap();

        assert!(is_markdown_document(&markdown));
        assert!(is_primary_document(&markdown));
        assert!(is_primary_document(&text));
        assert!(!is_primary_document(&json));
        assert!(is_supported_document(&json));
        assert!(is_supported_document(&extensionless));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn rejects_binary_and_invalid_utf8_files() {
        let dir = temp_test_dir("binary");
        let binary = dir.join("image.bin");
        let invalid_utf8 = dir.join("invalid.dat");
        let binary_markdown = dir.join("renamed.md");
        fs::write(&binary, [0x89, b'P', b'N', b'G', 0, 1]).unwrap();
        fs::write(&invalid_utf8, [0xff, 0xfe, b'A']).unwrap();
        fs::write(&binary_markdown, [0, 1, 2]).unwrap();

        assert!(!is_supported_document(&binary));
        assert!(!is_supported_document(&invalid_utf8));
        assert!(!is_supported_document(&binary_markdown));

        let _ = fs::remove_dir_all(dir);
    }
}
