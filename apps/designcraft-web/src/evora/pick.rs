//! Evora build (patch CIC-08, class E4): the engine's file pickers become Evora Drive's (capability
//! `pick.drive`, Evora's HOST-PROTOCOL §4.4, §7.6).
//!
//! Under the bridge, every file the engine asks to read comes from the host, never from the
//! browser's own picker: File › Place, Load Swatches, Load Styles and the shortcut sets send
//! `evora:pick {purpose: "import"}` with what they accept. The host answers with one `evora:file` per
//! chosen file (its bytes, under `maxDocumentBytes`), then closes the exchange with
//! `evora:ack {ok: true}`; the file then reaches the engine through its inbox
//! ([`designcraft_ui_egui::Services::inbox`]) with the request captured when the picker opened, as
//! from the browser's picker. `evora:ack {ok: false}` answers the picker with nothing. File › Open
//! belongs to the host (one document per editor, DC-9): a picker for a document is never sent.
//! Without `pick.drive` effective, a picker answers nothing. One exchange at a time.

use std::cell::RefCell;

use designcraft_ui_egui::{ImportRequest, ImportedFile, Inbox};
use wasm_bindgen::{JsCast as _, JsValue};

use crate::evora::ehp::{Bridge, get, is_name, object, only_keys};

/// What File › Place accepts (the extensions of the browser picker of the upstream web shell, the
/// engine's own documents left to the host).
const PLACE: [&str; 17] = [
    ".png", ".jpg", ".jpeg", ".gif", ".webp", ".tif", ".tiff", ".bmp", ".psd", ".svg", ".pdf", ".ai", ".txt", ".docx", ".rtf", ".md", ".xlsx",
];
/// `error` of `evora:file` (§4.3).
const FILE_ERRORS: [&str; 4] = ["not_found", "forbidden", "too_large", "unsupported"];

/// The picker in progress: the `evora:pick` id, the request it answers, the files received so far.
struct Pick {
    id: String,
    request: ImportRequest,
    files: Vec<(String, Vec<u8>)>,
    inbox: Inbox,
}

thread_local! {
    static PICK: RefCell<Option<Pick>> = const { RefCell::new(None) };
}

/// What a reader accepts, by its purpose; `None`: no picker under the host.
fn accept_for(purpose: &str) -> Option<Vec<&'static str>> {
    Some(match purpose {
        "place" => PLACE.to_vec(),
        "swatches" => vec![".ase"],
        "shortcuts" => vec![".txt"],
        p if p.starts_with("styles") => vec![".designcraft", ".idml"],
        // File › Open, a drop and anything unknown: the host's (one document per editor).
        _ => return None,
    })
}

/// `Services::open_async` under the bridge: `evora:pick`, or nothing when the host has no picker,
/// one is already in progress, or the purpose is the host's.
pub fn open(bridge: &Bridge, inbox: &Inbox, request: ImportRequest) {
    if !bridge.has("pick.drive") {
        log::warn!("designcraft-web: the host offers no file picker (pick.drive): nothing to read");
        return;
    }
    let Some(accept) = accept_for(&request.purpose) else {
        log::info!("designcraft-web: a picker for `{}` belongs to the host", request.purpose);
        return;
    };
    if PICK.with(|p| p.borrow().is_some()) {
        return;
    }
    let list = js_sys::Array::new();
    for a in accept {
        list.push(&JsValue::from_str(a));
    }
    let payload = object(&[("purpose", "import".into()), ("accept", list.into()), ("multiple", false.into())]);
    let Some(id) = payload.and_then(|p| bridge.post("evora:pick", None, &p)) else { return };
    PICK.with(|p| *p.borrow_mut() = Some(Pick { id, request, files: Vec::new(), inbox: inbox.clone() }));
}

/// `evora:file {path, name?, bytes | url | error}` answering the picker `re` (§4.3): a file's bytes
/// (an `ArrayBuffer` of 1 byte up to `max_bytes`) are kept for the closing acknowledgement; an error
/// or an URL (`media.ranges` is not declared) is logged and skipped. A malformed message is ignored.
pub fn on_file(re: &str, p: &JsValue, max_bytes: f64) {
    if !only_keys(p, &["path", "name", "bytes", "url", "error"]) {
        return;
    }
    let Some(path) = get(p, "path").as_string().filter(|s| is_path(s)) else { return };
    let name = get(p, "name");
    let name = if name.is_undefined() {
        path.rsplit('/').next().unwrap_or_default().to_string()
    } else {
        match name.as_string().filter(|n| is_name(n)) {
            Some(n) => n,
            None => return,
        }
    };
    let given: Vec<&str> = ["bytes", "url", "error"].into_iter().filter(|k| !get(p, k).is_undefined()).collect();
    let file = match given.as_slice() {
        ["bytes"] => {
            let bytes = get(p, "bytes");
            if !bytes.is_instance_of::<js_sys::ArrayBuffer>() {
                return;
            }
            let length = f64::from(bytes.unchecked_ref::<js_sys::ArrayBuffer>().byte_length());
            if length < 1.0 || length > max_bytes {
                log::warn!("designcraft-web: a picked file is empty or over the document limit: skipped");
                return;
            }
            Some(js_sys::Uint8Array::new(&bytes).to_vec())
        }
        ["error"] => {
            if !get(p, "error").as_string().is_some_and(|e| FILE_ERRORS.contains(&e.as_str())) {
                return;
            }
            None
        }
        ["url"] => {
            if get(p, "url").as_string().is_none() {
                return;
            }
            None
        }
        _ => return,
    };
    PICK.with(|cell| {
        let mut cell = cell.borrow_mut();
        let Some(pick) = cell.as_mut().filter(|k| k.id == re) else { return };
        match file {
            Some(bytes) if !name.is_empty() => pick.files.push((name, bytes)),
            _ => log::warn!("designcraft-web: the host could not hand over a picked file: skipped"),
        }
    });
}

/// `evora:ack` closing the picker `re` (checked by the bridge): `ok` hands the first file received to
/// the engine's inbox, anything else answers it with nothing. Ignored when `re` is not the picker's.
pub fn settle(re: &str, ok: bool, ctx: Option<&egui::Context>) {
    let pick = PICK.with(|cell| {
        let mut cell = cell.borrow_mut();
        if cell.as_ref().is_some_and(|k| k.id == re) { cell.take() } else { None }
    });
    let Some(pick) = pick else { return };
    if !ok {
        return;
    }
    let Some((name, bytes)) = pick.files.into_iter().next() else { return };
    pick.inbox.lock().unwrap_or_else(std::sync::PoisonError::into_inner).push(ImportedFile { request: pick.request, name, bytes });
    if let Some(ctx) = ctx {
        ctx.request_repaint();
    }
}

/// Is a picker waiting for the host (`diag.stats`)?
pub fn pending() -> bool {
    PICK.with(|p| p.borrow().is_some())
}

/// `path` of §4.2: `/`, no control character, no `..` segment.
fn is_path(p: &str) -> bool {
    p.starts_with('/') && !p.chars().any(char::is_control) && !p.split('/').any(|s| s == "..")
}
