use crate::vaults::{Entry, VaultContent};
use percent_encoding::percent_decode_str;
use std::error::Error;
use url::Url;

pub fn read_vault(file_path: &str) -> Result<VaultContent, Box<dyn Error>> {
	let image = image::open(file_path)?;
	let mut prepared_image = rqrr::PreparedImage::prepare(image.to_luma8());
	let grids = prepared_image.detect_grids();

	if grids.is_empty() {
		return Err("Nessun QR code trovato nell'immagine.".into());
	}

	let mut entries = Vec::with_capacity(grids.len());
	for grid in grids {
		let (_, content) = grid.decode()?;
		entries.push(parse_otp_uri(&content)?);
	}

	Ok(VaultContent { entries })
}

fn parse_otp_uri(content: &str) -> Result<Entry, Box<dyn Error>> {
	let uri = Url::parse(content)
		.map_err(|_| "Il QR code non contiene una URI OTP supportata.")?;
	if uri.scheme() != "otpauth" {
		return Err("Il QR code non contiene una URI OTP supportata.".into());
	}

	let kind = uri
		.host_str()
		.unwrap_or_default()
		.to_ascii_lowercase();
	if !matches!(kind.as_str(), "totp" | "hotp") {
		return Err(format!("Tipo OTP non supportato: {}", kind).into());
	}

	let encoded_label = uri
		.path_segments()
		.and_then(|mut segments| segments.next())
		.filter(|label| !label.is_empty())
		.ok_or("Etichetta OTP mancante nella URI.")?;
	let label = percent_decode_str(encoded_label).decode_utf8()?;
	let (label_issuer, name) = match label.split_once(':') {
		Some((issuer, name)) => (issuer.trim(), name.trim()),
		None => ("", label.trim()),
	};
	if name.is_empty() {
		return Err("Nome account mancante nella URI OTP.".into());
	}

	let query: std::collections::HashMap<String, String> =
		uri.query_pairs().into_owned().collect();
	let secret = query
		.get("secret")
		.filter(|secret| !secret.trim().is_empty())
		.ok_or("Segreto OTP mancante nella URI.")?
		.to_string();
	let normalized_secret = secret
		.trim()
		.replace(' ', "")
		.replace('-', "")
		.replace('=', "")
		.to_ascii_uppercase();
	data_encoding::BASE32_NOPAD.decode(normalized_secret.as_bytes())?;

	let issuer = query
		.get("issuer")
		.filter(|issuer| !issuer.trim().is_empty())
		.map(String::as_str)
		.unwrap_or(label_issuer)
		.to_string();
	let algorithm = query
		.get("algorithm")
		.map_or("SHA1", String::as_str)
		.to_ascii_uppercase();
	if !matches!(algorithm.as_str(), "SHA1" | "SHA256" | "SHA512") {
		return Err(format!("Algoritmo OTP non supportato: {}", algorithm).into());
	}

	let digits = parse_query_value(&query, "digits", 6_u32)?;
	if !(6..=8).contains(&digits) {
		return Err("Il numero di cifre OTP deve essere compreso tra 6 e 8.".into());
	}

	let period = if kind == "totp" {
		let period = parse_query_value(&query, "period", 30_u64)?;
		if period == 0 {
			return Err("Il periodo TOTP deve essere maggiore di zero.".into());
		}
		period
	} else {
		30
	};

	let counter = match query.get("counter") {
		Some(value) => value.parse::<u64>()?,
		None if kind == "hotp" => return Err("Contatore HOTP mancante nella URI.".into()),
		None => 0,
	};

	Ok(Entry {
		issuer,
		name: name.to_string(),
		secret,
		kind,
		algorithm,
		digits,
		period,
		counter,
	})
}

fn parse_query_value<T>(
	query: &std::collections::HashMap<String, String>,
	key: &str,
	default: T,
) -> Result<T, Box<dyn Error>>
where
	T: std::str::FromStr,
	T::Err: Error + 'static,
{
	match query.get(key) {
		Some(value) => Ok(value.parse()?),
		None => Ok(default),
	}
}

#[cfg(test)]
mod tests {
	use super::parse_otp_uri;

	#[test]
	fn parses_totp_uri_and_decodes_account_label() {
		let entry = parse_otp_uri(
			"otpauth://totp/Example%20Org:alice%40example.com?secret=JBSWY3DPEHPK3PXP&issuer=Example+Org&algorithm=SHA256&digits=8&period=45",
		)
		.unwrap();

		assert_eq!(entry.issuer, "Example Org");
		assert_eq!(entry.name, "alice@example.com");
		assert_eq!(entry.kind, "totp");
		assert_eq!(entry.algorithm, "SHA256");
		assert_eq!(entry.digits, 8);
		assert_eq!(entry.period, 45);
	}

	#[test]
	fn parses_hotp_uri_with_counter() {
		let entry =
			parse_otp_uri("otpauth://hotp/Service:account?secret=JBSWY3DPEHPK3PXP&counter=12")
				.unwrap();

		assert_eq!(entry.kind, "hotp");
		assert_eq!(entry.counter, 12);
	}

	#[test]
	fn rejects_hotps_without_a_counter() {
		assert!(parse_otp_uri("otpauth://hotp/Service:account?secret=JBSWY3DPEHPK3PXP")
			.is_err());
	}
}
