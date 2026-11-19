#![windows_subsystem = "windows"]

mod ui;
pub mod vaults;
pub mod storage;
mod pwdmgr;

use eframe::egui;


fn main() -> Result<(), eframe::Error> {


    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([500.0, 400.0])
            .with_title("OTP Manager"),
        ..Default::default()
    };

    eframe::run_native(
        "OTP Manager",
        options,
        Box::new(|cc| Box::new(ui::OtpApp::new(cc))),
    )
}

