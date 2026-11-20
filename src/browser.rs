//! Small wrappers around the browser features the shop uses. Each one fails
//! quietly, so the shop still works where storage or the clipboard is blocked.

use wasm_bindgen::{JsCast, JsValue, closure::Closure};

/// The part of the address after `#`, e.g. "housing".
pub fn hash() -> String {
    web_sys::window()
        .and_then(|w| w.location().hash().ok())
        .unwrap_or_default()
        .trim_start_matches('#')
        .to_string()
}

/// Sets the part of the address after `#`, as a new history entry, so Back
/// returns to the one before. An empty value takes the `#` off altogether,
/// without moving the page: a bare `#` would send the browser to the top.
pub fn set_hash(value: &str) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let location = window.location();
    if !value.is_empty() {
        let _ = location.set_hash(value);
        return;
    }
    // Nothing to take off. (The hash of "page/#" reads as "", so this looks
    // at the whole address.)
    if !location.href().unwrap_or_default().contains('#') {
        return;
    }
    let address = format!(
        "{}{}",
        location.pathname().unwrap_or_default(),
        location.search().unwrap_or_default()
    );
    if let Ok(history) = window.history() {
        let _ = history.push_state_with_url(&JsValue::NULL, "", Some(&address));
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

/// Reads a cookie this site has set.
pub fn cookie(name: &str) -> Option<String> {
    let cookies = html_document()?.cookie().ok()?;
    cookies.split(';').find_map(|pair| {
        let (key, value) = pair.trim().split_once('=')?;
        (key == name).then(|| value.to_string())
    })
}

/// Sets a cookie for this site that the browser deletes by itself after
/// `max_age` seconds (0 deletes it straight away). It's for the page's own
/// use: it goes along with requests to the site, but the server ignores it.
pub fn set_cookie(name: &str, value: &str, max_age: u32) {
    let Some(document) = html_document() else {
        return;
    };
    // Secure wherever the site is served over HTTPS. It's left off on plain
    // http://localhost (trunk serve), where some browsers would refuse it.
    let https = web_sys::window()
        .and_then(|w| w.location().protocol().ok())
        .is_some_and(|protocol| protocol == "https:");
    let secure = if https { "; Secure" } else { "" };
    let _ = document.set_cookie(&format!(
        "{name}={value}; Max-Age={max_age}; Path=/; SameSite=Lax{secure}"
    ));
}

/// Whether this browser keeps cookies for the site.
pub fn cookies_enabled() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    // Looked up dynamically, as web-sys doesn't have it.
    let navigator: JsValue = window.navigator().into();
    js_sys::Reflect::get(&navigator, &"cookieEnabled".into())
        .ok()
        .and_then(|enabled| enabled.as_bool())
        .unwrap_or(false)
}

/// The time now, in milliseconds since 1970.
pub fn now() -> f64 {
    js_sys::Date::now()
}

fn html_document() -> Option<web_sys::HtmlDocument> {
    web_sys::window()?.document()?.dyn_into().ok()
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

/// Moves keyboard focus to an element without scrolling to it, so the next
/// Tab carries on from there. The element needs a tabindex to take focus.
pub fn focus_quietly(id: &str) {
    let Some(element) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id(id))
        .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
    else {
        return;
    };
    let options = web_sys::FocusOptions::new();
    options.set_prevent_scroll(true);
    let _ = element.focus_with_options(&options);
}

/// Whether the page is scrolled all the way down, give or take `slack` pixels.
pub fn at_bottom(slack: f64) -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    let Some(root) = window.document().and_then(|d| d.document_element()) else {
        return false;
    };
    let window_height = window
        .inner_height()
        .ok()
        .and_then(|h| h.as_f64())
        .unwrap_or(0.0);
    let scrolled = window.scroll_y().unwrap_or(0.0);
    scrolled + window_height >= f64::from(root.scroll_height()) - slack
}

/// Calls `changed` whenever the first element matching `selector` changes
/// size, and once when it starts (after the page is laid out). It keeps
/// watching for as long as the returned `SizeWatch` is kept.
pub fn watch_size(selector: &str, changed: impl Fn() + 'static) -> Option<SizeWatch> {
    let element = web_sys::window()?
        .document()?
        .query_selector(selector)
        .ok()??;
    let changed = Closure::<dyn Fn()>::new(changed);
    let observer = web_sys::ResizeObserver::new(changed.as_ref().unchecked_ref()).ok()?;
    observer.observe(&element);
    Some(SizeWatch {
        observer,
        _changed: changed,
    })
}

/// Stops watching when dropped. See `watch_size`.
pub struct SizeWatch {
    observer: web_sys::ResizeObserver,
    _changed: Closure<dyn Fn()>,
}

impl Drop for SizeWatch {
    fn drop(&mut self) {
        self.observer.disconnect();
    }
}

/// Where the first element matching `selector` is, relative to the window.
pub fn rect(selector: &str) -> Option<web_sys::DomRect> {
    let element = web_sys::window()?
        .document()?
        .query_selector(selector)
        .ok()??;
    Some(element.get_bounding_client_rect())
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
