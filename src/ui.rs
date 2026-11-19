use crate::storage;
use crate::pwdmgr;
use crate::vaults::{self, Otp, VaultContent, VaultKind};
use eframe::egui;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::path::PathBuf;


const USER_VAULT_NAME: &str = ".otp-vault";

struct RunInfo {
	vault: VaultContent,
	otps: Vec<Otp>,
	last_update: Instant,
}

impl RunInfo {
	pub fn new(vault: VaultContent) -> Self {
		Self {
			vault,
			otps: Vec::new(),
			last_update: Instant::now(),
		}
	}

	pub fn touch(self: &mut Self) {
		self.last_update = Instant::now();
	}
}
enum AppStatus {
	Starting,
	AskVaultPassword,
	ShowLoadingMessage,
	Loading,
	Running,
	ImportVaultOptions,
	ImportVault,
	Error,
}

pub struct OtpApp {
	status: AppStatus,
	// file_path: String,
	file_path: PathBuf,
	password: String,
	vault_info: RunInfo,
	error: Option<String>,
}

impl OtpApp {


	pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
		Self {
			status: AppStatus::Starting,
			file_path: PathBuf::default(),
			password: String::default(),
			vault_info: RunInfo {
				vault: VaultContent { entries: Vec::new() },
				otps: Vec::new(),
				last_update: Instant::now(),				
			},
			error: None,
		}
	}

	/// Disegna una progress bar circolare personalizzata
	fn draw_circular_progress(ui: &mut egui::Ui, progress: f32, text: &str) {
		let size = egui::vec2(45.0, 45.0);
		let (rect, _response) = ui.allocate_exact_size(size, egui::Sense::hover());
		let painter = ui.painter_at(rect);

		let center = rect.center();
		let radius = rect.width() / 2.0 - 4.0;

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
				egui::Stroke::new(CIRCLE_WIDTH, egui::Color32::from_rgb(0, 122, 255)),
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

	fn show_error(&self, ui: &mut egui::Ui, message: Option<&String>) {
		let msg = if let Some(text) = message {
			text
		} else {
			"Unknown"
		};
		ui.label(
			egui::RichText::new(format!("Error: {}", msg))
				.strong()
				.size(16.0),
		);
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
						self.password = String::default();
						self.status = AppStatus::AskVaultPassword;
					}
					// self.header(ui, "Services");

					// ui.label(
					// 	egui::RichText::new("Nessun servizio configurato")
					// 		.strong()
					// 		.size(16.0),
					// );

					// let but_load = ui.button("Carica vault");
					// if but_load.clicked() {
					// 	self.status = AppStatus::LoadVault;
					// 	ctx.request_repaint();
					// }

					self.error = None;
					// self.password = String::default();
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
							self.password = String::default();
							ctx.request_repaint();
						}
					}
				},
				AppStatus::ShowLoadingMessage => {
					self.header_and_msg(ui, "Lettura vault","Caricamento in corso");

					self.status = AppStatus::Loading;

					ctx.request_repaint();
				},
				AppStatus::Loading => {
					self.header_and_msg(ui, "Lettura vault","Caricamento in corso");

					let vault_file = self.file_path.clone();
					let mut vault_found = false;
					if let Ok(found) = std::fs::exists(&vault_file) {
						vault_found = found;
					}
					if( vault_found ) {
						let vault = storage::open_config(&vault_file, &self.password);
	
						if let Ok(v) = vault {
							self.status = AppStatus::Running;
							self.vault_info = RunInfo::new(v);
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
					if self.vault_info.last_update.elapsed() >= Duration::from_secs(1) {
						self.vault_info.touch();

						let now = SystemTime::now()
							.duration_since(UNIX_EPOCH)
							.unwrap()
							.as_secs();

						self.vault_info.otps = self.vault_info.vault.get_otps(now).unwrap_or_default();
					}

					self.header(ui,"Services");

					// Tabella o lista visiva
					egui::ScrollArea::vertical().show(ui, |ui| {
						for otp in &mut self.vault_info.otps {
							ui.group(|ui| {
								ui.horizontal(|ui| {
									// Informazioni del sito e codice OTP
									ui.vertical(|ui| {
										ui.label(
											egui::RichText::new(&otp.issuer).strong().size(16.0),
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
											// Calcola la percentuale di progresso (da 1.0 a 0.0)
											let progress = (otp.remaining_seconds as f32
												/ otp.period as f32)
												.clamp(0.0, 1.0);
											let seconds_text =
												format!("{:.0}s", otp.remaining_seconds);

											// Disegna la progress bar circolare
											Self::draw_circular_progress(
												ui,
												progress,
												&seconds_text,
											);
										},
									);
								});
							});
							ui.add_space(5.0);
						}

						if ui.button("Import").clicked() {
							self.password = String::default();
							self.status = AppStatus::ImportVaultOptions;
							ctx.request_repaint();
						};
					});

					// Richiede un redraw continuo per mantenere l'animazione fluida del timer
					// ctx.request_repaint();



					ctx.request_repaint_after(Duration::from_secs(1));
				},
				AppStatus::ImportVaultOptions => {
					self.header_and_msg(ui,"Import OTP","Scegli il file da importare" );

					// TODO: Vault Type Combo

					ui.add_sized(
						[300.0, 38.0],
						egui::TextEdit::singleline(&mut self.password)
							.password(true)
							.hint_text("Enter import password"),
					);
					ui.add_space(12.0);

					ui.horizontal(|ui| {
						if ui.button("OK").clicked() {
							let files = rfd::FileDialog::new()
								// .add_filter("text", &["txt", "rs"])
								// .add_filter("rust", &["rs", "toml"])
								// .set_directory("/")
								.pick_file();

							if let Some(path) = files {
								self.file_path = path;
								self.status = AppStatus::ImportVault;
							}
						}

						if ui.button("Annulla").clicked() {
							self.status = AppStatus::Running;
							ctx.request_repaint();
						}
					});

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
							if let Ok(password) = pwdmgr::retrieve_password() {
								
								if let Some(home) = std::env::home_dir() {
									let output_path = home.join(USER_VAULT_NAME);
									
									if let Ok(_) = storage::save_vault(&vault, &output_path, &password) {
										self.status = AppStatus::Running;
										self.error = None;
									}
								}
							}

						}
					}
					else {
						self.error = Some("Vault non trovato".to_string());
						self.status = AppStatus::Error;
					}

					ctx.request_repaint();
				},
				AppStatus::Error => {
					self.header(ui,"Something went wrong");

					self.show_error(ui, self.error.as_ref());

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
