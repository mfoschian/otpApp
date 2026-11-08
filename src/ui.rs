use crate::vaults;
use crate::{
	ui::AppStatus::Starting,
	vaults::{Otp, VaultContent},
};
use eframe::egui;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};


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
	GetVault,
	GetPassword,
	Loading,
	Running(RunInfo),
	Error(String),
}

pub struct OtpApp {
	status: AppStatus,
	file_path: String,
	password: String,
}

impl OtpApp {
	pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
		Self { status: Starting, file_path: String::default(), password: String::default() }
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
}

impl eframe::App for OtpApp {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

		let status = &mut self.status;

		egui::CentralPanel::default().show(ctx, |ui| {
			match status {
				AppStatus::Starting => {
					ui.heading("Services");
					ui.add_space(10.0);

					// ui.label(egui::RichText::new("...").strong().size(16.0));
					ui.label(
						egui::RichText::new("Nessun servizio configurato")
							.strong()
							.size(16.0),
					);

					let but_load = ui.button("Carica vault");
					if but_load.clicked() {
						*status = AppStatus::GetVault;
						ctx.request_repaint();
					}
				},
				AppStatus::GetVault => {
					ui.heading("Apertura vault");
					ui.add_space(10.0);
					ui.label(egui::RichText::new("Scegli il file da importare".to_string()).strong().size(16.0));

					// ctx.request_repaint();

					let files = rfd::FileDialog::new()
						// .add_filter("text", &["txt", "rs"])
						// .add_filter("rust", &["rs", "toml"])
						// .set_directory("/")
						.pick_file();

					let mut ok = false;

					if let Some(file) = files {
						if let Ok(path) = file.into_string() {
							self.file_path = path;
							*status = AppStatus::GetPassword;
							ok = true;
						}
					}

					if !ok {
						*status = AppStatus::Starting;
					}
					ctx.request_repaint();
				},
				AppStatus::GetPassword => {
					ui.heading("Vault password");
					ui.add_space(10.0);

					ui.label(egui::RichText::new("Inserire la password per {}".to_string()).strong().size(16.0));
					ui.add_space(8.0);

					ui.add_sized(
						[300.0, 38.0],
						egui::TextEdit::singleline(&mut self.password)
							.password(true)
							// .hint_text("Enter your password"),
					);
					ui.add_space(12.0);

					ui.horizontal(|ui| {
						if ui.button("OK").clicked() {
						*status = AppStatus::Loading;
						ctx.request_repaint();
					}

						if ui.button("Annulla").clicked() {
							*status = AppStatus::Starting;
							ctx.request_repaint();
						}
					});
				},
				AppStatus::Loading => {
					ui.heading("Lettura vault");
					ui.add_space(10.0);
					ui.label(egui::RichText::new("Caricamento in corso".to_string()).strong().size(16.0));

					let vault = vaults::get_vault_content(
						vaults::VaultKind::Aegis,
						&self.file_path,
						&self.password,
					);

					*status = if let Ok(v) = vault {
						AppStatus::Running(RunInfo::new(v))
					} else {
						AppStatus::Error("Lettura del vault fallita!".to_string())
					};

					ctx.request_repaint();
				},
				AppStatus::Error(message) => {
					ui.heading("Services");
					ui.add_space(10.0);

					ui.label(
						egui::RichText::new(format!("Error: {}", message))
							.strong()
							.size(16.0),
					);
				},
				AppStatus::Running(app) => {
					if app.last_update.elapsed() >= Duration::from_secs(1) {
						app.touch();

						let now = SystemTime::now()
							.duration_since(UNIX_EPOCH)
							.unwrap()
							.as_secs();

						app.otps = app.vault.get_otps(now).unwrap_or_default();
					}

					ui.heading("Services");
					ui.add_space(10.0);

					// Tabella o lista visiva
					egui::ScrollArea::vertical().show(ui, |ui| {
						for otp in &mut app.otps {
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
					});

					// Richiede un redraw continuo per mantenere l'animazione fluida del timer
					// ctx.request_repaint();
					ctx.request_repaint_after(Duration::from_secs(1));
				},
			}
		});
	}
}
