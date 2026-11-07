mod ui;
pub mod vaults;

use std::env;
use std::path::Path;

use eframe::egui;


fn main() -> Result<(), eframe::Error> {

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
	let password = if let Ok(pwd) = rpassword::prompt_password("Inserisci la password della cassaforte Aegis: ") {
		pwd
	}
	 else {
		panic!("Impossibile avviare l'applicazione");
	};

	// Legge e analizza il JSON del file Aegis
	println!("Lettura del file in corso: {}", file_path);
	let vault = if let Ok(v) = vaults::get_vault_content(vaults::VaultKind::Aegis, file_path, &password) {
		v
	}
	else {
		panic!("Lettura del vault fallita!");
	};

	if vault.entries.is_empty() {
		println!("Nessun OTP trovato nel vault decifrato.");
		return Ok(());
	}

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([350.0, 400.0])
            .with_title("OTP Manager"),
        ..Default::default()
    };

    eframe::run_native(
        "OTP Manager",
        options,
        Box::new(|cc| Box::new(ui::OtpApp::new(cc, vault))),
    )
}

