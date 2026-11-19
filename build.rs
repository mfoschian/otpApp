fn main() {
    // Esegue questo blocco solo se stiamo compilando per Windows
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/appIcon.ico"); // Percorso relativo alla root del progetto
        res.compile().expect("Errore nella compilazione delle risorse Windows");
    }
}