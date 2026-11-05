use data_encoding::BASE32_NOPAD;
use hmac::{Hmac, Mac};
use serde_json::Value;
use sha1::Sha1;
use sha2::{Sha256, Sha512};
use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OtpEntry {
    pub service: String,
    pub issuer: String,
    pub kind: String,
    pub code: String,
    pub remaining_seconds: u64,
    pub period: u64,
}

impl OtpEntry {
    pub fn remaining_time_text(&self) -> String {
        format_remaining_time(self.remaining_seconds)
    }
}

pub fn extract_otps(vault_json: &Value) -> Result<Vec<OtpEntry>, Box<dyn Error>> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    extract_otps_at(vault_json, now)
}

pub fn extract_otps_at(vault_json: &Value, now: u64) -> Result<Vec<OtpEntry>, Box<dyn Error>> {
    let entries = vault_json
        .get("entries")
        .and_then(Value::as_array)
        .map_or(&[][..], |items| items.as_slice());

    let mut result = Vec::new();
    for entry in entries {
        if let Some(parsed) = parse_entry(entry, now)? {
            result.push(parsed);
        }
    }

    Ok(result)
}

pub fn print_otps(vault_json: &Value) -> Result<(), Box<dyn Error>> {
    let entries = extract_otps(vault_json)?;

    if entries.is_empty() {
        println!("Nessun OTP trovato nel vault decifrato.");
        return Ok(());
    }

    println!("\n--- OTP ---");
    for entry in entries {
        println!(
            "{} | {:0>2} s | {}",
            entry.code,
            entry.remaining_seconds,
            // entry.remaining_time_text(),
            entry.service,
        );
    }

    Ok(())
}

fn parse_entry(entry: &Value, now: u64) -> Result<Option<OtpEntry>, Box<dyn Error>> {
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

    let code = generate_code(secret, &kind, algorithm.as_str(), digits, period, now, counter)?;
    let remaining_seconds = if kind == "totp" {
        if period == 0 {
            0
        } else {
            let remainder = now % period;
            if remainder == 0 {
                period
            } else {
                period - remainder
            }
        }
    } else {
        0
    };

    let service = if issuer.trim().is_empty() {
        name
    } else {
        format!("{} ({})", issuer, name)
    };

    Ok(Some(OtpEntry {
        service,
        issuer: issuer.clone(),
        kind,
        code,
        remaining_seconds,
        period,
    }))
}

fn generate_code(
    secret: &str,
    kind: &str,
    algorithm: &str,
    digits: u32,
    period: u64,
    now: u64,
    counter: u64,
) -> Result<String, Box<dyn Error>> {
    let key = decode_base32(secret)?;
    let value = match kind {
        "totp" => now / period,
        "hotp" => counter,
        _ => return Err(format!("Tipo OTP non supportato: {}", kind).into()),
    };

    let digest = hmac_digest(&key, value, algorithm)?;
    let offset = (digest[digest.len() - 1] & 0x0f) as usize;
    let binary = ((digest[offset] & 0x7f) as u32) << 24
        | ((digest[offset + 1] & 0xff) as u32) << 16
        | ((digest[offset + 2] & 0xff) as u32) << 8
        | (digest[offset + 3] & 0xff) as u32;

    let modulus = 10_u32.pow(digits);
    let otp = binary % modulus;

    Ok(format!("{:0width$}", otp, width = digits as usize))
}

fn decode_base32(secret: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let normalized = secret
        .trim()
        .replace(' ', "")
        .replace('-', "")
        .replace('=', "")
        .to_ascii_uppercase();

    let decoded = BASE32_NOPAD.decode(normalized.as_bytes())?;
    Ok(decoded)
}

fn hmac_digest(key: &[u8], counter: u64, algorithm: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut counter_bytes = [0u8; 8];
    counter_bytes.copy_from_slice(&counter.to_be_bytes());

    match algorithm.to_ascii_uppercase().as_str() {
        "SHA1" => {
            let mut mac = Hmac::<Sha1>::new_from_slice(key)?;
            mac.update(&counter_bytes);
            Ok(mac.finalize().into_bytes().to_vec())
        }
        "SHA256" => {
            let mut mac = Hmac::<Sha256>::new_from_slice(key)?;
            mac.update(&counter_bytes);
            Ok(mac.finalize().into_bytes().to_vec())
        }
        "SHA512" => {
            let mut mac = Hmac::<Sha512>::new_from_slice(key)?;
            mac.update(&counter_bytes);
            Ok(mac.finalize().into_bytes().to_vec())
        }
        value => Err(format!("Algoritmo OTP non supportato: {}", value).into()),
    }
}

fn format_remaining_time(seconds: u64) -> String {
    let minutes = seconds / 60;
    let secs = seconds % 60;
    format!("{:02}:{:02}", minutes, secs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_totp_entries_and_generates_expected_code() {
        let vault = json!({
            "version": 1,
            "entries": [
                {
                    "type": "otpauth",
                    "name": "GitHub",
                    "issuer": "GitHub",
                    "secret": "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ",
                    "algorithm": "SHA1",
                    "digits": 8,
                    "period": 30,
                    "type": "totp"
                }
            ]
        });

        let entries = extract_otps_at(&vault, 59).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].service, "GitHub (GitHub)");
        assert_eq!(entries[0].code, "94287082");
        assert_eq!(entries[0].remaining_seconds, 1);
        assert_eq!(entries[0].remaining_time_text(), "00:01");
    }

    #[test]
    fn formats_remaining_time_in_mm_ss() {
        assert_eq!(format_remaining_time(65), "01:05");
        assert_eq!(format_remaining_time(0), "00:00");
    }
}
