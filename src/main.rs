mod ui;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1200.0, 900.0)),
        ..Default::default()
    };
    eframe::run_native(
        "RayTracer!",
        options,
        Box::new(|_| Box::new(ui::App::new())),
    )
}
