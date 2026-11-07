mod ui;
pub mod vaults;

use eframe::egui;


fn main() -> Result<(), eframe::Error> {


    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([350.0, 400.0])
            .with_title("OTP Manager"),
        ..Default::default()
    };

    eframe::run_native(
        "OTP Manager",
        options,
        Box::new(|cc| Box::new(ui::OtpApp::new(cc))),
    )
}

