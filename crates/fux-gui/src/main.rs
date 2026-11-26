//! `fux-gui`: the graphical interface of fux-kontrapunkt.

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Fux – Kontrapunkt")
            .with_inner_size([1360.0, 880.0])
            .with_min_inner_size([960.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "fux-gui",
        options,
        Box::new(|cc| Ok(Box::new(fux_gui::FuxApp::new(cc)))),
    )
}
