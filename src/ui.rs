use crate::storage;
use crate::pwdmgr;
use crate::vaults::{self, Otp, VaultContent, VaultKind};
use eframe::egui;
use eframe::egui::Color32;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::path::PathBuf;


const USER_VAULT_NAME: &str = ".otp-vault";

#[derive(Debug)]
enum AppError {
	CannotRetrievePassword,
	HomeDirectoryUnknown,
	EntryNotFound,
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::CannotRetrievePassword => write!(f, "Impossibile recuperare la password dal gestore di credenziali"),
            AppError::HomeDirectoryUnknown => write!(f, "Home directory non conosciuta"),
            AppError::EntryNotFound => write!(f, "Elemento non presente"),
        }
    }
}

impl std::error::Error for AppError {}


struct RunInfo {
	vault: VaultContent,
	otps: Vec<Otp>,
	last_update: Instant,
	min_remaining: u64,
}

impl RunInfo {
	pub fn new(vault: VaultContent) -> Self {
		Self {
			vault,
			otps: Vec::new(),
			last_update: Instant::now(),
			min_remaining: 0,
		}
	}

	pub fn calc_otps(self: &mut Self) {
		let now = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.unwrap()
		.as_secs();
	
		self.otps = self.vault.get_otps(now).unwrap_or_default();
		let min = self.otps.iter().map(|otp| otp.remaining_seconds ).min();	
		self.min_remaining = if let Some(value) = min {
			value
		}
		else {
			0
		};
		self.last_update = Instant::now();
	}

}
enum AppStatus {
	Starting,
	AskVaultPassword,
	ShowLoadingMessage,
	ChangeVaultPassword,
	Loading,
	Running,
	ImportVaultOptions,
	ImportVault,
	Error,
}

pub struct OtpApp {
	status: AppStatus,
	file_path: PathBuf,
	password: String,
	new_password: String,
	password_confirm: String,
	vault_info: RunInfo,
	error: Option<String>,
	otp_to_delete: Option<usize>,
}

impl OtpApp {


	pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
		Self {
			status: AppStatus::Starting,
			file_path: PathBuf::default(),
			password: String::default(),
			new_password: String::default(),
			password_confirm: String::default(),
			vault_info: RunInfo {
				vault: VaultContent { entries: Vec::new() },
				otps: Vec::new(),
				last_update: Instant::now(),
				min_remaining: 0,			
			},
			otp_to_delete: None,
			error: None,
		}
	}

	/// Disegna una progress bar circolare personalizzata
	fn draw_circular_progress(ui: &mut egui::Ui, progress: f32, text: &str, remaining_secs: u64) {
		let size = egui::vec2(45.0, 45.0);
		let (rect, _response) = ui.allocate_exact_size(size, egui::Sense::hover());
		let painter = ui.painter_at(rect);

		let center = rect.center();
		let radius = rect.width() / 2.0 - 4.0;

		let circle_color: egui::Color32 = if remaining_secs < 10 {
			// red
			egui::Color32::from_rgb(222, 4, 26)
		}
		else if remaining_secs < 15 {
			// yellow
			egui::Color32::from_rgb(247, 180, 22)
		}
		else {
			// bluish
			egui::Color32::from_rgb(0, 122, 255)
		};

		const CIRCLE_WIDTH: f32 = 3.0;
		// Cerchio di sfondo
		painter.circle_stroke(
			center,
			radius,
			egui::Stroke::new(CIRCLE_WIDTH, ui.visuals().faint_bg_color),
		);

		// Arco di progresso (rappresenta il tempo rimanente in senso orario)
		let num_points = 30;
		let mut points = Vec::new();
		let start_angle = -std::f32::consts::FRAC_PI_2; // Inizia dall'alto
		let end_angle = start_angle + progress * 2.0 * std::f32::consts::PI;

		for i in 0..=num_points {
			let t = i as f32 / num_points as f32;
			let angle = start_angle + (end_angle - start_angle) * t;
			let x = center.x + radius * angle.cos();
			let y = center.y + radius * angle.sin();
			points.push(egui::pos2(x, y));
		}

		if points.len() > 1 {
			painter.add(egui::Shape::line(
				points,
				egui::Stroke::new(CIRCLE_WIDTH, circle_color),
			));
		}

		// Testo al centro (secondi rimanenti)
		painter.text(
			center,
			egui::Align2::CENTER_CENTER,
			text,
			egui::FontId::proportional(11.0),
			ui.visuals().text_color(),
		);
	}

	fn header(&self, ui: &mut egui::Ui, title: &str) {
		ui.heading(title);
		ui.add_space(10.0);
	}

	fn header_and_msg(&self, ui: &mut egui::Ui, title: &str, msg: &str) {
		self.header(ui,title);

		ui.label(egui::RichText::new(msg).strong().size(16.0));
		ui.add_space(8.0);
	}

	fn password_box(ui: &mut egui::Ui, label: &str, value: &mut String ) {
		ui.vertical(|ui| {
			ui.label(
				egui::RichText::new(label).strong(), // .size(16.0),
			);

			ui.add_sized(
				[300.0, 38.0],
				egui::TextEdit::singleline(value)
					.password(true)
					// .hint_text("Enter your password"),
			);
			ui.add_space(12.0);
		});
	}

	fn clear_passwords(self: &mut Self) {
		self.password = String::default();
		self.new_password = String::default();
		self.password_confirm = String::default();
	}

	fn error_box(ui: &mut egui::Ui, msg: &str) {
		ui.label(
			egui::RichText::new(format!("Error: {}", msg))
				.strong()
				.color(egui::Color32::RED)
				.size(16.0),
		);
	}

	fn show_error(ui: &mut egui::Ui, message: Option<&String>) {
		let msg = if let Some(text) = message {
			text
		} else {
			"Unknown"
		};
		OtpApp::error_box(ui, msg);
	}

	fn save_vault_with_pwd(password: &String, vault: &VaultContent) -> Result<(), Box<dyn std::error::Error>> {	
		if let Some(home) = std::env::home_dir() {
			let output_path = home.join(USER_VAULT_NAME);

			return storage::save_vault(vault, &output_path, &password);
		}
		else {
			return Err(Box::new(AppError::HomeDirectoryUnknown));
		}

	}

	fn save_vault(vault: &VaultContent) -> Result<(), Box<dyn std::error::Error>> {
		if let Ok(password) = pwdmgr::retrieve_password() {
			OtpApp::save_vault_with_pwd(&password, vault)
		}
		else {
			Err(Box::new(AppError::CannotRetrievePassword))
		}
	}

	fn remove_otp(vault: &mut VaultContent, ix: usize) -> Result<(), Box<dyn std::error::Error>> {
		if ix >= vault.entries.len() {
			Err(Box::new(AppError::EntryNotFound))
		}
		else {
			vault.entries.remove(ix);
			OtpApp::save_vault(vault)
		}
	}

	fn otp_label(otp: &Otp) -> String {
		format!("{} - {}", otp.issuer, otp.name)
	}
}

impl eframe::App for OtpApp {


	fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

		// let status = &mut self.status;

		egui::CentralPanel::default().show(ctx, |ui| {
			match self.status {
				AppStatus::Starting => {
					// TODO: splash screen

					self.error = None;

					if let Ok(password) = pwdmgr::retrieve_password() {

						if let Some(home) = std::env::home_dir() {
							self.file_path = home.join(USER_VAULT_NAME);
							self.password = password;
							self.status = AppStatus::ShowLoadingMessage;
						}
						else {
							self.error = Some("Vault non trovato".to_string()); // Forse da togliere
							self.status = AppStatus::Running;
						}
					}
					else {
						self.clear_passwords();
						self.status = AppStatus::AskVaultPassword;
					}

					self.error = None;

					ctx.request_repaint();
				},
				AppStatus::AskVaultPassword => {
					self.header_and_msg(ui,"Vault password", "Inserire la password");

					ui.add_sized(
						[300.0, 38.0],
						egui::TextEdit::singleline(&mut self.password)
							.password(true)
							// .hint_text("Enter your password"),
					);
					ui.add_space(12.0);

					if ui.button("OK").clicked() {
						if !self.password.is_empty() {
							if let Ok(_) = pwdmgr::store_password(&self.password) {
								self.status = AppStatus::Starting;
							}
							else {
								self.error = Some("Errore nel salvataggio della password".to_string());
								self.status = AppStatus::Error;
							}
							self.clear_passwords();
							ctx.request_repaint();
						}
					}
				},
				AppStatus::ShowLoadingMessage => {
					self.header_and_msg(ui, "Lettura vault","Caricamento in corso");

					self.status = AppStatus::Loading;

					// ctx.request_repaint();
					ctx.request_repaint_after(Duration::from_secs(1));
				},
				AppStatus::Loading => {
					self.header_and_msg(ui, "Lettura vault","Caricamento in corso");

					let vault_file = self.file_path.clone();
					let mut vault_found = false;
					if let Ok(found) = std::fs::exists(&vault_file) {
						vault_found = found;
					}
					if vault_found {
						let vault = storage::open_config(&vault_file, &self.password);
	
						if let Ok(v) = vault {
							self.status = AppStatus::Running;
							self.vault_info = RunInfo::new(v);
							self.vault_info.calc_otps();
						} else {
							self.error = Some("Lettura del vault fallita!".to_string());
							self.status = AppStatus::Error;
						}
					}
					else {
						self.error = Some("Vault non trovato!".to_string());
						self.status = AppStatus::Running;
						// No file found
					}

					ctx.request_repaint();
				},
				AppStatus::Running => {
					let elapsed = self.vault_info.last_update.elapsed().as_secs();
					if elapsed >= self.vault_info.min_remaining {
						self.vault_info.calc_otps();
					}

					self.header(ui,"Services");

					// Tabella o lista visiva
					egui::ScrollArea::vertical().show(ui, |ui| {
						if let Some(error) = self.error.as_ref() {
							OtpApp::error_box(ui, &error);
						}
						for (ix,otp) in &mut self.vault_info.otps.iter().enumerate() {
							let group = ui.group(|ui| {
								ui.horizontal(|ui| {
									// Informazioni del sito e codice OTP
									ui.vertical(|ui| {
										let label = format!("{} - {}", &otp.issuer, &otp.name);
										ui.label(
											egui::RichText::new(&label).strong().size(16.0),
										);
										ui.label(
											egui::RichText::new(&otp.code)
												.font(egui::FontId::monospace(22.0))
												.color(egui::Color32::from_rgb(46, 204, 113)),
										);
									});

									ui.with_layout(
										egui::Layout::right_to_left(egui::Align::Center),
										|ui| {
											let remaining_seconds = otp.remaining_seconds - elapsed;
											// Calcola la percentuale di progresso (da 1.0 a 0.0)
											let progress = (remaining_seconds as f32
												/ otp.period as f32)
												.clamp(0.0, 1.0);
											let seconds_text =
												format!("{:.0}s", remaining_seconds);

											// Disegna la progress bar circolare
											Self::draw_circular_progress(
												ui,
												progress,
												&seconds_text,
												remaining_seconds
											);
										},
									);
								});
							});
							let response = group.response.interact(egui::Sense::click());
							if response.hovered() {
								ui.painter().rect_stroke(
									response.rect,
									ui.style().visuals.widgets.noninteractive.rounding,
									egui::Stroke::new(2.0_f32, Color32::RED),
								);
							}
							if response.double_clicked() {
								ctx.copy_text(otp.code.clone());
							}

							response.context_menu(|ui| {
								if ui.button("Copia Token").clicked() {
									ctx.copy_text(otp.code.clone());
									ui.close_menu();
								}
								if ui.button("Elimina Voce").clicked() {
									if let None = self.otp_to_delete {
										self.otp_to_delete = Some(ix);
									}
									ui.close_menu();
								}
							});

									ui.add_space(5.0);
						}


						if let Some(index) = self.otp_to_delete {

							let label = OtpApp::otp_label(&self.vault_info.otps[index]);
							egui::Window::new("Conferma eliminazione")
								// .id(ui.make_persistent_id(index))
								.collapsible(false)
								.resizable(false)
								.anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
								.show(ui.ctx(), |ui| {
									ui.label(format!("Sei sicuro di voler eliminare {} ?", label));
									ui.add_space(8.0);

									ui.horizontal(|ui| {
										if ui.button("Sì").clicked() {
											self.otp_to_delete = None;
											if let Err(err) = OtpApp::remove_otp(&mut self.vault_info.vault,  index) {
												self.error = Some(format!("Cancellazione fallita: {}", err));
											}
											else {
												self.vault_info.calc_otps();
												return;
											}
										}

										if ui.button("No").clicked() {
											// println!("Eliminazione annullata.");
											self.otp_to_delete = None;
										}
									});
								});
						}
						
												
						if ui.button("Import").clicked() {
							self.clear_passwords();
							self.file_path = PathBuf::default();
							self.status = AppStatus::ImportVaultOptions;
							ctx.request_repaint();
						};

						if ui.button("Password").clicked() {
							self.clear_passwords();
							self.status = AppStatus::ChangeVaultPassword;
							ctx.request_repaint();
						};
					});

					// Richiede un redraw continuo per mantenere l'animazione fluida del timer
					// ctx.request_repaint();



					ctx.request_repaint_after(Duration::from_secs(1));
				},
				AppStatus::ImportVaultOptions => {
					self.header(ui,"Import OTP");
					
					// TODO: Vault Type Combo

					ui.horizontal(|ui| {
							
						if ui.button("Scegli file").clicked() {
							let files = rfd::FileDialog::new()
								// .add_filter("text", &["txt", "rs"])
								// .add_filter("rust", &["rs", "toml"])
								// .set_directory("/")
								.pick_file();

							if let Some(path) = files {
								self.file_path = path;
							}
						}

						if ui.button("Annulla").clicked() {
							self.status = AppStatus::Running;
							ctx.request_repaint();
							return;
						}
					});

					let mut file_choosed = false;

					if !self.file_path.is_empty() {
						if let Some(path) = self.file_path.to_str() {
							file_choosed = true;

							ui.add_space(12.0);
							ui.label(path);
							ui.add_space(12.0);
						}
					}

					if file_choosed {	
						ui.add_sized(
							[300.0, 38.0],
							egui::TextEdit::singleline(&mut self.password)
								.password(true)
								.hint_text("Enter import password"),
						);
						ui.add_space(12.0);
	
						if file_choosed {
							if ui.button("Importa").clicked() {
								self.status = AppStatus::ImportVault;
								ctx.request_repaint();
							}
						}
					}
				},
				AppStatus::ImportVault => {
					ui.heading("Import in corso");
					ui.add_space(10.0);
					// ui.label(egui::RichText::new("Scegli il file da importare".to_string()).strong().size(16.0));

					if let Ok(file_name) = self.file_path.clone().into_string() {
						if let Ok(vault) = vaults::get_vault_content(VaultKind::Aegis, &file_name, &self.password) {
							vault.entries.iter().for_each(|entry| {
								self.vault_info.vault.entries.push(entry.clone());
							});

							// Save vault
							self.error = Some("Errore nel salvataggio del vault".to_string());
							self.status = AppStatus::Error;

							if let Ok(_) = OtpApp::save_vault(&self.vault_info.vault) {
								self.vault_info.calc_otps();
								self.status = AppStatus::Running;
								self.error = None;
							}
						}
					}
					else {
						self.error = Some("Vault non trovato".to_string());
						self.status = AppStatus::Error;
					}

					ctx.request_repaint();
				},
				AppStatus::ChangeVaultPassword => {
					self.header_and_msg(ui,"Cambia Vault password", "Inserire la password");

					ui.vertical(|ui| {
						OtpApp::password_box( ui, "Vecchia password", &mut self.password);
						OtpApp::password_box( ui, "Nuova password", &mut self.new_password);
						OtpApp::password_box( ui, "Conferma password", &mut self.password_confirm);

						if ui.button("OK").clicked() {
							if self.password.is_empty()
								|| self.new_password.is_empty()
								|| self.password_confirm.is_empty() {

								self.error = Some("Inserisci tutte le password".to_string());
								ctx.request_repaint();
								return;
							}
							if self.new_password != self.password_confirm  {
								self.error = Some("La nuova password e quella di conferma non corrispondono".to_string());
								ctx.request_repaint();
								return;							
							};

							if let Ok(pwd) = pwdmgr::retrieve_password() {
								if pwd == self.password {
									let Ok(_) = OtpApp::save_vault_with_pwd(&self.new_password, &self.vault_info.vault) else {
										self.error = Some("Vault salvato, ma è fallito il salvataggio della password".to_string());
										self.status = AppStatus::Error;
										ctx.request_repaint();
										return;
									};

									let Ok(_) = pwdmgr::store_password(&self.new_password) else {
										self.error = Some("Errore nel salvataggio della password nel sistema !".to_string());
										self.status = AppStatus::Error;
										ctx.request_repaint();
										return;
									};

									self.status = AppStatus::Running;
									self.error = None;
									self.clear_passwords();
								}
								else {
									self.error = Some("Vecchia password non valida".to_string());
									ctx.request_repaint();
								}
							}
							else {
								self.error = Some("Errore nella lettura della password da sistema !".to_string());
								ctx.request_repaint();
							}
						}

						if let Some(err) = self.error.as_ref() {
							OtpApp::error_box(ui, err);
						}

						if ui.button("Annulla").clicked() {
							self.clear_passwords();
							self.status = AppStatus::Running;
							ctx.request_repaint();
						}
					});
				},
				AppStatus::Error => {
					self.header(ui,"Something went wrong");
					
					OtpApp::show_error(ui, self.error.as_ref());
					
					ui.add_space(12.0);
					
					if ui.button("Restart").clicked() {
						self.status = AppStatus::Starting;
						ctx.request_repaint();
					}
				},
			}
		});
	}
}
