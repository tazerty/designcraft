//! Evora build (patch CIC-07, class E4): the native Evora host protocol bridge, EHP v1
//! (`apps/creative/HOST-PROTOCOL.md` in Evora), on the structure of the Evora bridge of our
//! photocraft fork (EVORA-LUM-13 and later), itself modelled on pdfcraft's (EVORA-COL-12).
//!
//! `?host=evora` (read by key and value, never by a substring search) turns it on. The parent page
//! is trusted only when its origin is listed in `evora-host.json`, served next to `index.html` and
//! written by Evora's provisioning: `{"parentOrigins": [...]}`, bare http(s) origins. The list never
//! comes from a URL parameter, a message or the referrer alone: when the browser records the parent
//! (`location.ancestorOrigins`), that origin must be listed; otherwise the referrer may only pick one
//! of the listed origins, or the single listed one is used.
//!
//! A message is examined only when `event.source` is the parent window AND `event.origin` is the
//! admitted origin, then checked in the order of HOST-PROTOCOL §5.2: plain object, `v === 1`, known
//! type, effective capability, exact envelope `{v, type, id, re, payload}`, ULID ids (`re` on a
//! response only), the payload's schema, binary fields (an `ArrayBuffer`, 1 byte up to the limit),
//! and `re` matching what the engine waits for. Anything else is ignored. Outgoing messages name the
//! admitted origin as `targetOrigin` (never `*`) and carry fresh ULIDs.
//!
//! The engine announces itself (`evora:ready`), waits for `evora:init` before it starts
//! (`web::start`), then acknowledges it once running, or answers `evora:failed {code}`. Requests that
//! need the app are queued and applied at the start of a frame (`docs`). Cicero keeps no browser
//! storage (`storage.memory`, guard CIC-04): `evora:purge` checks the engine origin's storage, empties
//! what another page could have left there, and is acknowledged only after that check (`storage`).
//! With `doc.saved-ack` effective, `evora:saved` decides the saved state (`docs`); a panic of the
//! engine is reported once as `evora:failed {code: "panic"}` (§4.4). Every file the engine writes
//! leaves as a message, never as a download: the host's document as `evora:save`, a recovery copy as
//! `evora:save {saveAs: true}`, anything else as `evora:export` (`export.drive`).

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use wasm_bindgen::{JsCast as _, JsValue, closure::Closure};

/// Build slug of Cicero (Evora's technical slug `layout`, D-P18 a) and protocol version (§4.4, §10).
const APP: &str = "layout";
const EHP: &str = "1.0";
/// What this engine honours and its smoke test exercises (§9.1). One document per editor (DC-9):
/// no `doc.multi`; no browser storage: `storage.memory`, never `storage.ns`.
pub const CAPABILITIES: [&str; 15] = [
    "doc.open",
    "doc.save",
    "doc.saved-ack",
    "doc.dirty",
    "doc.request-save",
    "export.drive",
    "pick.drive",
    "command",
    "command.catalog",
    "font.inject",
    "storage.memory",
    "storage.purge",
    "live.lang",
    "live.theme",
    "ui.focus-out",
];
/// The admitted parent origins, beside `index.html` (§2.3).
const CONFIG: &str = "evora-host.json";
const LANGS: [&str; 14] = ["ar", "de", "en", "es", "fr", "hi", "id", "it", "ja", "ko", "pt", "ru", "tr", "zh-Hans"];
const REASONS: [&str; 5] = ["autosave", "visibility", "close", "logout", "user"];
/// `ackCode` of §4.2: what `evora:saved {ok: false}` may carry in `error`.
pub const ACK_CODES: [&str; 16] = [
    "clean",
    "cancelled",
    "conflict",
    "too_large",
    "quota",
    "forbidden",
    "not_found",
    "invalid_content",
    "unsupported",
    "rate_limited",
    "read_only",
    "busy",
    "capability",
    "network",
    "server",
    "timeout",
];
/// The capabilities an `evora:init` may name: §4.2, `ui.focus-out` of amendment A-19, and the suite's
/// extensions `prefs.sync` and `command.catalog` (DT-S17), which a host offers once it serves them.
const CAPABILITY_NAMES: [&str; 21] = [
    "ui.focus-out",
    "prefs.sync",
    "command.catalog",
    "doc.open",
    "doc.save",
    "doc.saved-ack",
    "doc.dirty",
    "doc.multi",
    "doc.request-save",
    "export.drive",
    "media.need",
    "media.ranges",
    "pick.drive",
    "command",
    "storage.ns",
    "storage.memory",
    "storage.purge",
    "live.lang",
    "live.theme",
    "font.inject",
    "author",
];

/// `evora:init`, once checked: what the engine starts with.
pub struct Init {
    /// The message's id, acknowledged once the engine runs.
    pub id: String,
    /// The engine language the host chose (its E3 mapping); `None` (JSON `null`): English.
    pub engine_lang: Option<String>,
    pub dark: bool,
}

/// What the host asks of the running app, applied at the start of a frame (`docs`).
pub enum HostEvent {
    /// `evora:open` with bytes: a document to open under the host's `docRef`.
    Open { id: String, doc_ref: String, name: String, bytes: Vec<u8> },
    /// `evora:open {docRef, name, new: true}`: a blank document of the host.
    OpenNew { id: String, doc_ref: String, name: String },
    /// `evora:request-save`: the host's document (one per editor).
    RequestSave { id: String, doc_ref: Option<String> },
    /// `evora:theme`: dark or light.
    Theme { id: String, dark: bool },
    /// `evora:lang`: the engine language (`None`: English).
    Lang { id: String, engine_lang: Option<String> },
    /// `evora:command {command, params: {}}`: of the compiled list, or (with `command.catalog`
    /// effective) of the catalogue, `confirmed` for a destructive one.
    Command { id: String, command: String, confirmed: bool },
    /// `evora:saved` answering the `evora:save` `re`, with `doc.saved-ack` effective; `doc_ref`: the
    /// new document of a successful `saveAs` (a recovery copy).
    Saved { re: String, ok: bool },
    /// `evora:font {family, format, bytes}` (patch CIC-10): a font file to register.
    Font { id: String, family: String, bytes: Vec<u8> },
}

/// A save handed to the host, bound to its document at the end of the frame (`docs`).
pub struct Sent {
    /// The `evora:save` id its answer carries in `re`.
    pub id: String,
    /// The host's document written.
    pub doc_ref: String,
    /// `evora:save {saveAs: true}`: a recovery copy, a new document of the host.
    pub save_as: bool,
}

struct State {
    parent: web_sys::Window,
    origin: String,
    ready_id: String,
    initialized: bool,
    init: Option<Init>,
    /// Resolves the promise `wait_init` awaits.
    wake: Option<js_sys::Function>,
    /// The effective capabilities: what the host offers and this engine declares (§9).
    caps: Vec<String>,
    max_document: f64,
    /// `limits.maxExportBytes` of `evora:init`.
    max_export: f64,
    read_only: bool,
    /// Every id received from the host: a replayed one is ignored (§3).
    seen: HashSet<String>,
    events: Vec<HostEvent>,
    ctx: Option<egui::Context>,
    /// `evora:save` ids waiting for their `evora:saved`.
    pending_saves: Vec<String>,
    /// `evora:request-save` waiting for the save it asked for: (docRef, request id).
    expected_saves: Vec<(String, String)>,
    /// Saves sent since the app last took them (`docs`).
    sent: Vec<Sent>,
    /// `evora:export` ids waiting for their `evora:ack`.
    pending_exports: Vec<String>,
    /// The host's document open in the engine: the `docRef` of an export and of a recovery copy.
    doc_ref: Option<String>,
    /// Counters read by `diag.stats` (smoke): saves, exports, focus-outs, refused downloads.
    saves: u32,
    exports: u32,
    focus_outs: u32,
}

/// The connected bridge (a parent admitted by `evora-host.json`).
#[derive(Clone)]
pub struct Bridge(Rc<RefCell<State>>);

/// Read `evora-host.json`, find the admitted parent, listen, and announce `evora:ready`. `None`: no
/// admitted parent, the bridge stays mute.
pub async fn connect() -> Option<Bridge> {
    let (window, parent, origin) = admitted_parent().await?;
    install_panic_hook(&parent, &origin);
    let bridge = Bridge(Rc::new(RefCell::new(State {
        parent,
        origin,
        ready_id: String::new(),
        initialized: false,
        init: None,
        wake: None,
        caps: Vec::new(),
        max_document: 0.0,
        max_export: 0.0,
        read_only: false,
        seen: HashSet::new(),
        events: Vec::new(),
        ctx: None,
        pending_saves: Vec::new(),
        expected_saves: Vec::new(),
        sent: Vec::new(),
        pending_exports: Vec::new(),
        doc_ref: None,
        saves: 0,
        exports: 0,
        focus_outs: 0,
    })));
    let listener = bridge.clone();
    let on_message = Closure::<dyn FnMut(web_sys::MessageEvent)>::new(move |event: web_sys::MessageEvent| listener.receive(&event));
    if window.add_event_listener_with_callback("message", on_message.as_ref().unchecked_ref()).is_err() {
        return None;
    }
    // The listener lives as long as the page.
    on_message.forget();
    let capabilities = js_sys::Array::new();
    for c in CAPABILITIES {
        capabilities.push(&JsValue::from_str(c));
    }
    let payload = object(&[
        ("app", APP.into()),
        ("version", env!("CARGO_PKG_VERSION").into()),
        ("ehp", EHP.into()),
        ("capabilities", capabilities.into()),
    ])?;
    let id = bridge.send("evora:ready", None, &payload, None)?;
    bridge.0.borrow_mut().ready_id = id;
    log::info!("designcraft-web: Evora host bridge ready, waiting for evora:init");
    Some(bridge)
}

impl Bridge {
    /// Wait for a valid `evora:init` (the engine waits without limit, §4.5).
    pub async fn wait_init(&self) -> Init {
        loop {
            let received = self.0.borrow_mut().init.take();
            if let Some(init) = received {
                return init;
            }
            let state = self.0.clone();
            let promise = js_sys::Promise::new(&mut |resolve, _reject| {
                state.borrow_mut().wake = Some(resolve);
            });
            let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
        }
    }

    /// The running app's context, woken when a request arrives.
    pub fn attach(&self, ctx: egui::Context) {
        self.0.borrow_mut().ctx = Some(ctx);
    }

    /// Requests waiting for the app.
    pub fn take_events(&self) -> Vec<HostEvent> {
        std::mem::take(&mut self.0.borrow_mut().events)
    }

    /// Is `capability` in the effective set?
    pub fn has(&self, capability: &str) -> bool {
        let st = self.0.borrow();
        st.initialized && st.caps.iter().any(|c| c == capability)
    }

    /// The effective capabilities (`diag.stats`).
    pub fn effective(&self) -> Vec<String> {
        self.0.borrow().caps.clone()
    }

    /// `evora:ack {ok, code?}` answering `re`.
    pub fn ack(&self, re: &str, ok: bool, code: Option<&str>) {
        let mut fields: Vec<(&str, JsValue)> = vec![("ok", ok.into())];
        if let Some(code) = code {
            fields.push(("code", code.into()));
        }
        if let Some(payload) = object(&fields) {
            let _ = self.send("evora:ack", Some(re), &payload, None);
        }
    }

    /// `evora:ack {ok: true, result}` answering the `evora:command` `re` (§4.5).
    pub fn ack_result(&self, re: &str, result: &serde_json::Value) {
        let Ok(value) = js_sys::JSON::parse(&result.to_string()) else { return };
        if let Some(payload) = object(&[("ok", true.into()), ("result", value)]) {
            let _ = self.send("evora:ack", Some(re), &payload, None);
        }
    }

    /// `evora:commands` (`command.catalog`, DT-S17): the command catalogue. No answer; `false` when
    /// not sent.
    pub fn commands(&self, catalog: &serde_json::Value) -> bool {
        if !self.has("command.catalog") {
            return false;
        }
        let Ok(value) = js_sys::JSON::parse(&catalog.to_string()) else { return false };
        let Ok(payload) = value.dyn_into::<js_sys::Object>() else { return false };
        self.send("evora:commands", None, &payload, None).is_some()
    }

    /// `evora:failed {code}`: the engine could not start (§4.4).
    pub fn failed(&self, code: &str) {
        if let Some(payload) = object(&[("code", code.into())]) {
            let _ = self.send("evora:failed", None, &payload, None);
        }
    }

    /// `evora:focus-out {}` (amendment A-19 of §7.9): the user asked, at the keyboard, to leave the
    /// engine. Only with `ui.focus-out` effective; `false` when not sent.
    pub fn focus_out(&self) -> bool {
        if !self.has("ui.focus-out") {
            return false;
        }
        let Some(payload) = object(&[]) else { return false };
        let sent = self.send("evora:focus-out", None, &payload, None).is_some();
        if sent {
            let mut st = self.0.borrow_mut();
            st.focus_outs = st.focus_outs.saturating_add(1);
        }
        sent
    }

    /// Post `kind` to the admitted parent (the picker of `pick`); returns its id.
    pub(crate) fn post(&self, kind: &str, re: Option<&str>, payload: &js_sys::Object) -> Option<String> {
        self.send(kind, re, payload, None)
    }

    /// `evora:dirty {dirty, docRef}` (`doc.dirty`).
    pub fn dirty(&self, doc_ref: &str, dirty: bool) {
        if !self.has("doc.dirty") {
            return;
        }
        if let Some(payload) = object(&[("dirty", dirty.into()), ("docRef", doc_ref.into())]) {
            let _ = self.send("evora:dirty", None, &payload, None);
        }
    }

    /// The next save of `doc_ref` answers `evora:request-save` `request`.
    pub fn expect_save(&self, doc_ref: &str, request: &str) {
        self.0.borrow_mut().expected_saves.push((doc_ref.to_owned(), request.to_owned()));
    }

    /// Withdraw `request` if no save took it; `true` when it was still waiting.
    pub fn cancel_expected_save(&self, request: &str) -> bool {
        let mut st = self.0.borrow_mut();
        let before = st.expected_saves.len();
        st.expected_saves.retain(|(_, r)| r != request);
        st.expected_saves.len() != before
    }

    /// `evora:save {docRef, name, saveAs: false, bytes}` of the host's document, answering the
    /// waiting `evora:request-save` if any. `Err` holds the `evora:ack` code of the refusal.
    pub fn save(&self, name: &str, bytes: &[u8]) -> Result<(), &'static str> {
        let doc_ref = self.0.borrow().doc_ref.clone().ok_or("not_found")?;
        let re = self.take_expected_save(&doc_ref);
        self.send_save(&doc_ref, name, bytes, re.as_deref(), false)
    }

    /// The request a save of `doc_ref` answers, if one is waiting (taken).
    fn take_expected_save(&self, doc_ref: &str) -> Option<String> {
        let mut st = self.0.borrow_mut();
        let i = st.expected_saves.iter().position(|(d, _)| d == doc_ref)?;
        Some(st.expected_saves.remove(i).1)
    }

    /// `evora:save {docRef, name, saveAs: true, bytes}`: a copy that becomes a new document of the
    /// host (the recovery copies after a lost graphics device, CIC-14). Needs `doc.saved-ack`.
    pub fn save_as(&self, name: &str, bytes: &[u8]) -> Result<(), &'static str> {
        if !self.has("doc.saved-ack") {
            return Err("capability");
        }
        let doc_ref = self.0.borrow().doc_ref.clone().ok_or("not_found")?;
        self.send_save(&doc_ref, name, bytes, None, true)
    }

    fn send_save(&self, doc_ref: &str, name: &str, bytes: &[u8], re: Option<&str>, save_as: bool) -> Result<(), &'static str> {
        let max = {
            let st = self.0.borrow();
            if !st.initialized || !st.caps.iter().any(|c| c == "doc.save") {
                return Err("capability");
            }
            if st.read_only {
                return Err("read_only");
            }
            st.max_document
        };
        if bytes.is_empty() || bytes.len() as f64 > max {
            return Err("too_large");
        }
        let name = clean_name(name, "document.designcraft");
        let buffer = js_sys::Uint8Array::from(bytes).buffer();
        let payload = object(&[
            ("docRef", doc_ref.into()),
            ("name", name.as_str().into()),
            ("saveAs", save_as.into()),
            ("bytes", buffer.clone().into()),
        ])
        .ok_or("busy")?;
        let id = self.send("evora:save", re, &payload, Some(&buffer)).ok_or("busy")?;
        let mut st = self.0.borrow_mut();
        st.pending_saves.push(id.clone());
        st.sent.push(Sent { id, doc_ref: doc_ref.to_owned(), save_as });
        st.saves = st.saves.saturating_add(1);
        Ok(())
    }

    /// The saves sent since the last call.
    pub fn take_sent(&self) -> Vec<Sent> {
        std::mem::take(&mut self.0.borrow_mut().sent)
    }

    /// `evora:export {docRef?, name, mime, bytes}` (§4.4, §7.4): a file the engine produced, for
    /// Drive, never a download. The MIME type is a hint (the server computes its own). `Err`: the
    /// `evora:ack` code.
    pub fn export(&self, name: &str, bytes: &[u8]) -> Result<(), &'static str> {
        let (max, doc_ref) = {
            let st = self.0.borrow();
            if !st.initialized || !st.caps.iter().any(|c| c == "export.drive") {
                return Err("capability");
            }
            (st.max_export, st.doc_ref.clone())
        };
        if bytes.is_empty() || bytes.len() as f64 > max {
            return Err("too_large");
        }
        let name = clean_name(name, "export");
        let buffer = js_sys::Uint8Array::from(bytes).buffer();
        let mut fields: Vec<(&str, JsValue)> = Vec::new();
        if let Some(doc_ref) = doc_ref {
            fields.push(("docRef", doc_ref.into()));
        }
        fields.push(("name", name.as_str().into()));
        fields.push(("mime", crate::browser_policy::mime_for(&name).into()));
        fields.push(("bytes", buffer.clone().into()));
        let payload = object(&fields).ok_or("busy")?;
        let id = self.send("evora:export", None, &payload, Some(&buffer)).ok_or("busy")?;
        let mut st = self.0.borrow_mut();
        st.pending_exports.push(id);
        st.exports = st.exports.saturating_add(1);
        Ok(())
    }

    /// The host's document open in the engine (`docs`).
    pub fn set_doc_ref(&self, doc_ref: Option<String>) {
        self.0.borrow_mut().doc_ref = doc_ref;
    }

    pub fn doc_ref(&self) -> Option<String> {
        self.0.borrow().doc_ref.clone()
    }

    /// `diag.stats` counters: saves and exports sent, focus-outs, saves waiting for their answer.
    pub fn counters(&self) -> serde_json::Value {
        let st = self.0.borrow();
        serde_json::json!({"saves": st.saves, "exports": st.exports, "focusOuts": st.focus_outs, "pendingSaves": st.pending_saves.len()})
    }

    /// Post a message to the admitted parent origin; returns its `id`.
    fn send(&self, kind: &str, re: Option<&str>, payload: &js_sys::Object, transfer: Option<&JsValue>) -> Option<String> {
        let (message, id) = envelope(kind, re, payload)?;
        let st = self.0.borrow();
        let sent = match transfer {
            Some(t) => st.parent.post_message_with_transfer(&message, &st.origin, &js_sys::Array::of1(t)),
            None => st.parent.post_message(&message, &st.origin),
        };
        sent.ok().map(|()| id)
    }

    fn push(&self, event: HostEvent) {
        let ctx = {
            let mut st = self.0.borrow_mut();
            st.events.push(event);
            st.ctx.clone()
        };
        if let Some(ctx) = ctx {
            ctx.request_repaint();
        }
    }

    fn receive(&self, event: &web_sys::MessageEvent) {
        // §5.1: source AND origin.
        {
            let st = self.0.borrow();
            if event.origin() != st.origin || !event.source().is_some_and(|s| js_sys::Object::is(&s, &st.parent)) {
                return;
            }
        }
        // §5.2: a plain object, version 1, a known type, an effective capability, an exact envelope.
        let data = event.data();
        if !plain_object(&data) || get(&data, "v").as_f64() != Some(1.0) {
            return;
        }
        let Some(kind) = get(&data, "type").as_string() else { return };
        let capability = match kind.as_str() {
            "evora:init" => None,
            "evora:open" => Some("doc.open"),
            "evora:saved" => Some("doc.save"),
            "evora:request-save" => Some("doc.request-save"),
            "evora:lang" => Some("live.lang"),
            "evora:theme" => Some("live.theme"),
            "evora:purge" => Some("storage.purge"),
            "evora:command" => Some("command"),
            // A file answering the engine's picker (§4.3).
            "evora:file" => Some("pick.drive"),
            "evora:font" => Some("font.inject"),
            // The host's answer to an `evora:export` or the close of a picker: read only when it
            // answers one of them (`on_ack`); any other is ignored.
            "evora:ack" => None,
            // Unknown here, or a capability this engine doesn't declare: ignored (§4.1, §10).
            _ => return,
        };
        if let Some(c) = capability
            && !self.has(c)
        {
            return;
        }
        if !only_keys(&data, &["v", "type", "id", "re", "payload"]) {
            return;
        }
        let Some(id) = get(&data, "id").as_string().filter(|i| is_ulid(i)) else { return };
        let re = get(&data, "re");
        let re = if re.is_undefined() {
            None
        } else {
            match re.as_string().filter(|r| is_ulid(r)) {
                Some(r) => Some(r),
                None => return,
            }
        };
        // `re` answers something: on a response only (§3).
        if matches!(kind.as_str(), "evora:init" | "evora:saved" | "evora:ack" | "evora:file") != re.is_some() {
            return;
        }
        let payload = get(&data, "payload");
        if !plain_object(&payload) {
            return;
        }
        if !self.0.borrow_mut().seen.insert(id.clone()) {
            return;
        }
        match kind.as_str() {
            "evora:init" => self.on_init(&id, re.as_deref(), &payload),
            "evora:open" => self.on_open(&id, &payload),
            "evora:saved" => self.on_saved(re.as_deref(), &payload),
            "evora:request-save" => self.on_request_save(&id, &payload),
            "evora:lang" => self.on_lang(&id, &payload),
            "evora:theme" => self.on_theme(&id, &payload),
            "evora:purge" => self.on_purge(&id, &payload),
            "evora:command" => self.on_command(&id, &payload),
            "evora:ack" => self.on_ack(re.as_deref(), &payload),
            "evora:file" => self.on_file(re.as_deref(), &payload),
            "evora:font" => self.on_font(&id, &payload),
            _ => {}
        }
    }

    fn on_init(&self, id: &str, re: Option<&str>, p: &JsValue) {
        {
            let st = self.0.borrow();
            if st.initialized || re != Some(st.ready_id.as_str()) {
                return;
            }
        }
        let required = ["lang", "engineLang", "theme", "accent", "readOnly", "storageNs", "capabilities", "prefs", "limits"];
        let mut allowed = required.to_vec();
        allowed.push("displayName");
        // The suite's preference snapshot (`prefs.sync`): accepted, unused by this engine.
        allowed.push("prefsSnapshot");
        if !only_keys(p, &allowed) || required.iter().any(|k| get(p, k).is_undefined()) {
            return;
        }
        let lang_ok = get(p, "lang").as_string().is_some_and(|l| LANGS.contains(&l.as_str()));
        let engine_lang = engine_lang(&get(p, "engineLang"));
        let dark = match get(p, "theme").as_string().as_deref() {
            Some("dark") => true,
            Some("light") => false,
            _ => return,
        };
        let accent_ok = is_accent(&get(p, "accent"));
        let read_only = get(p, "readOnly").as_bool();
        let storage_ok = get(p, "storageNs").as_string().is_some_and(|ns| is_storage_ns(&ns));
        let display_name = get(p, "displayName");
        let display_ok = display_name.is_undefined() || display_name.as_string().is_some_and(|n| (1..=100).contains(&n.chars().count()));
        let caps = string_list(&get(p, "capabilities"));
        let caps_ok = caps.as_ref().is_some_and(|c| c.iter().all(|x| CAPABILITY_NAMES.contains(&x.as_str())));
        let prefs = get(p, "prefs");
        let prefs_ok = plain_object(&prefs)
            && js_sys::Object::values(prefs.unchecked_ref::<js_sys::Object>()).iter().all(|v| v.as_string().is_some() || v.as_f64().is_some() || v.as_bool().is_some());
        let limits = get(p, "limits");
        let max_document = positive_integer(&get(&limits, "maxDocumentBytes"));
        let max_export = positive_integer(&get(&limits, "maxExportBytes"));
        let limits_ok = plain_object(&limits) && only_keys(&limits, &["maxDocumentBytes", "maxExportBytes", "maxPrefsBytes"]);
        let snapshot_v = get(p, "prefsSnapshot");
        let snapshot_ok = snapshot_v.is_undefined() || snapshot_v.as_string().is_some();
        let (Some(engine_lang), Some(read_only), Some(caps), Some(max_document), Some(max_export)) = (engine_lang, read_only, caps, max_document, max_export)
        else {
            return;
        };
        if !(lang_ok && accent_ok && storage_ok && display_ok && caps_ok && prefs_ok && limits_ok && snapshot_ok) {
            return;
        }
        let wake = {
            let mut st = self.0.borrow_mut();
            st.caps = caps.into_iter().filter(|c| CAPABILITIES.contains(&c.as_str())).collect();
            st.max_document = max_document;
            st.max_export = max_export;
            st.read_only = read_only;
            st.initialized = true;
            st.init = Some(Init { id: id.to_owned(), engine_lang, dark });
            st.wake.take()
        };
        if let Some(wake) = wake {
            let _ = wake.call0(&JsValue::NULL);
        }
    }

    fn on_open(&self, id: &str, p: &JsValue) {
        if !only_keys(p, &["docRef", "name", "bytes", "url", "new"]) {
            return;
        }
        let Some(doc_ref) = get(p, "docRef").as_string().filter(|r| is_ulid(r)) else { return };
        let Some(name) = get(p, "name").as_string().filter(|n| is_name(n)) else { return };
        let given: Vec<&str> = ["bytes", "url", "new"].into_iter().filter(|k| !get(p, k).is_undefined()).collect();
        match given.as_slice() {
            // `url` needs `media.ranges`, which this engine doesn't declare.
            ["url"] => {
                if get(p, "url").as_string().is_some() {
                    self.ack(id, false, Some("capability"));
                }
                return;
            }
            // A blank document of the host.
            ["new"] => {
                if get(p, "new").as_bool() == Some(true) {
                    self.push(HostEvent::OpenNew { id: id.to_owned(), doc_ref, name });
                }
                return;
            }
            ["bytes"] => {}
            _ => return,
        }
        let bytes = get(p, "bytes");
        // An ArrayBuffer (never a SharedArrayBuffer or a typed view), 1 byte up to the limit.
        if !bytes.is_instance_of::<js_sys::ArrayBuffer>() {
            return;
        }
        let length = f64::from(bytes.unchecked_ref::<js_sys::ArrayBuffer>().byte_length());
        if length < 1.0 {
            return;
        }
        if length > self.0.borrow().max_document {
            self.ack(id, false, Some("too_large"));
            return;
        }
        let data = js_sys::Uint8Array::new(&bytes).to_vec();
        self.push(HostEvent::Open { id: id.to_owned(), doc_ref, name, bytes: data });
    }

    /// `evora:saved {ok, version?, docRef?, error?}` (§4.3): `ok: true` with a version and no error,
    /// `ok: false` with an error code and neither version nor docRef. With `doc.saved-ack`
    /// effective, the answer reaches the app; without it, the engine counted the document saved when
    /// it handed it over and the host keeps its own save state (§6.3).
    fn on_saved(&self, re: Option<&str>, p: &JsValue) {
        let Some(re) = re else { return };
        if !self.0.borrow().pending_saves.iter().any(|s| s == re) || !only_keys(p, &["ok", "version", "docRef", "error"]) {
            return;
        }
        let Some(ok) = get(p, "ok").as_bool() else { return };
        let version = get(p, "version");
        let doc_ref = get(p, "docRef");
        let error = get(p, "error");
        let valid = if ok {
            positive_integer(&version).is_some() && error.is_undefined() && (doc_ref.is_undefined() || doc_ref.as_string().is_some_and(|r| is_ulid(&r)))
        } else {
            error.as_string().is_some_and(|e| ACK_CODES.contains(&e.as_str())) && version.is_undefined() && doc_ref.is_undefined()
        };
        if !valid {
            return;
        }
        self.0.borrow_mut().pending_saves.retain(|s| s != re);
        if self.has("doc.saved-ack") {
            self.push(HostEvent::Saved { re: re.to_owned(), ok });
        }
    }

    /// `evora:ack {ok, code?}` answering an `evora:export` (§7.4): the export is settled; a refusal
    /// is logged (the host shows its own message). Answering an `evora:pick` (§7.6), it closes the
    /// picker (`pick`). Anything else is ignored.
    fn on_ack(&self, re: Option<&str>, p: &JsValue) {
        let Some(re) = re else { return };
        if !only_keys(p, &["ok", "code"]) {
            return;
        }
        let Some(ok) = get(p, "ok").as_bool() else { return };
        let code = get(p, "code");
        let code_ok = code.is_undefined() || code.as_string().is_some_and(|c| ACK_CODES.contains(&c.as_str()));
        if !code_ok || (!ok && code.is_undefined()) {
            return;
        }
        let export = self.0.borrow().pending_exports.iter().any(|e| e == re);
        if export {
            self.0.borrow_mut().pending_exports.retain(|e| e != re);
            if !ok {
                log::warn!("designcraft-web: the host refused an export ({})", code.as_string().unwrap_or_default());
            }
            return;
        }
        let ctx = self.0.borrow().ctx.clone();
        crate::evora::pick::settle(re, ok, ctx.as_ref());
    }

    /// `evora:file {path, name?, bytes | url | error}` answering the engine's picker (§4.3): kept by
    /// `pick` until the `evora:ack` that closes the exchange.
    fn on_file(&self, re: Option<&str>, p: &JsValue) {
        let Some(re) = re else { return };
        let max = self.0.borrow().max_document;
        crate::evora::pick::on_file(re, p, max);
    }

    /// `evora:font {family, format, weight?, style?, bytes}` (patch CIC-10, §4.3): a TrueType or
    /// OpenType file, an `ArrayBuffer` of 1 byte up to `maxDocumentBytes` (the host applies its own
    /// font limits before sending), registered at the start of a frame and acknowledged there.
    fn on_font(&self, id: &str, p: &JsValue) {
        if !only_keys(p, &["family", "format", "weight", "style", "bytes"]) {
            return;
        }
        let Some(family) = get(p, "family").as_string().filter(|f| is_family(f)) else { return };
        if !get(p, "format").as_string().is_some_and(|f| f == "ttf" || f == "otf") {
            return;
        }
        let weight = get(p, "weight");
        if !weight.is_undefined() && !(js_sys::Number::is_safe_integer(&weight) && weight.as_f64().is_some_and(|w| (1.0..=1000.0).contains(&w))) {
            return;
        }
        let style = get(p, "style");
        if !style.is_undefined() && !style.as_string().is_some_and(|s| s == "normal" || s == "italic") {
            return;
        }
        let bytes = get(p, "bytes");
        if !bytes.is_instance_of::<js_sys::ArrayBuffer>() {
            return;
        }
        let length = f64::from(bytes.unchecked_ref::<js_sys::ArrayBuffer>().byte_length());
        if length < 1.0 {
            return;
        }
        if length > self.0.borrow().max_document {
            self.ack(id, false, Some("too_large"));
            return;
        }
        let data = js_sys::Uint8Array::new(&bytes).to_vec();
        self.push(HostEvent::Font { id: id.to_owned(), family, bytes: data });
    }

    fn on_request_save(&self, id: &str, p: &JsValue) {
        if !only_keys(p, &["reason", "docRef"]) || !get(p, "reason").as_string().is_some_and(|r| REASONS.contains(&r.as_str())) {
            return;
        }
        let doc_ref = get(p, "docRef");
        let doc_ref = if doc_ref.is_undefined() {
            None
        } else {
            match doc_ref.as_string().filter(|r| is_ulid(r)) {
                Some(r) => Some(r),
                None => return,
            }
        };
        self.push(HostEvent::RequestSave { id: id.to_owned(), doc_ref });
    }

    fn on_lang(&self, id: &str, p: &JsValue) {
        if !only_keys(p, &["lang", "engineLang"]) || !get(p, "lang").as_string().is_some_and(|l| LANGS.contains(&l.as_str())) {
            return;
        }
        let Some(engine_lang) = engine_lang(&get(p, "engineLang")) else { return };
        self.push(HostEvent::Lang { id: id.to_owned(), engine_lang });
    }

    fn on_theme(&self, id: &str, p: &JsValue) {
        if !only_keys(p, &["theme", "accent"]) || !is_accent(&get(p, "accent")) {
            return;
        }
        let dark = match get(p, "theme").as_string().as_deref() {
            Some("dark") => true,
            Some("light") => false,
            _ => return,
        };
        self.push(HostEvent::Theme { id: id.to_owned(), dark });
    }

    /// `evora:command {command, params, confirmed?}` (§4.3, rule of 2026-10-27 for every command
    /// bridge of the suite): `params` must be `{}` (no free parameter); a command of the compiled
    /// list, or of the catalogue with `command.catalog` effective (`commands`). A well-formed command
    /// off both is refused with `capability`; a malformed message, or parameters given, is ignored.
    fn on_command(&self, id: &str, p: &JsValue) {
        let catalog = self.has("command.catalog");
        let keys: &[&str] = if catalog { &["command", "params", "confirmed"] } else { &["command", "params"] };
        if !only_keys(p, keys) {
            return;
        }
        let Some(command) = get(p, "command").as_string().filter(|c| is_command_name(c)) else { return };
        let params = get(p, "params");
        if !plain_object(&params) {
            return;
        }
        let confirmed = get(p, "confirmed");
        let Some(confirmed) = (if confirmed.is_undefined() { Some(false) } else { confirmed.as_bool() }) else { return };
        if js_sys::Object::keys(params.unchecked_ref::<js_sys::Object>()).length() != 0 {
            return;
        }
        if !crate::evora::commands::allowed(&command) && !catalog {
            self.ack(id, false, Some("capability"));
            return;
        }
        self.push(HostEvent::Command { id: id.to_owned(), command, confirmed });
    }

    /// `evora:purge {scope}` (HOST-PROTOCOL §7.7): this engine stores nothing (`storage.memory`,
    /// guard CIC-04); the purge checks the engine origin's storage, empties what it finds, and is
    /// acknowledged only after the check (`busy` when something stays).
    fn on_purge(&self, id: &str, p: &JsValue) {
        if !matches!(get(p, "scope").as_string().as_deref(), Some("all" | "current")) || !only_keys(p, &["scope"]) {
            return;
        }
        let bridge = self.clone();
        let id = id.to_owned();
        wasm_bindgen_futures::spawn_local(async move {
            if crate::evora::storage::purge().await {
                bridge.ack(&id, true, None);
            } else {
                bridge.ack(&id, false, Some("busy"));
            }
        });
    }
}

thread_local! {
    /// Where a panic is reported: the admitted parent window and its origin.
    static PANIC_TARGET: RefCell<Option<(web_sys::Window, String)>> = const { RefCell::new(None) };
}
/// A panic is reported once.
static PANIC_REPORTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// A panic of the engine reaches the host once, as `evora:failed {code: "panic"}` (§4.4), after the
/// hooks already installed (the browser console) have run.
fn install_panic_hook(parent: &web_sys::Window, origin: &str) {
    PANIC_TARGET.with(|t| *t.borrow_mut() = Some((parent.clone(), origin.to_owned())));
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        previous(info);
        if PANIC_REPORTED.swap(true, std::sync::atomic::Ordering::SeqCst) {
            return;
        }
        let _ = PANIC_TARGET.try_with(|t| {
            let Ok(target) = t.try_borrow() else { return };
            let Some((host_window, host_origin)) = target.as_ref() else { return };
            let Some(payload) = object(&[("code", "panic".into())]) else { return };
            if let Some((message, _)) = envelope("evora:failed", None, &payload) {
                let _ = host_window.post_message(&message, host_origin);
            }
        });
    }));
}

/// The parent window and its origin, when the page is framed by an origin `evora-host.json` admits.
async fn admitted_parent() -> Option<(web_sys::Window, web_sys::Window, String)> {
    let window = web_sys::window()?;
    let parent = window.parent().ok().flatten()?;
    if js_sys::Object::is(&parent, &window) {
        log::warn!("designcraft-web: ?host=evora but the page is not framed");
        return None;
    }
    let admitted = read_config(&window).await?;
    // `location.ancestorOrigins` (a DOMStringList; absent from web-sys and from some browsers).
    let location: JsValue = window.location().into();
    let ancestors = get(&location, "ancestorOrigins");
    let first_ancestor = (get(&ancestors, "length").as_f64().unwrap_or(0.0) >= 1.0)
        .then(|| js_sys::Reflect::get_u32(&ancestors, 0).ok().and_then(|o| o.as_string()))
        .flatten();
    let origin = match first_ancestor {
        Some(parent_origin) => {
            if !admitted.contains(&parent_origin) {
                log::warn!("designcraft-web: the parent origin is not admitted by {CONFIG}");
                return None;
            }
            parent_origin
        }
        None => {
            let referrer = window.document().map(|d| d.referrer()).unwrap_or_default();
            let from_referrer = web_sys::Url::new(&referrer).ok().map(|u| u.origin()).filter(|o| admitted.contains(o));
            match (from_referrer, admitted.as_slice()) {
                (Some(o), _) => o,
                (None, [only]) => only.clone(),
                _ => return None,
            }
        }
    };
    Some((window, parent, origin))
}

/// `evora-host.json`: `{ "parentOrigins": ["https://...", ...] }`, each a bare web origin. Empty,
/// missing or malformed: `None` (the bridge stays mute).
async fn read_config(window: &web_sys::Window) -> Option<Vec<String>> {
    let response = wasm_bindgen_futures::JsFuture::from(window.fetch_with_str(CONFIG)).await.ok()?;
    let response: web_sys::Response = response.dyn_into().ok()?;
    if !response.ok() {
        log::warn!("designcraft-web: no {CONFIG} next to the page: the Evora bridge stays mute");
        return None;
    }
    let text = wasm_bindgen_futures::JsFuture::from(response.text().ok()?).await.ok()?.as_string()?;
    let config = js_sys::JSON::parse(&text).ok()?;
    let list = get(&config, "parentOrigins");
    if !plain_object(&config) || !js_sys::Array::is_array(&list) {
        return None;
    }
    let origins: Vec<String> = js_sys::Array::from(&list).iter().filter_map(|o| o.as_string()).filter(|o| is_origin(o)).collect();
    if origins.is_empty() {
        log::warn!("designcraft-web: {CONFIG} admits no parent origin: the Evora bridge stays mute");
        return None;
    }
    Some(origins)
}

/// `http(s)://host[:port]`, nothing after.
fn is_origin(o: &str) -> bool {
    let rest = o.strip_prefix("https://").or_else(|| o.strip_prefix("http://"));
    rest.is_some_and(|r| !r.is_empty() && !r.contains(['/', '?', '#', '@', ' ', '\\']))
}

fn envelope(kind: &str, re: Option<&str>, payload: &js_sys::Object) -> Option<(js_sys::Object, String)> {
    let id = ulid();
    let mut fields: Vec<(&str, JsValue)> = vec![("v", 1.0.into()), ("type", kind.into()), ("id", id.as_str().into())];
    if let Some(re) = re {
        fields.push(("re", re.into()));
    }
    fields.push(("payload", payload.clone().into()));
    Some((object(&fields)?, id))
}

pub(crate) fn object(fields: &[(&str, JsValue)]) -> Option<js_sys::Object> {
    let m = js_sys::Object::new();
    for (key, value) in fields {
        js_sys::Reflect::set(&m, &JsValue::from_str(key), value).ok()?;
    }
    Some(m)
}

pub(crate) fn get(data: &JsValue, key: &str) -> JsValue {
    if !data.is_object() {
        return JsValue::UNDEFINED;
    }
    js_sys::Reflect::get(data, &JsValue::from_str(key)).unwrap_or(JsValue::UNDEFINED)
}

/// A plain object: not null, not an array, prototype `Object.prototype` (§5.2 step 2).
fn plain_object(v: &JsValue) -> bool {
    if !v.is_object() || js_sys::Array::is_array(v) {
        return false;
    }
    let plain = js_sys::Object::get_prototype_of(&js_sys::Object::new().into());
    js_sys::Object::is(&js_sys::Object::get_prototype_of(v), &plain)
}

/// No property outside `allowed` (`additionalProperties: false`).
pub(crate) fn only_keys(v: &JsValue, allowed: &[&str]) -> bool {
    if !v.is_object() {
        return false;
    }
    js_sys::Object::keys(v.unchecked_ref::<js_sys::Object>()).iter().all(|k| k.as_string().is_some_and(|k| allowed.contains(&k.as_str())))
}

/// A list of distinct strings.
fn string_list(v: &JsValue) -> Option<Vec<String>> {
    if !js_sys::Array::is_array(v) {
        return None;
    }
    let list: Vec<String> = js_sys::Array::from(v).iter().map(|x| x.as_string()).collect::<Option<Vec<_>>>()?;
    let mut unique = list.clone();
    unique.sort();
    unique.dedup();
    (unique.len() == list.len()).then_some(list)
}

/// A safe integer of at least 1.
fn positive_integer(v: &JsValue) -> Option<f64> {
    if !js_sys::Number::is_safe_integer(v) {
        return None;
    }
    v.as_f64().filter(|n| *n >= 1.0)
}

/// `engineLang` of §4.2: `null`, or `^[A-Za-z]{2,3}([-_][A-Za-z0-9]{2,8})*$`. Outer `None`: invalid.
fn engine_lang(v: &JsValue) -> Option<Option<String>> {
    if v.is_null() {
        return Some(None);
    }
    v.as_string().filter(|l| is_lang_tag(l)).map(Some)
}

fn is_lang_tag(tag: &str) -> bool {
    let mut parts = tag.split(['-', '_']);
    let first_ok = parts.next().is_some_and(|p| (2..=3).contains(&p.len()) && p.chars().all(|c| c.is_ascii_alphabetic()));
    first_ok && parts.all(|p| (2..=8).contains(&p.len()) && p.chars().all(|c| c.is_ascii_alphanumeric()))
}

/// `command` of §4.3: `^[a-z][A-Za-z0-9_.-]*$`.
fn is_command_name(c: &str) -> bool {
    let mut chars = c.chars();
    chars.next().is_some_and(|first| first.is_ascii_lowercase()) && chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '-'))
}

/// `accent` of §4.2: `#` and six hex digits.
fn is_accent(v: &JsValue) -> bool {
    v.as_string().is_some_and(|a| a.len() == 7 && a.starts_with('#') && a.chars().skip(1).all(|c| c.is_ascii_hexdigit()))
}

/// `storageNs` of §4.3: 64 lowercase hex digits (checked, unused: this engine stores nothing).
fn is_storage_ns(ns: &str) -> bool {
    ns.len() == 64 && ns.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

pub fn is_ulid(s: &str) -> bool {
    s.len() == 26 && s.bytes().all(|b| CROCKFORD.contains(&b))
}

/// A fresh ULID: 48 bits of milliseconds, then 80 random bits (§3: unique per sender).
fn ulid() -> String {
    let mut time = js_sys::Date::now() as u64;
    let mut out = [b'0'; 26];
    for slot in out.iter_mut().take(10).rev() {
        *slot = CROCKFORD[(time % 32) as usize];
        time /= 32;
    }
    for slot in out.iter_mut().skip(10) {
        *slot = CROCKFORD[((js_sys::Math::random() * 32.0) as usize) % 32];
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// `name` of §4.2: 1 to 255 characters, no `/`, `\` or control character.
pub fn is_name(n: &str) -> bool {
    (1..=255).contains(&n.chars().count()) && !n.chars().any(|c| c == '/' || c == '\\' || c.is_control())
}

/// `family` of `evora:font` (§4.3): not empty, no control character.
fn is_family(f: &str) -> bool {
    !f.trim().is_empty() && !f.chars().any(char::is_control)
}

/// A file name the host's schema accepts (`fallback` when nothing is left).
fn clean_name(n: &str, fallback: &str) -> String {
    let cleaned: String = n.chars().map(|c| if c == '/' || c == '\\' || c.is_control() { '_' } else { c }).take(255).collect();
    if cleaned.trim().is_empty() { fallback.to_string() } else { cleaned }
}
