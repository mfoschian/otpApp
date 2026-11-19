use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use scrypt::{scrypt, Params};
use serde::Deserialize;
use std::fs;
use std::path::Path;
use serde_json::Value;
use std::error::Error;
// use std::time::{SystemTime, UNIX_EPOCH};

use crate::vaults;

pub fn read_vault(file_path: &str, password: &str) -> Result<vaults::VaultContent, Box<dyn std::error::Error>> {
	let parsed_json = read_aegis_vault(file_path, password)?;

	let aegis_entries = parsed_json
		.get("entries")
		.and_then(Value::as_array)
		.map_or(&[][..], |items| items.as_slice());

	let mut entries: Vec<vaults::Entry> = Vec::new();
	for entry in aegis_entries {
		if let Some(parsed) = parse_entry(entry)? {
			entries.push(parsed);
		}
	}	

	Ok(vaults::VaultContent { entries })
}



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

fn read_aegis_vault(file_path: &str, password: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
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




// #[derive(Debug, Clone, PartialEq, Eq)]
// pub struct OtpEntry {
// 	pub service: String,
// 	pub issuer: String,
// 	pub kind: String,
// 	pub code: String,
// 	pub remaining_seconds: u64,
// 	pub period: u64,
// }

// impl OtpEntry {
// 	pub fn remaining_time_text(&self) -> String {
// 		format_remaining_time(self.remaining_seconds)
// 	}
// }

// pub fn extract_otps(vault_json: &Value) -> Result<Vec<OtpEntry>, Box<dyn Error>> {
// 	let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
// 	extract_otps_at(vault_json, now)
// }

// pub fn extract_otps_at(vault_json: &Value, now: u64) -> Result<Vec<OtpEntry>, Box<dyn Error>> {
// 	let entries = vault_json
// 		.get("entries")
// 		.and_then(Value::as_array)
// 		.map_or(&[][..], |items| items.as_slice());

// 	let mut result = Vec::new();
// 	for entry in entries {
// 		if let Some(parsed) = parse_entry(entry, now)? {
// 			result.push(parsed);
// 		}
// 	}

// 	Ok(result)
// }

// pub fn print_otps(vault_json: &Value) -> Result<(), Box<dyn Error>> {
// 	let entries = extract_otps(vault_json)?;

// 	if entries.is_empty() {
// 		println!("Nessun OTP trovato nel vault decifrato.");
// 		return Ok(());
// 	}

// 	println!("\n--- OTP ---");
// 	for entry in entries {
// 		println!(
// 			"{} | {:0>2} s | {}",
// 			entry.code,
// 			entry.remaining_seconds,
// 			// entry.remaining_time_text(),
// 			entry.service,
// 		);
// 	}

// 	Ok(())
// }

// fn parse_entry(entry: &Value, now: u64) -> Result<Option<OtpEntry>, Box<dyn Error>> {
fn parse_entry(entry: &Value) -> Result<Option<vaults::Entry>, Box<dyn Error>> {
	let info = entry.get("info").and_then(Value::as_object);

	let secret = entry
		.get("secret")
		.and_then(Value::as_str)
		.or_else(|| info.and_then(|v| v.get("secret")).and_then(Value::as_str))
		.filter(|value| !value.trim().is_empty());

	let Some(secret) = secret else {
		return Ok(None);
	};

	let kind = entry
		.get("type")
		.and_then(Value::as_str)
		.or_else(|| info.and_then(|v| v.get("type")).and_then(Value::as_str))
		.unwrap_or("totp")
		.to_ascii_lowercase();

	let kind = if kind == "otpauth" || kind == "otp" {
		entry
			.get("otpauth")
			.and_then(Value::as_str)
			.unwrap_or("totp")
			.to_ascii_lowercase()
	} else {
		kind
	};

	if !matches!(kind.as_str(), "totp" | "hotp") {
		return Ok(None);
	}

	let name = entry
		.get("name")
		.and_then(Value::as_str)
		.or_else(|| entry.get("label").and_then(Value::as_str))
		.or_else(|| info.and_then(|v| v.get("label")).and_then(Value::as_str))
		.unwrap_or("Senza nome")
		.to_string();

	let issuer = entry
		.get("issuer")
		.and_then(Value::as_str)
		.or_else(|| info.and_then(|v| v.get("issuer")).and_then(Value::as_str))
		.unwrap_or("")
		.to_string();

	let algorithm = entry
		.get("algorithm")
		.and_then(Value::as_str)
		.or_else(|| info.and_then(|v| v.get("algorithm")).and_then(Value::as_str))
		.unwrap_or("SHA1")
		.to_ascii_uppercase();

	let digits = entry
		.get("digits")
		.and_then(Value::as_u64)
		.or_else(|| info.and_then(|v| v.get("digits")).and_then(Value::as_u64))
		.unwrap_or(6) as u32;

	let period = entry
		.get("period")
		.and_then(Value::as_u64)
		.or_else(|| info.and_then(|v| v.get("period")).and_then(Value::as_u64))
		.unwrap_or(30);

	let counter = entry
		.get("counter")
		.and_then(Value::as_u64)
		.or_else(|| info.and_then(|v| v.get("counter")).and_then(Value::as_u64))
		.unwrap_or(0);

	// let code = generate_code(secret, &kind, algorithm.as_str(), digits, period, now, counter)?;
	// let remaining_seconds = if kind == "totp" {
	// 	if period == 0 {
	// 		0
	// 	} else {
	// 		let remainder = now % period;
	// 		if remainder == 0 {
	// 			period
	// 		} else {
	// 			period - remainder
	// 		}
	// 	}
	// } else {
	// 	0
	// };

	// let service = if issuer.trim().is_empty() {
	// 	name
	// } else {
	// 	format!("{} ({})", issuer, name)
	// };

	// Ok(Some(OtpEntry {
	Ok(Some(crate::vaults::Entry {
		issuer: issuer.clone(),
		name,
		secret: String::from(secret),
		kind,
		algorithm,
		digits,
		period,
		counter
	}))
}


// fn format_remaining_time(seconds: u64) -> String {
// 	let minutes = seconds / 60;
// 	let secs = seconds % 60;
// 	format!("{:02}:{:02}", minutes, secs)
// }

// #[cfg(test)]
// mod tests {
// 	use super::*;
// 	use serde_json::json;

// 	#[test]
// 	fn parses_totp_entries_and_generates_expected_code() {
// 		let vault = json!({
// 			"version": 1,
// 			"entries": [
// 				{
// 					"type": "otpauth",
// 					"name": "GitHub",
// 					"issuer": "GitHub",
// 					"secret": "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ",
// 					"algorithm": "SHA1",
// 					"digits": 8,
// 					"period": 30,
// 					"type": "totp"
// 				}
// 			]
// 		});

// 		let entries = extract_otps_at(&vault, 59).unwrap();
// 		assert_eq!(entries.len(), 1);
// 		assert_eq!(entries[0].service, "GitHub (GitHub)");
// 		assert_eq!(entries[0].code, "94287082");
// 		assert_eq!(entries[0].remaining_seconds, 1);
// 		assert_eq!(entries[0].remaining_time_text(), "00:01");
// 	}

// 	#[test]
// 	fn formats_remaining_time_in_mm_ss() {
// 		assert_eq!(format_remaining_time(65), "01:05");
// 		assert_eq!(format_remaining_time(0), "00:00");
// 	}
// }
