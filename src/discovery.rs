use std::fs;
use std::path::PathBuf;
use crate::app::KeyInfo;
use directories::BaseDirs;
use walkdir::WalkDir;

pub fn get_files_in_cwd() -> Vec<PathBuf> {
    fs::read_dir(".")
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect()
}

pub fn discover_keys() -> Vec<KeyInfo> {
    let mut keys = Vec::new();
    
    // Add default Passphrase Only option
    keys.push(KeyInfo {
        name: "None (Passphrase Only)".to_string(),
        path: PathBuf::new(),
        is_secret: false,
        is_passphrase_only: true,
        public_key: None,
    });
    
    // Check CWD for keys
    if let Ok(entries) = fs::read_dir(".") {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "txt" || ext == "key" {
                   if let Some(key) = try_load_key(&path) {
                       keys.push(key);
                   }
                }
            }
        }
    }

    // Check ~/.ssh
    if let Some(base_dirs) = BaseDirs::new() {
        let ssh_dir = base_dirs.home_dir().join(".ssh");
        if ssh_dir.exists() {
            for entry in WalkDir::new(ssh_dir).max_depth(1).into_iter().flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(key) = try_load_key(path) {
                        keys.push(key);
                    }
                }
            }
        }

        // Check ~/.config/age/
        let age_config_dir = base_dirs.home_dir().join(".config").join("age");
        if age_config_dir.exists() {
            for entry in WalkDir::new(age_config_dir).max_depth(2).into_iter().flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(key) = try_load_key(path) {
                        keys.push(key);
                    }
                }
            }
        }
    }

    keys
}

fn try_load_key(path: &std::path::Path) -> Option<KeyInfo> {
    let content = fs::read_to_string(path).ok()?;
    
    // Simple check if it looks like an age key
    if content.contains("AGE-SECRET-KEY-") {
        let public_key = content.lines()
            .find(|line| line.starts_with("# public key: "))
            .map(|line| line.replace("# public key: ", "").trim().to_string());

        return Some(KeyInfo {
            name: path.file_name()?.to_string_lossy().to_string(),
            path: path.to_path_buf(),
            is_secret: true,
            is_passphrase_only: false,
            public_key,
        });
    }

    // Check for SSH public keys
    if content.starts_with("ssh-ed25519") || content.starts_with("ssh-rsa") {
         return Some(KeyInfo {
            name: path.file_name()?.to_string_lossy().to_string(),
            path: path.to_path_buf(),
            is_secret: false,
            is_passphrase_only: false,
            public_key: Some(content.trim().to_string()),
        });
    }

    None
}
