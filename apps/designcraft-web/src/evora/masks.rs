//! Evora build (patches CIC-05 and CIC-15, class E7): what the host build hides and refuses.
//!
//! Under the bridge the editor holds ONE document of the host (DC-9): creating, opening or closing a
//! document belongs to the host shell, which also owns printing, packaging to a folder, links to the
//! outside, the About window and leaving the application. Those menu items are hidden
//! ([`hide_menu_items`], through the menus' own hidden-item list) and every action of the list is
//! refused before the app acts on it ([`refuses`], the host filter of couture CL6-01/CL6-02), by its
//! menu, its shortcut, Quick Apply or a dialog alike. The bridge itself opens and replaces the host's
//! document under [`HostCall`], which lifts the refusal for the time of that call only.
//!
//! INDD and INDT never reach the engine: the host's picker does not offer them (`pick`). The
//! upstream build has no script menu at v0.13.0.

use std::cell::Cell;
use std::rc::Rc;

use designcraft_ui_egui::DesignApp;
use designcraft_ui_egui::menus::{self, Item};

/// Menu items hidden under the host (and refused, [`refuses`]).
const HIDDEN: [&str; 17] = [
    "app.newDocumentDialog",
    "app.openDialog",
    "app.openRecent",
    "file.clearRecent",
    "file.close",
    "app.saveCopyDialog",
    "app.quit",
    "app.packageDialog",
    "app.printBooklet",
    "app.placeAndLink",
    "help.about",
    "help.discord",
    "help.appPage",
    "help.github",
    "help.website",
    "help.app",
    "help.issues",
];
/// Actions refused under the host besides the hidden items: the engine commands behind them.
const REFUSED: [&str; 9] = [
    "file.open",
    "file.openIdml",
    "file.openBytes",
    "file.openRecent",
    "file.saveACopy",
    "file.package",
    "file.printBooklet",
    "file.print",
    "story.placeAndLink",
];
/// Whole families refused: new documents, closing, the outside links.
const REFUSED_PREFIXES: [&str; 3] = ["file.new", "file.close", "help."];

/// Is `id` a menu item the host build hides?
pub fn hidden(id: &str) -> bool {
    HIDDEN.contains(&id)
}

/// Is `id` refused under the host?
pub fn refuses(id: &str) -> bool {
    HIDDEN.contains(&id) || REFUSED.contains(&id) || REFUSED_PREFIXES.iter().any(|p| id.starts_with(p))
}

/// The bridge's own calls (open, replace the host's document): the refusal is lifted while one runs.
#[derive(Clone, Default)]
pub struct HostCall(Rc<Cell<bool>>);

impl HostCall {
    pub fn active(&self) -> bool {
        self.0.get()
    }

    /// Run `f` as the host.
    pub fn run<R>(&self, f: impl FnOnce() -> R) -> R {
        let before = self.0.replace(true);
        let r = f();
        self.0.set(before);
        r
    }
}

/// The host filter installed in `Services::host_filter`: `Some(reason)` refuses the action.
pub fn filter(host: HostCall) -> designcraft_ui_egui::HostFilterFn {
    Box::new(move |id: &str| (refuses(id) && !host.active()).then(|| "this action belongs to Evora".to_string()))
}

/// Hide the items of [`HIDDEN`] in the menus (their hidden-item list, keyed by menu path and label).
pub fn hide_menu_items(app: &mut DesignApp) {
    fn walk(items: &[Item], path: &str, out: &mut Vec<String>) {
        for it in items {
            match it {
                Item::Sub(name, children) => walk(children, &format!("{path}/{name}"), out),
                Item::Cmd { label, id, .. } if hidden(id) => out.push(menus::menu_key(path, label)),
                _ => {}
            }
        }
    }
    let mut keys = Vec::new();
    for (menu, entries) in menus::menu_tree() {
        walk(&entries, menu, &mut keys);
    }
    for key in keys {
        if !app.ui.hidden_menu_items.contains(&key) {
            app.ui.hidden_menu_items.push(key);
        }
    }
}
