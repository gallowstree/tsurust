//! WASM (browser) entry point for Tsurust client

use wasm_bindgen::JsCast;

pub fn main() {
    // Initialize panic hook and logging for WASM
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        // Get the canvas element from the document
        let window = web_sys::window().expect("no window");
        let document = window.document().expect("no document");
        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("no canvas element with id 'the_canvas_id'");
        let canvas: web_sys::HtmlCanvasElement = canvas
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("element is not a canvas");

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(client_egui::TemplateApp::new(cc)))),
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
