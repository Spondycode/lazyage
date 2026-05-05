use std::fs::File;
use std::io::{Read, Write, BufReader};
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

pub fn encrypt_with_key(input_path: &Path, recipient_key: &str) -> Result<String> {
    let mut input_file = File::open(input_path)?;
    let mut data = Vec::new();
    input_file.read_to_end(&mut data)?;

    let output_path = input_path.with_extension(format!("{}.age", input_path.extension().unwrap_or_default().to_str().unwrap_or(""))).with_extension("age");
    let output_file = File::create(&output_path)?;
    
    let recipient = if recipient_key.starts_with("age1") {
        Box::new(age::x25519::Recipient::from_str(recipient_key).map_err(|e| anyhow!("Invalid age key: {:?}", e))?) as Box<dyn Recipient + Send>
    } else if recipient_key.starts_with("ssh-") {
        Box::new(age::ssh::Recipient::from_str(recipient_key).map_err(|e| anyhow!("Invalid SSH key: {:?}", e))?) as Box<dyn Recipient + Send>
    } else {
        return Err(anyhow!("Unsupported recipient format"));
    };

    let encryptor = Encryptor::with_recipients(vec![recipient]).expect("Failed to create encryptor");
    
    let armored_writer = age::armor::ArmoredWriter::wrap_output(output_file, age::armor::Format::AsciiArmor)?;
    let mut writer = encryptor.wrap_output(armored_writer)?;
    writer.write_all(&data)?;
    writer.finish()?.finish()?;

    Ok(output_path.to_string_lossy().to_string())
}

pub fn encrypt_with_passphrase(input_path: &Path, passphrase: &str) -> Result<String> {
    let mut input_file = File::open(input_path)?;
    let mut data = Vec::new();
    input_file.read_to_end(&mut data)?;

    let output_path = input_path.with_extension("age");
    let output_file = File::create(&output_path)?;

    let encryptor = Encryptor::with_user_passphrase(passphrase.to_string().into());
    
    let armored_writer = age::armor::ArmoredWriter::wrap_output(output_file, age::armor::Format::AsciiArmor)?;
    let mut writer = encryptor.wrap_output(armored_writer)?;
    writer.write_all(&data)?;
    writer.finish()?.finish()?;

    Ok(output_path.to_string_lossy().to_string())
}

pub fn decrypt_file(input_path: &Path, identities: Vec<PathBuf>, passphrase: Option<&str>) -> Result<String> {
    let input_file = File::open(input_path)?;
    let mut buf_reader = BufReader::new(input_file);
    
    let mut header = [0u8; 35];
    let _ = buf_reader.read_exact(&mut header);
    
    let input_file = File::open(input_path)?;
    let reader: Box<dyn Read + Send> = if header.starts_with(b"-----BEGIN AGE ENCRYPTED FILE-----") {
        Box::new(age::armor::ArmoredReader::new(input_file))
    } else {
        Box::new(input_file)
    };

    let decryptor = Decryptor::new(reader).map_err(|e| anyhow!("Failed to create decryptor: {:?}", e))?;
    let mut decrypted_data = Vec::new();

    match decryptor {
        Decryptor::Passphrase(d) => {
            if let Some(p) = passphrase {
                let mut reader = d.decrypt(&p.to_string().into(), None).map_err(|e| anyhow!("Decryption failed: {:?}", e))?;
                reader.read_to_end(&mut decrypted_data)?;
            } else {
                return Err(anyhow!("Passphrase required"));
            }
        }
        Decryptor::Recipients(d) => {
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

            if age_identities.is_empty() && passphrase.is_none() {
                 return Err(anyhow!("No valid age secret keys found."));
            }

            let mut reader = d.decrypt(age_identities.iter().map(|i| i.as_ref() as &dyn Identity)).map_err(|e| anyhow!("Decryption failed: {:?}", e))?;
            reader.read_to_end(&mut decrypted_data)?;
        }
    }

    let output_path = input_path.with_extension("decrypted");
    let mut output_file = File::create(&output_path)?;
    output_file.write_all(&decrypted_data)?;

    Ok(output_path.to_string_lossy().to_string())
}
