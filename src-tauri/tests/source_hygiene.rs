use std::{
    fs,
    path::{Path, PathBuf},
};

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri must live inside the repository")
        .to_path_buf()
}

fn should_skip(path: &Path) -> bool {
    let text = path.to_string_lossy();
    text.contains("\\target\\")
        || text.contains("/target/")
        || text.contains("\\node_modules\\")
        || text.contains("/node_modules/")
        || text.contains("\\.git\\")
        || text.contains("/.git/")
        || text.ends_with("Cargo.lock")
        || text.ends_with("pnpm-lock.yaml")
        || text.contains("\\dist\\")
        || text.contains("/dist/")
}

fn collect_sources(directory: &Path, output: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if should_skip(&path) {
            continue;
        }
        if path.is_dir() {
            collect_sources(&path, output);
            continue;
        }
        let extension = path.extension().and_then(|value| value.to_str()).unwrap_or("");
        if matches!(extension, "rs" | "ts" | "tsx" | "json" | "yml" | "yaml" | "toml" | "md" | "kt" | "gradle") {
            output.push(path);
        }
    }
}

fn credential_markers() -> Vec<(&'static str, String)> {
    // Части собираются в рантайме, чтобы сам тест не содержал искомых подписей.
    vec![
        ("GitHub personal token", format!("{}p_", "gh")),
        ("GitHub OAuth token", format!("{}ho_", "g")),
        ("Google API key", format!("{}za", "AI")),
        ("OpenAI project key", format!("{}-proj-", "sk")),
        ("Slack bot token", format!("{}bot-token-", "xox")),
        ("PEM private key", format!("-----BEGIN {} KEY-----", "PRIVATE")),
        ("hardcoded password assignment", format!("{} = \"", "password")),
        ("hardcoded secret assignment", format!("{} = \"", "secret")),
        ("hardcoded token assignment", format!("{} = \"", "token")),
    ]
}

#[test]
fn tracked_sources_contain_no_embedded_credentials() {
    let root = repository_root();
    let mut files = Vec::new();
    collect_sources(&root, &mut files);
    assert!(files.len() > 20, "expected the full source tree to be scanned");

    let markers = credential_markers();
    let mut offenders = Vec::new();
    for file in files {
        let contents = match fs::read_to_string(&file) {
            Ok(contents) => contents,
            Err(_) => continue,
        };
        let relative = file.strip_prefix(&root).unwrap_or(&file).display();
        for (name, needle) in &markers {
            if contents.contains(needle) {
                offenders.push(format!("{relative}: {name}"));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "credential-like literals found in source:\n{}",
        offenders.join("\n")
    );
}
