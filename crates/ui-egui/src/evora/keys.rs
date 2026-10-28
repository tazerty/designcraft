//! Evora build (patch EVORA-CIC-13, class E9): Evora's default shortcuts.
//!
//! They REPLACE every default of the upstream build (`menus::default_shortcut`, the engine's commands
//! and the UI commands alike): the conventions of the systems and browsers for the universal actions
//! (undo, clipboard, save, find, zoom, text style, command search), then a small grid Evora designed
//! by function for layout work (group, place, insert pages, preview). Rules:
//! - no key of a single character is bound by default (WCAG 2.1.4): the tools' single-key shortcuts
//!   are off ([`TOOL_KEYS`]); the member binds any command in Edit › Keyboard Shortcuts;
//! - no combination the browsers keep for themselves (new window or tab, close, tab switching, quit,
//!   developer tools), nor the host's F6 (`ui.focus-out`);
//! - no Ctrl+Alt with a letter or digit that types a character on the keyboards of Evora's languages
//!   (the AltGr key sends Ctrl+Alt).
//!
//! No shortcut set of another program is offered, nor its import ([`IMPORT_OFFERED`]).

/// Evora's default shortcuts: (command id, shortcut as the app writes it; `Cmd` is Ctrl off macOS).
pub const DEFAULTS: &[(&str, &str)] = &[
    // Universal conventions of the systems and browsers.
    ("edit.undo", "Cmd+Z"),
    ("edit.redo", "Cmd+Shift+Z"),
    ("edit.cut", "Cmd+X"),
    ("edit.copy", "Cmd+C"),
    ("edit.paste", "Cmd+V"),
    ("edit.pasteWithoutFormatting", "Cmd+Shift+V"),
    ("edit.selectAll", "Cmd+A"),
    ("edit.clear", "Delete"),
    ("app.save", "Cmd+S"),
    ("app.findChange", "Cmd+F"),
    ("app.palette", "Cmd+K"),
    ("view.zoomIn", "Cmd+="),
    ("view.zoomOut", "Cmd+-"),
    ("view.fitPage", "Cmd+0"),
    ("view.actualSize", "Cmd+1"),
    ("type.bold", "Cmd+B"),
    ("type.italic", "Cmd+I"),
    ("type.underline", "Cmd+U"),
    // Evora's grid for layout work.
    ("object.group", "Cmd+G"),
    ("object.ungroup", "Cmd+Shift+G"),
    ("app.placeDialog", "Cmd+Alt+P"),
    ("layout.pages.insert", "Cmd+Alt+N"),
    ("view.togglePreview", "Cmd+Alt+V"),
];

/// The tools' single-key shortcuts (one character, or Shift with one): off in the Evora build.
pub const TOOL_KEYS: bool = false;

/// The import of another program's shortcut set (Edit › Keyboard Shortcuts): not offered.
pub const IMPORT_OFFERED: bool = false;

/// Evora's default shortcut of `id`, if it has one.
pub fn default_shortcut(id: &str) -> Option<&'static str> {
    DEFAULTS.iter().find(|(command, _)| *command == id).map(|(_, shortcut)| *shortcut)
}
