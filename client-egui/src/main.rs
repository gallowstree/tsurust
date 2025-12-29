#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    // Log to stdout (if run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    let native_options = eframe::NativeOptions::default();

    let _ = eframe::run_native(
        "Tsurust",
        native_options,
        Box::new(|cc| Box::new(client_egui::TemplateApp::new(cc))),
    );
}

// When compiling to WASM:
#[cfg(target_arch = "wasm32")]
fn main() {
    // Initialize panic hook and logging for WASM
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let start_result = eframe::WebRunner::new()
            .start(
                "the_canvas_id",
                web_options,
                Box::new(|cc| Box::new(client_egui::TemplateApp::new(cc))),
            )
            .await;

        // Remove the loading screen
        let window = web_sys::window().expect("no window");
        let document = window.document().expect("no document");
        if let Some(loading) = document.get_element_by_id("loading") {
            loading.remove();
        }

        start_result.expect("failed to start eframe");
    });
}
