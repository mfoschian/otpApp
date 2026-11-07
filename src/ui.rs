use eframe::egui;
use crate::vaults::{VaultContent, Otp};

// use otp::OtpEntry;


pub struct OtpApp {
    vault: VaultContent,
    otps: Vec<Otp>,
}

impl OtpApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, vault: VaultContent) -> Self {
        Self {
            vault,
            otps: Vec::new(),
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
}

use std::time::{SystemTime, UNIX_EPOCH};

impl eframe::App for OtpApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Aggiorna lo stato di tutti i token OTP
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        self.otps = self.vault.get_otps(now).unwrap_or_default();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Services");
            ui.add_space(10.0);

            // Tabella o lista visiva
            egui::ScrollArea::vertical().show(ui, |ui| {
                for otp in &mut self.otps {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            // Informazioni del sito e codice OTP
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new(&otp.issuer).strong().size(16.0));
                                ui.label(
                                    egui::RichText::new(&otp.code)
                                        .font(egui::FontId::monospace(22.0))
                                        .color(egui::Color32::from_rgb(46, 204, 113)),
                                );
                            });

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                // Calcola la percentuale di progresso (da 1.0 a 0.0)
                                let progress = (otp.remaining_seconds as f32 / otp.period as f32).clamp(0.0, 1.0);
                                let seconds_text = format!("{:.0}s", otp.remaining_seconds);

                                // Disegna la progress bar circolare
                                Self::draw_circular_progress(ui, progress, &seconds_text);
                            });
                        });
                    });
                    ui.add_space(5.0);
                }
            });
        });

        // Richiede un redraw continuo per mantenere l'animazione fluida del timer
        ctx.request_repaint();
    }
}

