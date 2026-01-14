//! Native (desktop) entry point for Tsurust client

pub fn main() {
    // Log to stdout (if run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    let native_options = eframe::NativeOptions::default();

    let _ = eframe::run_native(
        "Tsurust",
        native_options,
        Box::new(|cc| Box::new(client_egui::TemplateApp::new(cc))),
    );
}
