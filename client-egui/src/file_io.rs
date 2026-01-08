use tsurust_common::game::GameExport;

/// Save a game export to a JSON file (native platforms)
#[cfg(not(target_arch = "wasm32"))]
pub fn save_game_export(export: &GameExport) {
    use rfd::FileDialog;
    use std::fs;

    // Generate filename with timestamp
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let default_filename = format!("tsurust_replay_{}.json", timestamp);

    if let Some(path) = FileDialog::new()
        .add_filter("Tsurust Replay", &["json"])
        .set_file_name(&default_filename)
        .save_file()
    {
        match serde_json::to_string_pretty(export) {
            Ok(json) => {
                if let Err(e) = fs::write(&path, json) {
                    eprintln!("Failed to write replay file: {}", e);
                } else {
                    println!("Replay saved to: {:?}", path);
                }
            }
            Err(e) => {
                eprintln!("Failed to serialize game export: {}", e);
            }
        }
    }
}

/// Load a game export from a JSON file (native platforms)
#[cfg(not(target_arch = "wasm32"))]
pub fn load_game_export() -> Option<GameExport> {
    use rfd::FileDialog;
    use std::fs;

    if let Some(path) = FileDialog::new()
        .add_filter("Tsurust Replay", &["json"])
        .pick_file()
    {
        match fs::read_to_string(&path) {
            Ok(json) => {
                match serde_json::from_str(&json) {
                    Ok(export) => {
                        println!("Replay loaded from: {:?}", path);
                        Some(export)
                    }
                    Err(e) => {
                        eprintln!("Failed to parse replay file: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to read replay file: {}", e);
                None
            }
        }
    } else {
        None
    }
}

/// Save a game export to a JSON file (WASM/browser)
#[cfg(target_arch = "wasm32")]
pub fn save_game_export(export: &GameExport) {
    use wasm_bindgen::JsCast;
    use web_sys::{Blob, Url, HtmlAnchorElement};

    // Generate filename with timestamp
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("tsurust_replay_{}.json", timestamp);

    // Serialize to JSON
    let json = match serde_json::to_string_pretty(export) {
        Ok(json) => json,
        Err(e) => {
            web_sys::console::error_1(&format!("Failed to serialize game export: {}", e).into());
            return;
        }
    };

    // Get window and document
    let window = match web_sys::window() {
        Some(window) => window,
        None => {
            web_sys::console::error_1(&"No window found".into());
            return;
        }
    };

    let document = match window.document() {
        Some(doc) => doc,
        None => {
            web_sys::console::error_1(&"No document found".into());
            return;
        }
    };

    // Create blob
    let array = js_sys::Array::new();
    array.push(&wasm_bindgen::JsValue::from_str(&json));

    let blob = match Blob::new_with_str_sequence(&array) {
        Ok(blob) => blob,
        Err(e) => {
            web_sys::console::error_1(&format!("Failed to create blob: {:?}", e).into());
            return;
        }
    };

    // Create object URL
    let url = match Url::create_object_url_with_blob(&blob) {
        Ok(url) => url,
        Err(e) => {
            web_sys::console::error_1(&format!("Failed to create object URL: {:?}", e).into());
            return;
        }
    };

    // Create download link
    let anchor = match document.create_element("a") {
        Ok(element) => match element.dyn_into::<HtmlAnchorElement>() {
            Ok(anchor) => anchor,
            Err(e) => {
                web_sys::console::error_1(&format!("Failed to cast to anchor: {:?}", e).into());
                Url::revoke_object_url(&url).ok();
                return;
            }
        },
        Err(e) => {
            web_sys::console::error_1(&format!("Failed to create anchor: {:?}", e).into());
            Url::revoke_object_url(&url).ok();
            return;
        }
    };

    anchor.set_href(&url);
    anchor.set_download(&filename);
    anchor.click();

    // Clean up
    Url::revoke_object_url(&url).ok();
    web_sys::console::log_1(&format!("Replay download triggered: {}", filename).into());
}

/// Load a game export from a JSON file (WASM/browser)
/// Note: This triggers a file picker and the result is handled asynchronously via callback
#[cfg(target_arch = "wasm32")]
pub fn load_game_export<F>(callback: F)
where
    F: Fn(GameExport) + 'static,
{
    use wasm_bindgen::JsCast;
    use wasm_bindgen::closure::Closure;
    use web_sys::{HtmlInputElement, FileReader, Event};

    // Get window and document
    let window = match web_sys::window() {
        Some(window) => window,
        None => {
            web_sys::console::error_1(&"No window found".into());
            return;
        }
    };

    let document = match window.document() {
        Some(doc) => doc,
        None => {
            web_sys::console::error_1(&"No document found".into());
            return;
        }
    };

    // Create file input
    let input = match document.create_element("input") {
        Ok(element) => match element.dyn_into::<HtmlInputElement>() {
            Ok(input) => input,
            Err(e) => {
                web_sys::console::error_1(&format!("Failed to cast to input: {:?}", e).into());
                return;
            }
        },
        Err(e) => {
            web_sys::console::error_1(&format!("Failed to create input: {:?}", e).into());
            return;
        }
    };

    input.set_type("file");
    input.set_accept(".json");

    // Set up file reader callback
    let on_change = Closure::wrap(Box::new(move |event: Event| {
        let target = match event.target() {
            Some(target) => target,
            None => {
                web_sys::console::error_1(&"No event target".into());
                return;
            }
        };

        let input = match target.dyn_into::<HtmlInputElement>() {
            Ok(input) => input,
            Err(e) => {
                web_sys::console::error_1(&format!("Failed to cast to input: {:?}", e).into());
                return;
            }
        };

        let files = match input.files() {
            Some(files) => files,
            None => {
                web_sys::console::error_1(&"No files selected".into());
                return;
            }
        };

        let file = match files.get(0) {
            Some(file) => file,
            None => {
                web_sys::console::error_1(&"No file at index 0".into());
                return;
            }
        };

        let reader = match FileReader::new() {
            Ok(reader) => reader,
            Err(e) => {
                web_sys::console::error_1(&format!("Failed to create FileReader: {:?}", e).into());
                return;
            }
        };

        let reader_clone = reader.clone();
        let on_load = Closure::wrap(Box::new(move |_event: Event| {
            let result = match reader_clone.result() {
                Ok(result) => result,
                Err(e) => {
                    web_sys::console::error_1(&format!("Failed to get reader result: {:?}", e).into());
                    return;
                }
            };

            let text = match result.as_string() {
                Some(text) => text,
                None => {
                    web_sys::console::error_1(&"Result is not a string".into());
                    return;
                }
            };

            match serde_json::from_str::<GameExport>(&text) {
                Ok(export) => {
                    web_sys::console::log_1(&"Replay loaded successfully".into());
                    callback(export);
                }
                Err(e) => {
                    web_sys::console::error_1(&format!("Failed to parse replay file: {}", e).into());
                }
            }
        }) as Box<dyn FnMut(_)>);

        reader.set_onload(Some(on_load.as_ref().unchecked_ref()));
        on_load.forget(); // Keep closure alive

        if let Err(e) = reader.read_as_text(&file) {
            web_sys::console::error_1(&format!("Failed to read file: {:?}", e).into());
        }
    }) as Box<dyn FnMut(_)>);

    input.set_onchange(Some(on_change.as_ref().unchecked_ref()));
    input.click();
    on_change.forget(); // Keep closure alive
}
