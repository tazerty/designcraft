//! Evora build (patch CIC-04, class E5): the engine keeps nothing in the browser.
//!
//! The upstream web build persists nothing (`eframe` without `persistence`; `web-sys` without
//! `Storage`, `IdbFactory`, `CacheStorage` or the file system features, a CI guard keeps it so).
//! `evora:purge` is then a check: the engine origin's local and session storage, IndexedDB
//! databases and Cache Storage are read through the page's own objects (no `web-sys` storage
//! feature), anything another page left there is removed, and the purge is acknowledged only when a
//! second reading finds them empty.

use wasm_bindgen::{JsCast as _, JsValue};

fn get(v: &JsValue, key: &str) -> JsValue {
    js_sys::Reflect::get(v, &JsValue::from_str(key)).unwrap_or(JsValue::UNDEFINED)
}

fn call0(target: &JsValue, method: &str) -> Option<JsValue> {
    let f = get(target, method).dyn_into::<js_sys::Function>().ok()?;
    f.call0(target).ok()
}

fn call1(target: &JsValue, method: &str, arg: &JsValue) -> Option<JsValue> {
    let f = get(target, method).dyn_into::<js_sys::Function>().ok()?;
    f.call1(target, arg).ok()
}

async fn resolve(v: Option<JsValue>) -> Option<JsValue> {
    let p = v?.dyn_into::<js_sys::Promise>().ok()?;
    wasm_bindgen_futures::JsFuture::from(p).await.ok()
}

/// Items left in the origin's storage: (web storage entries, IndexedDB databases, caches).
async fn count(window: &JsValue) -> (u32, u32, u32) {
    let mut web = 0;
    for area in ["localStorage", "sessionStorage"] {
        // A sandboxed or blocked storage throws on access: nothing readable there.
        let s = js_sys::Reflect::get(window, &JsValue::from_str(area)).unwrap_or(JsValue::UNDEFINED);
        web += get(&s, "length").as_f64().unwrap_or(0.0) as u32;
    }
    let idb = get(window, "indexedDB");
    let dbs = if idb.is_object() { resolve(call0(&idb, "databases")).await } else { None };
    let dbs = dbs.map(|d| js_sys::Array::from(&d).length()).unwrap_or(0);
    let caches = get(window, "caches");
    let keys = if caches.is_object() { resolve(call0(&caches, "keys")).await } else { None };
    let keys = keys.map(|k| js_sys::Array::from(&k).length()).unwrap_or(0);
    (web, dbs, keys)
}

/// Check, empty what is found, check again: `true` when nothing is left.
pub async fn purge() -> bool {
    let Some(window) = web_sys::window().map(JsValue::from) else { return false };
    if count(&window).await == (0, 0, 0) {
        return true;
    }
    for area in ["localStorage", "sessionStorage"] {
        let s = js_sys::Reflect::get(&window, &JsValue::from_str(area)).unwrap_or(JsValue::UNDEFINED);
        if s.is_object() {
            let _ = call0(&s, "clear");
        }
    }
    let idb = get(&window, "indexedDB");
    if idb.is_object()
        && let Some(list) = resolve(call0(&idb, "databases")).await
    {
        for db in js_sys::Array::from(&list).iter() {
            let name = get(&db, "name");
            if name.as_string().is_some() {
                let _ = call1(&idb, "deleteDatabase", &name);
            }
        }
    }
    let caches = get(&window, "caches");
    if caches.is_object()
        && let Some(list) = resolve(call0(&caches, "keys")).await
    {
        for key in js_sys::Array::from(&list).iter() {
            let _ = resolve(call1(&caches, "delete", &key)).await;
        }
    }
    // A database deletion completes later: read again for a short while before answering.
    for _ in 0..PURGE_READS {
        if count(&window).await == (0, 0, 0) {
            return true;
        }
        pause(PURGE_PAUSE_MS).await;
    }
    false
}

/// Readings after the deletions, and the pause between two (instrument bounds of the check, not a
/// product setting: the host gives up on its own purge timeout).
const PURGE_READS: u32 = 20;
const PURGE_PAUSE_MS: i32 = 100;

async fn pause(ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        if let Some(w) = web_sys::window() {
            let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms);
        }
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}
