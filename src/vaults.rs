mod aegis;

use std::error::Error;
pub struct Entry {
	pub issuer: String,
	pub name: String,
	pub secret: String,
	pub kind: String,
	pub algorithm: String,
	pub digits: u32,
	pub period: u64,
	pub counter: u64,
}

pub struct Otp {
	pub issuer: String,
	pub name: String,
	pub code: String,
	pub remaining_seconds: u64,
	pub period: u64,
}

pub struct VaultContent {
	pub entries: Vec<Entry>,
}

impl VaultContent {
	pub fn get_otps(&self, now: u64) -> Result<Vec<Otp>, Box<dyn Error>> {
		let mut otps: Vec<Otp> = Vec::new();
		for entry in &self.entries {
			let otp = entry.otp(now);
			if let Ok(val) = otp {
				otps.push(val);
			}
		}
		Ok(otps)
	}
}

#[derive(Debug)]
pub enum VaultKind {
	Aegis,
}

pub fn get_vault_content(
	kind: VaultKind,
	file_name: &str,
	password: &str,
) -> Result<VaultContent, Box<dyn Error>> {
	match kind {
		VaultKind::Aegis => aegis::read_vault(file_name, password),

		// _ => Err(format!("Vault type not supported: {:?}", kind).into()),
	}
}

impl Entry {
	pub fn otp(self: &Self, now: u64) -> Result<Otp, Box<dyn Error>> {
		generate_code(
			&self.issuer,
			&self.name,
			&self.secret,
			&self.kind,
			&self.algorithm,
			self.digits,
			self.period,
			now,
			self.counter,
		)
	}
}

//
// Implementation details for generating OTP codes, decoding base32 secrets, and computing HMAC digests are provided below.
//
use data_encoding::BASE32_NOPAD;
use hmac::{Hmac, Mac};
use sha1::Sha1;
use sha2::{Sha256, Sha512};

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
		value => Err(format!("OTP algorithm not supported: {}", value).into()),
	}
}

fn generate_code(
	issuer: &str,
	name: &str,
	secret: &str,
	kind: &str,
	algorithm: &str,
	digits: u32,
	period: u64,
	now: u64,
	counter: u64,
) -> Result<Otp, Box<dyn Error>> {
	let key = decode_base32(secret)?;
	let value = match kind {
		"totp" => now / period,
		"hotp" => counter,
		_ => return Err(format!("OTP kind not supported: {}", kind).into()),
	};

	let digest = hmac_digest(&key, value, algorithm)?;
	let offset = (digest[digest.len() - 1] & 0x0f) as usize;
	let binary = ((digest[offset] & 0x7f) as u32) << 24
		| ((digest[offset + 1] & 0xff) as u32) << 16
		| ((digest[offset + 2] & 0xff) as u32) << 8
		| (digest[offset + 3] & 0xff) as u32;

	let modulus = 10_u32.pow(digits);
	let otp = binary % modulus;

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
	}
	else {
		0
	};

	let result = Otp {
		issuer: issuer.to_string(),
		name: name.to_string(),
		code: format!("{:0width$}", otp, width = digits as usize),
		remaining_seconds,
		period,
	};

	Ok(result)
}
