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
        selected: false,
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
                    if let Some(key) = try_load_key(path) {
                        keys.push(key);
                    }
                }
            }
        }

        // Check ~/.config/age/
        let age_config_dir = home.join(".config").join("age");
        if age_config_dir.exists() {
            for entry in WalkDir::new(&age_config_dir).follow_links(true).max_depth(2).into_iter().flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(key) = try_load_key(path) {
                        keys.push(key);
                    }
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
                        if let Some(key) = try_load_key(path) {
                            keys.push(key);
                        }
                    }
                }
            }
        }
    }

    keys
}

fn try_load_key(path: &std::path::Path) -> Option<KeyInfo> {
    let content = fs::read_to_string(path).ok()?;
    let trimmed_content = content.trim();
    
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
            selected: false,
        });
    }

    // Check for native age public keys
    if trimmed_content.starts_with("age1") {
         return Some(KeyInfo {
            name: path.file_name()?.to_string_lossy().to_string(),
            path: path.to_path_buf(),
            is_secret: false,
            is_passphrase_only: false,
            public_key: Some(trimmed_content.to_string()),
            selected: false,
        });
    }

    // Check for SSH public keys
    if trimmed_content.starts_with("ssh-ed25519") 
        || trimmed_content.starts_with("ssh-rsa")
        || trimmed_content.starts_with("ecdsa-sha2-")
        || trimmed_content.starts_with("ssh-dss") 
    {
         return Some(KeyInfo {
            name: path.file_name()?.to_string_lossy().to_string(),
            path: path.to_path_buf(),
            is_secret: false,
            is_passphrase_only: false,
            public_key: Some(trimmed_content.to_string()),
            selected: false,
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_try_load_key() {
        let test_dir = std::env::current_dir().unwrap().join("test_temp_keys");
        fs::create_dir_all(&test_dir).unwrap();
        
        let secret_path = test_dir.join("secret.key");
        fs::write(&secret_path, "# public key: age1v9l...\nAGE-SECRET-KEY-1...").unwrap();
        
        let key = try_load_key(&secret_path).unwrap();
        assert_eq!(key.name, "secret.key");
        assert!(key.is_secret);
        assert_eq!(key.public_key, Some("age1v9l...".to_string()));

        let public_path = test_dir.join("public.key");
        fs::write(&public_path, "age1...").unwrap();
        let key = try_load_key(&public_path).unwrap();
        assert_eq!(key.name, "public.key");
        assert!(!key.is_secret);
        assert_eq!(key.public_key, Some("age1...".to_string()));

        let ssh_path = test_dir.join("id_ed25519.pub");
        fs::write(&ssh_path, "ssh-ed25519 AAA...").unwrap();
        let key = try_load_key(&ssh_path).unwrap();
        assert_eq!(key.name, "id_ed25519.pub");
        assert!(!key.is_secret);
        assert_eq!(key.public_key, Some("ssh-ed25519 AAA...".to_string()));
        
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
