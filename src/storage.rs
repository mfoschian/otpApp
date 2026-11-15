use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use argon2::Argon2;
use base64::{Engine as _, engine::general_purpose::STANDARD_NO_PAD};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::io;
use std::path::PathBuf;


const TOKEN_PREFIX: &str = "v1:";
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;

use crate::vaults::{VaultContent, Entry};


fn encode_token(token: &str, password: &str) -> Result<String, Box<dyn Error>> {
    let mut salt = [0; SALT_LEN];
    let mut nonce_bytes = [0; NONCE_LEN];
    let mut rng = rand::rngs::OsRng;
    rng.try_fill_bytes(&mut salt)
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "secure random generation failed"))?;
    rng.try_fill_bytes(&mut nonce_bytes)
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "secure random generation failed"))?;

    let key = derive_key(password, &salt)?;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid encryption key"))?;
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce_bytes), token.as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "token encryption failed"))?;

    let mut payload = Vec::with_capacity(SALT_LEN + NONCE_LEN + ciphertext.len());
    payload.extend_from_slice(&salt);
    payload.extend_from_slice(&nonce_bytes);
    payload.extend_from_slice(&ciphertext);
    Ok(format!("{TOKEN_PREFIX}{}", STANDARD_NO_PAD.encode(payload)))
}

fn decrypt_token(token: &str, password: &str) -> Result<String, Box<dyn Error>> {
    let encoded = token
        .strip_prefix(TOKEN_PREFIX)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "unsupported token format"))?;
    let payload = STANDARD_NO_PAD.decode(encoded)?;
    if payload.len() < SALT_LEN + NONCE_LEN + TAG_LEN {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "token is too short").into());
    }

    let salt = &payload[..SALT_LEN];
    let nonce = Nonce::from_slice(&payload[SALT_LEN..SALT_LEN + NONCE_LEN]);
    let ciphertext = &payload[SALT_LEN + NONCE_LEN..];
    let key = derive_key(password, salt)?;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid encryption key"))?;
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "token decryption failed"))?;

    Ok(String::from_utf8(plaintext)?)
}

fn derive_key(password: &str, salt: &[u8]) -> Result<[u8; 32], io::Error> {
    let mut key = [0; 32];
    Argon2::default()
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "key derivation failed"))?;
    Ok(key)
}

fn encrypt_vault(vault: &VaultContent, password: &str) -> Result<VaultContent, Box<dyn Error>> {

    let enc_vault = VaultContent { entries: vec!() };
    let secret = encode_token(token, password)?;

    for entry in &mut vault.entries {
        let e = Entry {
            issuer: entry.issuer.clone(),
            name: entry.name.clone(),
            kind: entry.kind.clone(),
            algorithm: entry.algorithm.clone(),
            digits: entry.digits,
            period: entry.period,
            counter: entry.counter,
            secret,
        };
    }

    Ok(enc_vault);
}

pub fn save_vault(vault: &VaultContent, output_path: &PathBuf, password: &str) -> Result<(), Box<dyn Error>> {

    encrypt_vault_tokens(&mut config, password)?;

    let mut output = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"\t");
    let mut serializer = serde_json::Serializer::with_formatter(&mut output, formatter);
    config.serialize(&mut serializer)?;
    fs::write(output_path, output)?;
    println!("Encrypted config written to {IMPORT_OUTPUT}");
    Ok(())
}



pub fn open_config(path: PathBuf, password: &str) -> Result<VaultContent, Box<dyn Error>> {
    let contents = fs::read_to_string(path)?;
    let mut vault = serde_json::from_str(&contents)?;

    for entry in &mut vault.entries {
        entry.secret = decrypt_token(&entry.secret, &password)?;
    }

    vault
}

/*
pub fn open_user_config(password: &str) -> Result<VaultContent, Box<dyn Error>> {
    
let home = std::env::home_dir()?;
let config_file = home.join(USER_VAULT_NAME);
let f = config_file.to_str()?;

open_config(f, password)?;
}
*/
