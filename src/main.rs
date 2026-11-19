#![windows_subsystem = "windows"]

mod ui;
pub mod vaults;
pub mod storage;
mod pwdmgr;

use eframe::egui;
use std::sync::Arc;



fn load_icon(bytes: &[u8]) -> egui::IconData {
    let image = image::load_from_memory(bytes)
        .expect("Impossibile decodificare il file .ico")
        .into_rgba8();
    
    let (width, height) = image.dimensions();
    
    egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    }
}




fn main() -> Result<(), eframe::Error> {

    let icon_bytes = include_bytes!("../assets/appIcon.ico");
    let icon_data = load_icon(icon_bytes);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([500.0, 400.0])
            .with_title("OTP Manager")
            .with_icon(Arc::new(icon_data)),
        ..Default::default()
    };

    eframe::run_native(
        "OTP Manager",
        options,
        Box::new(|cc| Box::new(ui::OtpApp::new(cc))),
    )
}

