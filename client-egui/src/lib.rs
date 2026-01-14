#![warn(clippy::all, rust_2018_idioms)]

mod app;
mod rendering;
mod tile_button;
mod board_renderer;
mod hand_renderer;
mod player_card;
mod screens;
mod components;
mod messaging;
mod stats_display;
mod replay_state;
mod file_io;
pub mod ws_client;

pub use app::TemplateApp;

// ----------------------------------------------------------------------------
// When compiling for web:

#[cfg(target_arch = "wasm32")]
use eframe::wasm_bindgen::{self, prelude::*, JsCast};

/// This is the entry-point for all the web-assembly.
/// This is called once from the HTML.
/// It loads the app, installs some callbacks, then returns.
/// You can add more callbacks like this if you want to call in to your code.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn start(canvas_id: &str) -> Result<(), eframe::wasm_bindgen::JsValue> {
    // Make sure panics are logged using `console.error`.
    console_error_panic_hook::set_once();

    // Redirect tracing to console.log and friends:
    tracing_wasm::set_as_global_default();

    let web_options = eframe::WebOptions::default();

    // Get the canvas element from the document
    let document = web_sys::window()
        .expect("no window")
        .document()
        .expect("no document");
    let canvas = document
        .get_element_by_id(canvas_id)
        .unwrap_or_else(|| panic!("no canvas element with id '{}'", canvas_id));
    let canvas: web_sys::HtmlCanvasElement = canvas
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .expect("element is not a canvas");

    eframe::WebRunner::new()
        .start(
            canvas,
            web_options,
            Box::new(|cc| Ok(Box::new(TemplateApp::new(cc)))),
        )
        .await
}
