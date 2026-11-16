//! Small wrappers around the browser features the shop uses. Each one fails
//! quietly, so the shop still works where storage or the clipboard is blocked.

use wasm_bindgen::{JsCast, JsValue};

/// The part of the address after `#`, e.g. "housing".
pub fn hash() -> String {
    web_sys::window()
        .and_then(|w| w.location().hash().ok())
        .unwrap_or_default()
        .trim_start_matches('#')
        .to_string()
}

pub fn set_hash(value: &str) {
    if let Some(window) = web_sys::window() {
        let _ = window.location().set_hash(value);
    }
}

pub fn set_title(title: &str) {
    if let Some(document) = web_sys::window().and_then(|w| w.document()) {
        document.set_title(title);
    }
}

/// Reads something saved in this browser.
pub fn load(key: &str) -> Option<String> {
    web_sys::window()?
        .local_storage()
        .ok()??
        .get_item(key)
        .ok()?
}

/// Saves something in this browser only.
pub fn save(key: &str, value: &str) {
    if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = storage.set_item(key, value);
    }
}

/// Copies text to the clipboard. Returns false if the browser refused, so the
/// caller can show the text for copying by hand.
pub async fn copy(text: &str) -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    // Looked up dynamically so it works whether or not the browser has it.
    let navigator: JsValue = window.navigator().into();
    let Ok(clipboard) = js_sys::Reflect::get(&navigator, &"clipboard".into()) else {
        return false;
    };
    if clipboard.is_undefined() || clipboard.is_null() {
        return false;
    }
    let Some(write_text) = js_sys::Reflect::get(&clipboard, &"writeText".into())
        .ok()
        .and_then(|f| f.dyn_into::<js_sys::Function>().ok())
    else {
        return false;
    };
    let Ok(promise) = write_text.call1(&clipboard, &JsValue::from_str(text)) else {
        return false;
    };
    match promise.dyn_into::<js_sys::Promise>() {
        Ok(promise) => wasm_bindgen_futures::JsFuture::from(promise).await.is_ok(),
        Err(_) => false,
    }
}

/// Adds a `<script src=...>` to the page once.
pub fn load_script(src: &str) {
    let Some(document) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    let id = "external-script";
    if document.get_element_by_id(id).is_some() {
        return;
    }
    if let Ok(script) = document.create_element("script") {
        script.set_id(id);
        let _ = script.set_attribute("src", src);
        let _ = script.set_attribute("async", "");
        if let Some(head) = document.head() {
            let _ = head.append_child(&script);
        }
    }
}

/// Smoothly scrolls an element into view (instantly if the visitor prefers
/// reduced motion).
pub fn scroll_to(id: &str) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(element) = window.document().and_then(|d| d.get_element_by_id(id)) else {
        return;
    };
    let options = web_sys::ScrollIntoViewOptions::new();
    options.set_block(web_sys::ScrollLogicalPosition::Start);
    options.set_behavior(if prefers_reduced_motion() {
        web_sys::ScrollBehavior::Instant
    } else {
        web_sys::ScrollBehavior::Smooth
    });
    element.scroll_into_view_with_scroll_into_view_options(&options);
}

pub fn prefers_reduced_motion() -> bool {
    web_sys::window()
        .and_then(|w| {
            w.match_media("(prefers-reduced-motion: reduce)")
                .ok()
                .flatten()
        })
        .is_some_and(|query| query.matches())
}
