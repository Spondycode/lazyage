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
        recipients: Vec::new(),
        selected: false,
    });
    
    // Check CWD for keys
    if let Ok(entries) = fs::read_dir(".") {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "txt" || ext == "key" {
                   keys.extend(load_keys_from_file(&path));
                }
            }
        }
    }

    let home_dir = BaseDirs::new()
        .map(|bd| bd.home_dir().to_path_buf())
        .or_else(|| std::env::var("HOME").ok().map(PathBuf::from));

    if let Some(home) = home_dir {
        // Check ~/.ssh
        let ssh_dir = home.join(".ssh");
        if ssh_dir.exists() {
            for entry in WalkDir::new(&ssh_dir).follow_links(true).max_depth(1).into_iter().flatten() {
                let path = entry.path();
                if path.is_file() {
                    keys.extend(load_keys_from_file(path));
                }
            }
        }

        // Check ~/.config/age/
        let age_config_dir = home.join(".config").join("age");
        if age_config_dir.exists() {
            for entry in WalkDir::new(&age_config_dir).follow_links(true).max_depth(2).into_iter().flatten() {
                let path = entry.path();
                if path.is_file() {
                    keys.extend(load_keys_from_file(path));
                }
            }
        }

        // Check platform-specific config dir (e.g. ~/Library/Application Support/age on macOS)
        if let Some(base_dirs) = BaseDirs::new() {
            let plat_config_dir = base_dirs.config_dir().join("age");
            if plat_config_dir.exists() && plat_config_dir != age_config_dir {
                for entry in WalkDir::new(&plat_config_dir).follow_links(true).max_depth(2).into_iter().flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        keys.extend(load_keys_from_file(path));
                    }
                }
            }
        }
    }

    keys
}

fn load_keys_from_file(path: &std::path::Path) -> Vec<KeyInfo> {
    let mut found_keys = Vec::new();
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return found_keys,
    };

    let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();

    // 1. Check if it's an age secret key or plugin identity file
    if content.contains("AGE-SECRET-KEY-") || content.contains("AGE-PLUGIN-") {
        for (i, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("AGE-SECRET-KEY-") || trimmed.starts_with("AGE-PLUGIN-") {
                let public_key = content.lines().take(i)
                    .find(|l| l.starts_with("# public key: "))
                    .map(|l| l.replace("# public key: ", "").trim().to_string());
                
                let mut recipients = Vec::new();
                if let Some(pk) = public_key {
                    recipients.push(pk);
                }

                found_keys.push(KeyInfo {
                    name: if found_keys.is_empty() { filename.clone() } else { format!("{} (Key {})", filename, found_keys.len() + 1) },
                    path: path.to_path_buf(),
                    is_secret: true,
                    is_passphrase_only: false,
                    recipients,
                    selected: false,
                });
            }
        }
        if !found_keys.is_empty() {
            return found_keys;
        }
    }

    // 2. Check for public keys (one per line, supporting multiple)
    let mut file_recipients = Vec::new();
    for line in content.lines() {
        let mut trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Strip common labels
        if trimmed.to_lowercase().starts_with("public key: ") {
            trimmed = trimmed[12..].trim();
        } else if trimmed.to_lowercase().starts_with("recipient: ") {
            trimmed = trimmed[11..].trim();
        }

        if trimmed.starts_with("age1") || 
           trimmed.starts_with("ssh-ed25519") || 
           trimmed.starts_with("ssh-rsa") || 
           trimmed.starts_with("ecdsa-sha2-") || 
           trimmed.starts_with("ssh-dss") 
        {
            // Extract only the key part (first word for age keys, or part of SSH keys)
            let key_part = if trimmed.starts_with("age1") {
                trimmed.split_whitespace().next().unwrap_or(trimmed).to_string()
            } else {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 2 {
                    format!("{} {}", parts[0], parts[1])
                } else {
                    trimmed.to_string()
                }
            };
            file_recipients.push(key_part);
        }
    }

    if !file_recipients.is_empty() {
        found_keys.push(KeyInfo {
            name: filename,
            path: path.to_path_buf(),
            is_secret: false,
            is_passphrase_only: false,
            recipients: file_recipients,
            selected: false,
        });
    }

    found_keys
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_load_keys_from_file() {
        let test_dir = std::env::current_dir().unwrap().join("test_temp_keys");
        fs::create_dir_all(&test_dir).unwrap();
        
        let secret_path = test_dir.join("secret.key");
        fs::write(&secret_path, "# public key: age1v9l...\nAGE-SECRET-KEY-1...").unwrap();
        
        let keys = load_keys_from_file(&secret_path);
        assert_eq!(keys.len(), 1);
        let key = &keys[0];
        assert_eq!(key.name, "secret.key");
        assert!(key.is_secret);
        assert_eq!(key.recipients, vec!["age1v9l...".to_string()]);

        let public_path = test_dir.join("public.key");
        fs::write(&public_path, "age1...\nage1another...").unwrap();
        let keys = load_keys_from_file(&public_path);
        assert_eq!(keys.len(), 1);
        let key = &keys[0];
        assert_eq!(key.name, "public.key");
        assert!(!key.is_secret);
        assert_eq!(key.recipients.len(), 2);
        assert_eq!(key.recipients[0], "age1...");
        assert_eq!(key.recipients[1], "age1another...");

        let ssh_path = test_dir.join("id_ed25519.pub");
        fs::write(&ssh_path, "ssh-ed25519 AAA...").unwrap();
        let keys = load_keys_from_file(&ssh_path);
        assert_eq!(keys.len(), 1);
        let key = &keys[0];
        assert_eq!(key.name, "id_ed25519.pub");
        assert!(!key.is_secret);
        assert_eq!(key.recipients, vec!["ssh-ed25519 AAA...".to_string()]);
        
        fs::remove_dir_all(&test_dir).unwrap();
    }

    #[test]
    fn test_discover_keys_basic() {
        // We can't easily mock BaseDirs, but we can test the CWD part
        let test_dir = std::env::current_dir().unwrap().join("test_cwd_keys");
        fs::create_dir_all(&test_dir).unwrap();
        
        // Change to test_dir
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&test_dir).unwrap();

        fs::write("cwd_secret.key", "AGE-SECRET-KEY-1...").unwrap();
        fs::write("cwd_public.key", "age1...").unwrap();
        fs::write("random.txt", "not a key").unwrap();

        let keys = discover_keys();
        
        // Should find "None (Passphrase Only)" + 2 keys in CWD
        // Plus whatever is in the user's real ~/.ssh and ~/.config/age (which we can't easily control here)
        // So we just check if our keys are present
        assert!(keys.iter().any(|k| k.name == "cwd_secret.key"));
        assert!(keys.iter().any(|k| k.name == "cwd_public.key"));
        assert!(!keys.iter().any(|k| k.name == "random.txt"));

        std::env::set_current_dir(original_dir).unwrap();
        fs::remove_dir_all(&test_dir).unwrap();
    }
}
