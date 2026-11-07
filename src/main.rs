// mod otp;
pub mod vaults;

use std::env;
use std::path::Path;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn main() -> Result<(), Box<dyn std::error::Error>> {
	// Legge il percorso del file dai parametri della riga di comando
	let args: Vec<String> = env::args().collect();
	if args.len() < 2 {
		eprintln!("Uso: {} <file_backup_aegis.json>", args[0]);
		std::process::exit(1);
	}

	let file_path = &args[1];
	if !Path::new(file_path).exists() {
		eprintln!("Errore: Il file '{}' non esiste.", file_path);
		std::process::exit(1);
	}

	// Richiede la password all'utente in modo sicuro (senza mostrarla a schermo)
	let password = rpassword::prompt_password("Inserisci la password della cassaforte Aegis: ")?;

	// Legge e analizza il JSON del file Aegis
	println!("Lettura del file in corso: {}", file_path);
	let vault = vaults::get_vault_content(vaults::VaultKind::Aegis, file_path, &password)?;

	if vault.entries.is_empty() {
		println!("Nessun OTP trovato nel vault decifrato.");
		return Ok(());
	}

	loop {
		let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
		let otps = vault.get_otps(now)?;
		// Estrae e mostra i codici OTP disponibili dal vault decifrato

		println!("\n--- OTP ---");

		for entry in otps {
			println!(
				"{} | {:0>2} s | {}",
				entry.code,
				entry.remaining_seconds,
				entry.issuer,

			);
		}
		thread::sleep(Duration::from_secs(1));
	}

	// Ok(())
}
