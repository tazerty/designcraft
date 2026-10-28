//! Evora build (patch CIC-09, class E4): the commands the host may run through `evora:command`
//! (capability `command`, Evora's HOST-PROTOCOL §4.3) and the command catalogue (`command.catalog`,
//! DT-S17 of the suite: Evora runs the software's commands on request).
//!
//! Rule of 2026-10-27 for every command bridge of the suite (white list, no free parameter):
//! - the catalogue is the set of live menu items of this build, MINUS the shell's namespace,
//!   excluded by construction and not by enumeration: every `file.*`, `app.*`, `help.*`, `prefs.*`
//!   and `window.*` item is refused, SAVE the exports and imports that pass through the host
//!   (`evora:export`, `evora:pick`), admitted ONE BY ONE in [`ADMITTED`]; items the host build hides
//!   ([`crate::evora::masks`]) and items with fixed parameters are left out;
//! - `params` must be `{}` (no item of the catalogue declares a parameter schema): checked by the
//!   bridge before anything runs; a command runs as its menu item runs (`menus::activate` with no
//!   parameter), its dialog opening in the engine;
//! - a command that takes content or a setting of the member out (deletions, cuts, clears,
//!   flattenings, resets), or whose effect Edit › Undo does not take back, is destructive
//!   ([`DESTRUCTIVE`], rule of 2026-10-27 at 20 h 45 for every command bridge of the suite): refused
//!   without `confirmed: true` (`forbidden`). The smoke test proves the list on the catalogue the build
//!   delivers (`commands.json`): every command whose id or label carries a removal marker is
//!   destructive, save an exception it justifies one by one. The engine's `noundo` commands are NOT
//!   destructive by that alone: most of them change no document (selection, copy, navigation, Undo
//!   and Redo themselves).
//!
//! The compiled list (capability `command` alone) keeps the exports through the host and the
//! diagnostics the smoke test reads.

use designcraft_ui_egui::DesignApp;
use designcraft_ui_egui::menus::{self, Item};
use serde_json::{Value, json};

/// The shell's namespace, refused by construction (rule of 2026-10-27).
const SHELL_PREFIXES: [&str; 5] = ["file.", "app.", "help.", "prefs.", "window."];
/// Exports and imports that pass through the host, admitted one by one: every file they write leaves
/// as `evora:export`, every file they read comes from `evora:pick`.
pub const ADMITTED: [&str; 12] = [
    "app.exportPdf",
    "app.exportInteractivePdf",
    "app.exportPng",
    "app.exportIdml",
    "app.exportEpub",
    "app.exportFixedEpub",
    "app.exportHtml",
    "app.exportPptx",
    "app.exportXml",
    "app.exportText",
    "app.placeDialog",
    "app.loadSwatches",
];
/// The compiled list: exports through the host that write at once (no dialog), and the picker.
const ENGINE_COMMANDS: [&str; 3] = ["app.exportInteractivePdf", "app.exportPng", "app.placeDialog"];
/// Diagnostics answered by the bridge (smoke): what the engine holds (`diag.stats`), the catalogue,
/// the items the rule refuses, the masked and the destructive ones (`diag.menus`); the interface Evora
/// designed (`diag.ui`: theme, workspaces, menus, shortcuts, tool) and the strings the tables show in the
/// current language (`diag.labels`), EVORA-CIC-02, 11, 12, 13, 19 and 22.
const DIAGNOSTICS: [&str; 4] = ["diag.stats", "diag.menus", "diag.ui", "diag.labels"];

/// FNV-1a 64 (offset basis and prime): the catalogue's fingerprint, an identity of the build's
/// commands, not a secret.
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Is `command` on the compiled list?
pub fn allowed(command: &str) -> bool {
    ENGINE_COMMANDS.contains(&command) || DIAGNOSTICS.contains(&command)
}

/// Is `command` a diagnostic answered by the bridge?
pub fn is_diagnostic(command: &str) -> bool {
    DIAGNOSTICS.contains(&command)
}

/// The catalogue admits `id` by its name alone (shell namespace refused, admitted exports kept).
pub fn admitted_name(id: &str) -> bool {
    ADMITTED.contains(&id) || !SHELL_PREFIXES.iter().any(|p| id.starts_with(p))
}

/// Commands that take content or a setting of the member out, or whose effect Edit › Undo does not
/// take back: the bridge refuses them without `confirmed: true`.
pub const DESTRUCTIVE: &[&str] = &[
    // The shell's namespace, refused by construction: kept for an admission or an upstream version
    // that would bring one in (revert to the saved state, close).
    "file.revert",
    "file.close",
    "file.closeAll",
    "file.closeOthers",
    "file.closeAllOthers",
    // Cut and Clear: the selection leaves the document.
    "edit.cut",
    "edit.clear",
    // Clear Transformations: the member's rotation, scale and shear are reset.
    "transform.clear",
    // Removing a flex layout: the member's flex layout is taken out.
    "flex.remove",
    // Table deletions: a row, a column, the whole table.
    "table.deleteRow",
    "table.deleteColumn",
    "table.delete",
    // Tracked changes: accepting all makes every tracked deletion final and drops the review;
    // rejecting all takes every tracked insertion out.
    "changes.acceptAll",
    "changes.rejectAll",
    // Flattenings: the text becomes paths (its words are gone), the table becomes text (its cells,
    // rows and styles are gone), the variable becomes fixed text.
    "type.createOutlines",
    "table.convertToText",
    "variables.convertToText",
];

/// A command of [`DESTRUCTIVE`]: refused without `confirmed: true`.
pub fn destructive(id: &str) -> bool {
    DESTRUCTIVE.contains(&id)
}

/// The live menu items of the build, once each, in menu order: (id, menu path, label), with no fixed
/// parameter and not hidden by the host build.
fn live_items(app: &DesignApp) -> Vec<(String, Vec<String>, String)> {
    fn walk(app: &DesignApp, items: &[Item], path: &mut Vec<String>, out: &mut Vec<(String, Vec<String>, String)>) {
        for it in items {
            match it {
                Item::Sub(name, children) => {
                    path.push(name.clone());
                    walk(app, children, path, out);
                    path.pop();
                }
                Item::Cmd { label, id, params, .. } => {
                    if !params.is_null() || id.is_empty() || out.iter().any(|(i, _, _)| i == id) {
                        continue;
                    }
                    // Hidden or refused by the host filter (`masks`): never in the catalogue, whatever
                    // its namespace (the filter would answer it with nothing, an `ok` that did nothing).
                    if crate::evora::masks::hidden(id) || crate::evora::masks::refuses(id) || !menus::shown(app, id) {
                        continue;
                    }
                    out.push((id.clone(), path.clone(), label.clone()));
                }
                Item::Sep => {}
            }
        }
    }
    let mut out = Vec::new();
    for (menu, entries) in menus::menu_tree() {
        let mut path = vec![menu.to_string()];
        walk(app, &entries, &mut path, &mut out);
    }
    out
}

/// `evora:commands`: the catalogue in the UI language. `hash`: FNV-1a 64 of each id followed by its
/// destructive flag, the same in every language and state, which the host compares with the one it
/// provisioned.
pub fn catalog(app: &DesignApp) -> Value {
    let lang = app.ui.language.clone();
    let mut hash = FNV_OFFSET;
    let mut commands: Vec<Value> = Vec::new();
    for (id, path, label) in live_items(app) {
        if !admitted_name(&id) {
            continue;
        }
        let destructive = destructive(&id);
        for b in id.bytes().chain(std::iter::once(if destructive { b'!' } else { b'\n' })) {
            hash = (hash ^ u64::from(b)).wrapping_mul(FNV_PRIME);
        }
        let path: Vec<&str> = path.iter().map(|p| designcraft_ui_egui::i18n::tr(&lang, p)).collect();
        commands.push(json!({
            "id": id,
            "label": designcraft_ui_egui::i18n::tr(&lang, &label),
            "path": path,
            "enabled": menus::menu_enabled(app, &id),
            "destructive": destructive,
        }));
    }
    json!({
        "lang": if lang.is_empty() { "en".to_string() } else { lang },
        "count": commands.len(),
        "hash": format!("{hash:016x}"),
        "params": {"type": "object", "maxProperties": 0},
        "commands": commands
    })
}

/// Is `command` in the catalogue (a live, admitted menu item the host build shows)?
pub fn in_catalog(app: &DesignApp, command: &str) -> bool {
    admitted_name(command) && live_items(app).iter().any(|(id, _, _)| id == command)
}

/// Run a command as its menu item runs it (its dialog opens in the engine). `Err`: the `evora:ack`
/// code of the refusal. `catalog`: `command.catalog` is effective.
pub fn run(app: &mut DesignApp, command: &str, confirmed: bool, catalog: bool) -> Result<(), &'static str> {
    let known = if catalog { in_catalog(app, command) || ENGINE_COMMANDS.contains(&command) } else { ENGINE_COMMANDS.contains(&command) };
    if is_diagnostic(command) || !known || crate::evora::masks::refuses(command) {
        return Err("capability");
    }
    if destructive(command) && !confirmed {
        return Err("forbidden");
    }
    if app.session.active().is_none() && command.starts_with("app.") {
        return Err("not_found");
    }
    if !menus::menu_enabled(app, command) {
        return Err("busy");
    }
    menus::activate(app, command, &Value::Null);
    Ok(())
}

/// `diag.menus`: the catalogue's ids, the live items refused by the rule, and the items the host
/// build hides that the menus hold.
pub fn diag_menus(app: &DesignApp) -> Value {
    let live = live_items(app);
    let catalog: Vec<&str> = live.iter().filter(|(id, _, _)| admitted_name(id)).map(|(id, _, _)| id.as_str()).collect();
    let refused: Vec<&str> = live.iter().filter(|(id, _, _)| !admitted_name(id)).map(|(id, _, _)| id.as_str()).collect();
    let destructive: Vec<&str> = catalog.iter().copied().filter(|id| destructive(id)).collect();
    let mut masked: Vec<String> = Vec::new();
    for (_, entries) in menus::menu_tree() {
        collect_masked(&entries, &mut masked);
    }
    json!({"catalog": catalog, "refused": refused, "masked": masked, "destructive": destructive})
}

fn collect_masked(items: &[Item], out: &mut Vec<String>) {
    for it in items {
        match it {
            Item::Sub(_, children) => collect_masked(children, out),
            Item::Cmd { id, .. } if crate::evora::masks::hidden(id) && !out.contains(id) => out.push(id.clone()),
            _ => {}
        }
    }
}

/// `diag.stats`: what the engine runs and holds (the smoke test's view of the engine).
pub fn stats(app: &DesignApp, doc_ref: Option<&str>, uid: Option<u64>, counters: Value, fonts: &[String]) -> Value {
    let docs = app.session.documents();
    let host = uid.and_then(|u| docs.iter().find(|d| d.uid == u));
    json!({
        "version": env!("CARGO_PKG_VERSION"),
        "documents": docs.len(),
        "docRef": doc_ref,
        "dirty": host.map(|d| d.is_dirty()),
        "revision": host.map(|d| d.revision),
        "pages": host.map(|d| d.doc.page_count()),
        "title": host.map(|d| d.doc.title.clone()),
        "language": app.ui.language,
        "dialog": app.ui.dialog.as_ref().map(|d| d.id.clone()),
        "pickPending": crate::evora::pick::pending(),
        "fonts": fonts,
        "bridge": counters,
        "wasmMemoryBytes": wasm_memory_bytes(),
    })
}

/// Size of the WebAssembly memory, in bytes (0 when unreadable).
fn wasm_memory_bytes() -> f64 {
    let read = |v: &wasm_bindgen::JsValue, key: &str| js_sys::Reflect::get(v, &wasm_bindgen::JsValue::from_str(key)).unwrap_or(wasm_bindgen::JsValue::UNDEFINED);
    read(&read(&wasm_bindgen::memory(), "buffer"), "byteLength").as_f64().unwrap_or(0.0)
}

/// `diag.ui` (EVORA-CIC-02, CIC-12, CIC-13, CIC-22): the interface Evora designed, as the engine holds
/// it: brightness and theme tokens, workspaces, the menus in order (and the dropped items left), the
/// shortcuts in effect, the current tool.
pub fn diag_ui(app: &DesignApp) -> Value {
    use designcraft_ui_egui::theme::{Brightness, Tokens};
    let t = Tokens::for_brightness(app.ui.brightness);
    let hex = |c: egui::Color32| format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b());
    let tree = menus::menu_tree();
    let dropped: usize = tree.iter().map(|(_, entries)| count_dropped(entries)).sum();
    json!({
        "brightness": app.ui.brightness.id(),
        "levels": Brightness::ALL.iter().map(|b| b.id()).collect::<Vec<_>>(),
        "tokens": {
            "appBar": hex(t.app_bar),
            "panel": hex(t.panel),
            "panelDarker": hex(t.panel_darker),
            "pasteboard": hex(t.pasteboard),
            "text": hex(t.text),
            "accent": hex(t.accent),
            "accentStrong": hex(t.accent_strong),
            "rowSelected": hex(t.row_selected),
            "radius": t.radius,
        },
        "workspace": app.ui.workspace,
        "workspaces": menus::WORKSPACES,
        "menus": tree.iter().map(|(menu, _)| *menu).collect::<Vec<_>>(),
        "droppedItems": dropped,
        "shortcuts": menus::effective_shortcuts(app),
        "tool": app.session.tool_id(),
    })
}

/// Items of `items` (submenus included) the Evora build drops (`evora::menus::dropped`): 0 expected.
fn count_dropped(items: &[Item]) -> usize {
    items
        .iter()
        .map(|it| match it {
            Item::Sub(_, children) => count_dropped(children),
            Item::Cmd { id, .. } if designcraft_ui_egui::evora::menus::dropped(id.as_str()) => 1,
            _ => 0,
        })
        .sum()
}

/// `diag.labels` (EVORA-CIC-11, CIC-19): every string the tables show in the current language, the
/// count of those translated, and Evora's own strings of this language left untranslated.
pub fn diag_labels(app: &DesignApp) -> Value {
    let lang = app.ui.language.clone();
    let served = designcraft_ui_egui::i18n::evora_served(&lang);
    let translated = served.iter().filter(|pair| pair.0 != pair.1).count();
    let own = designcraft_ui_egui::i18n::evora_own_keys(&lang);
    let missing: Vec<&str> = own.iter().copied().filter(|k| designcraft_ui_egui::i18n::tr(&lang, k) == *k).collect();
    json!({
        "lang": if lang.is_empty() { "en".to_string() } else { lang.clone() },
        "count": served.len(),
        "translated": translated,
        "own": own.len(),
        "missingOwn": missing,
        "labels": served.iter().map(|pair| pair.1).collect::<Vec<&str>>(),
    })
}
