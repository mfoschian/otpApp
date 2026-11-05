use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use scrypt::{scrypt, Params};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Deserialize, Debug)]
struct KeyParams {
	nonce: String,
	tag: String,
}

#[derive(Deserialize, Debug)]
struct Slot {
	r#type: u32,
	#[serde(default)]
	salt: Option<String>,
	#[serde(default)]
	n: Option<u32>,
	#[serde(default)]
	r: Option<u32>,
	#[serde(default)]
	p: Option<u32>,
	#[serde(default)]
	key: Option<String>,
	#[serde(default)]
	key_params: Option<KeyParams>,
}

#[derive(Deserialize, Debug)]
struct HeaderParams {
	nonce: String,
	tag: String,
}

#[derive(Deserialize, Debug)]
struct Header {
	slots: Vec<Slot>,
	params: HeaderParams,
}

#[derive(Deserialize, Debug)]
struct AegisVault {
	version: u32,
	header: Header,
	db: String,
}

pub fn read_vault(file_path: &str, password: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
	if !Path::new(file_path).exists() {
		return Err(format!("Il file '{}' non esiste.", file_path).into());
	}

	// Legge e analizza il JSON del file Aegis
	let file_content = fs::read_to_string(file_path)?;
	let vault: AegisVault = serde_json::from_str(&file_content)?;

	// Filtra gli slot di tipo password (type == 1)
	let password_slots: Vec<&Slot> = vault
		.header
		.slots
		.iter()
		.filter(|s| s.r#type == 1)
		.collect();

	if password_slots.is_empty() {
		return Err("Nessuno slot basato su password trovato nel file di backup.".into());
	}

	let mut master_key: Option<Vec<u8>> = None;

	// Tenta di decifrare la master key iterando sugli slot compatibili
	for slot in password_slots {
		let salt_hex = slot.salt.as_ref().ok_or("Slot password senza salt")?;
		let n = slot.n.ok_or("Slot password senza parametro N")?;
		let r = slot.r.ok_or("Slot password senza parametro r")?;
		let p = slot.p.ok_or("Slot password senza parametro p")?;
		let encrypted_key_hex = slot
			.key
			.as_ref()
			.ok_or("Slot password senza chiave cifrata")?;
		let k_params = slot
			.key_params
			.as_ref()
			.ok_or("Slot password senza key_params")?;

		let salt = hex::decode(salt_hex)?;
		let log_n = (n as f64).log2() as u8;
		let params = Params::new(log_n, r, p, 32)?;

		// Derivazione della chiave con scrypt
		let mut derived_key = vec![0u8; 32];
		scrypt(password.as_bytes(), &salt, &params, &mut derived_key)?;

		// Preparazione per AES-256-GCM (Master key wrapping)
		let cipher_key = Key::<Aes256Gcm>::try_from(derived_key.as_slice())?;
		let cipher = Aes256Gcm::new(&cipher_key);

		let nonce_bytes = hex::decode(&k_params.nonce)?;
		let nonce = Nonce::try_from(nonce_bytes.as_slice())?;

		let mut ciphertext = hex::decode(encrypted_key_hex)?;
		let tag = hex::decode(&k_params.tag)?;
		ciphertext.extend(tag); // AES-GCM richiede il tag accodato al ciphertext

		let payload = Payload {
			msg: &ciphertext,
			aad: b"",
		};

		if let Ok(decrypted_master_key) = cipher.decrypt(&nonce, payload) {
			master_key = Some(decrypted_master_key);
			break;
		}
	}

	let master_key = master_key
		.ok_or("Impossibile decifrare la Master Key: password errata o file corrotto.")?;

	// Decifratura del database principale (db)
	let db_ciphertext_b64 = &vault.db;
	let mut db_ciphertext = base64::Engine::decode(
		&base64::engine::general_purpose::STANDARD,
		db_ciphertext_b64,
	)?;

	let db_nonce_bytes = hex::decode(&vault.header.params.nonce)?;
	let db_nonce = Nonce::try_from(db_nonce_bytes.as_slice())?;

	let db_tag = hex::decode(&vault.header.params.tag)?;
	db_ciphertext.extend(db_tag);

	let db_cipher_key = Key::<Aes256Gcm>::try_from(master_key.as_slice())?;
	let db_cipher = Aes256Gcm::new(&db_cipher_key);

	let db_payload = Payload {
		msg: &db_ciphertext,
		aad: b"",
	};

	let decrypted_db_bytes = db_cipher
		.decrypt(&db_nonce, db_payload)
		.map_err(|_| "Errore durante la decifratura del database (chiave o tag non validi).")?;

	let decrypted_json_str = String::from_utf8(decrypted_db_bytes)?;

	// Formattazione e stampa a schermo del JSON decifrato
	let parsed_json: serde_json::Value = serde_json::from_str(&decrypted_json_str)?;

	Ok(parsed_json)
}
