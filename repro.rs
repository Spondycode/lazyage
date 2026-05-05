use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct KeyInfo {
    pub name: String,
    pub path: PathBuf,
    pub is_secret: bool,
    pub is_passphrase_only: bool,
    pub public_key: Option<String>,
}

fn try_load_key(path: &Path) -> Option<KeyInfo> {
    let content = fs::read_to_string(path).ok()?;
    
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

fn main() {
    let test_dir = Path::new("test_keys");
    fs::create_dir_all(test_dir.join("age")).unwrap();
    
    // Create a secret key
    fs::write(test_dir.join("age/oberyn.key"), "# public key: age1v9l... \nAGE-SECRET-KEY-1...").unwrap();
    // Create a public key (native age)
    fs::write(test_dir.join("age/david.key"), "age1...").unwrap();
    // Create an SSH key
    fs::write(test_dir.join("age/id_ed25519.pub"), "ssh-ed25519 AAA...").unwrap();
    
    println!("Testing discovery in {:?}", test_dir.join("age"));
    
    let mut keys = Vec::new();
    if test_dir.join("age").exists() {
        for entry in WalkDir::new(test_dir.join("age")).max_depth(2).into_iter().flatten() {
            let path = entry.path();
            if path.is_file() {
                println!("Checking {:?}", path);
                if let Some(key) = try_load_key(path) {
                    println!("  Found key: {}", key.name);
                    keys.push(key);
                } else {
                    println!("  Not a key");
                }
            }
        }
    }
    
    println!("Total keys found: {}", keys.len());
}
