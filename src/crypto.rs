use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use anyhow::{Result, anyhow};
use age::{Encryptor, Decryptor, Recipient, Identity};
use std::str::FromStr;
use directories::BaseDirs;
use chrono::Local;

use secrecy::ExposeSecret;

pub fn generate_new_key(filename: &str) -> Result<String> {
    let identity = age::x25519::Identity::generate();
    let pubkey = identity.to_public();
    
    let home_dir = BaseDirs::new()
        .map(|bd| bd.home_dir().to_path_buf())
        .or_else(|| std::env::var("HOME").ok().map(PathBuf::from))
        .ok_or_else(|| anyhow!("Could not find home directory"))?;

    let age_dir = home_dir.join(".config").join("age");
    if !age_dir.exists() {
        std::fs::create_dir_all(&age_dir)?;
    }

    let file_path = age_dir.join(format!("{}.key", filename));
    if file_path.exists() {
        return Err(anyhow!("File already exists: {}", file_path.display()));
    }

    let now = Local::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let content = format!(
        "# created: {}\n# public key: {}\n{}\n",
        now,
        pubkey,
        identity.to_string().expose_secret()
    );

    let mut file = File::create(&file_path)?;
    file.write_all(content.as_bytes())?;

    Ok(file_path.to_string_lossy().to_string())
}

pub fn encrypt_file(input_path: &Path, recipients: Vec<String>, passphrase: Option<String>) -> Result<String> {
    let mut input_file = File::open(input_path)?;
    let mut data = Vec::new();
    input_file.read_to_end(&mut data)?;

    let output_path = input_path.with_extension(format!("{}.age", input_path.extension().unwrap_or_default().to_str().unwrap_or(""))).with_extension("age");
    
    // Stage 1: Encrypt with recipients if any (Binary)
    let mut current_data = if !recipients.is_empty() {
        let mut age_recipients: Vec<Box<dyn Recipient + Send>> = Vec::new();
        for r in recipients {
            if r.starts_with("age1") {
                age_recipients.push(Box::new(age::x25519::Recipient::from_str(&r).map_err(|e| anyhow!("Invalid age key: {:?}", e))?));
            } else if r.starts_with("ssh-") {
                age_recipients.push(Box::new(age::ssh::Recipient::from_str(&r).map_err(|e| anyhow!("Invalid SSH key: {:?}", e))?));
            }
        }
        
        let encryptor = Encryptor::with_recipients(age_recipients).expect("Failed to create encryptor");
        let mut output = Vec::new();
        let mut writer = encryptor.wrap_output(&mut output)?;
        writer.write_all(&data)?;
        writer.finish()?;
        output
    } else {
        data
    };

    // Stage 2: Encrypt with passphrase if provided (Binary)
    if let Some(p) = passphrase {
        let encryptor = Encryptor::with_user_passphrase(p.into());
        let mut output = Vec::new();
        let mut writer = encryptor.wrap_output(&mut output)?;
        writer.write_all(&current_data)?;
        writer.finish()?;
        current_data = output;
    }

    // Final Stage: Add armor if it's an age file
    let mut final_output = Vec::new();
    let armored_writer = age::armor::ArmoredWriter::wrap_output(&mut final_output, age::armor::Format::AsciiArmor)?;
    let mut writer = armored_writer;
    writer.write_all(&current_data)?;
    writer.finish()?;

    let mut output_file = File::create(&output_path)?;
    output_file.write_all(&final_output)?;

    Ok(output_path.to_string_lossy().to_string())
}

pub fn decrypt_file(input_path: &Path, identities: Vec<PathBuf>, passphrase: Option<&str>) -> Result<String> {
    let mut current_data = std::fs::read(input_path)?;
    let mut decrypted_something = false;

    // Load age identities once
    let mut age_identities: Vec<Box<dyn Identity + Send>> = Vec::new();
    for id_path in identities {
        if let Ok(content) = std::fs::read_to_string(&id_path) {
            if content.contains("AGE-SECRET-KEY-") {
                if let Some(key_line) = content.lines().find(|l| l.starts_with("AGE-SECRET-KEY-")) {
                    if let Ok(id) = age::x25519::Identity::from_str(key_line.trim()) {
                        age_identities.push(Box::new(id));
                    }
                }
            }
        }
    }

    loop {
        let is_armored = current_data.starts_with(b"-----BEGIN AGE ENCRYPTED FILE-----");
        let is_binary = current_data.starts_with(b"age-encryption.org/v1");
        
        if !is_armored && !is_binary {
            break;
        }

        let mut next_data = Vec::new();
        let decrypt_result = {
            let reader: Box<dyn Read + Send> = if is_armored {
                Box::new(age::armor::ArmoredReader::new(std::io::Cursor::new(&current_data)))
            } else {
                Box::new(std::io::Cursor::new(&current_data))
            };

            let decryptor = match Decryptor::new(reader) {
                Ok(d) => d,
                Err(_) => break, // Not an age file or corrupted
            };

            match decryptor {
                Decryptor::Passphrase(d) => {
                    if let Some(p) = passphrase {
                        d.decrypt(&p.to_string().into(), None)
                            .map_err(|e| anyhow!("Decryption failed: {:?}", e))
                            .and_then(|mut r| {
                                r.read_to_end(&mut next_data)?;
                                Ok(true)
                            })
                    } else {
                        if decrypted_something {
                            Ok(false)
                        } else {
                            Err(anyhow!("Passphrase required"))
                        }
                    }
                }
                Decryptor::Recipients(d) => {
                    if age_identities.is_empty() {
                        Ok(false)
                    } else {
                        d.decrypt(age_identities.iter().map(|i| i.as_ref() as &dyn Identity))
                            .map_err(|e| anyhow!("Decryption failed: {:?}", e))
                            .and_then(|mut r| {
                                r.read_to_end(&mut next_data)?;
                                Ok(true)
                            })
                    }
                }
            }
        };

        match decrypt_result {
            Ok(true) => {
                current_data = next_data;
                decrypted_something = true;
            }
            Ok(false) => break,
            Err(e) => return Err(e),
        }
    }

    if !decrypted_something {
        return Err(anyhow!("Could not decrypt file with provided keys/passphrase."));
    }

    let output_path = input_path.with_extension("decrypted");
    let mut output_file = File::create(&output_path)?;
    output_file.write_all(&current_data)?;

    Ok(output_path.to_string_lossy().to_string())
}
