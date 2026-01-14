#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

// Conditionally compile the appropriate main entry point
#[cfg(not(target_arch = "wasm32"))]
mod main_native;

#[cfg(target_arch = "wasm32")]
mod main_wasm;

// Re-export the main function from the appropriate module
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    main_native::main();
}

#[cfg(target_arch = "wasm32")]
fn main() {
    main_wasm::main();
}
