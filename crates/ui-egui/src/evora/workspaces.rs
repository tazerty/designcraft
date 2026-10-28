//! Evora build (patch EVORA-CIC-12, class E9): Evora's workspaces, named and laid out by function, in
//! the order of the work: design the pages, set the text, treat color and images, prepare the
//! output, read it over. They replace the upstream build's built-in workspaces; saved custom
//! workspaces are untouched.

use crate::DesignApp;

/// Evora's built-in workspaces, in the order of the work.
pub const NAMES: &[&str] = &["Page Design", "Typesetting", "Color and Images", "Production", "Proofreading", "Essentials" /* SABORDAGE QA */];

/// The workspace a new interface starts in (its layout is the interface's default one).
pub const DEFAULT: &str = "Page Design";

/// A built-in workspace: the bars and panels it shows.
struct Preset {
    name: &'static str,
    /// The control bar above the document.
    control_bar: bool,
    /// The tab of the expanded dock, when it sets one.
    dock_tab: Option<&'static str>,
    /// The icon column's panel groups (the dock is collapsed into it), when it has one.
    groups: Option<&'static [&'static [&'static str]]>,
    /// The panel open beside the document.
    open_panel: Option<&'static str>,
}

/// Typesetting: styles, then formatting, then glyphs and type features, then the text's flow.
const TYPESETTING_GROUPS: &[&[&str]] = &[
    &["paragraphStyles", "characterStyles"],
    &["paragraph", "character"],
    &["glyphs", "typography"],
    &["story", "textWrap"],
];

/// Color and images: color, then effects, then the placed files.
const COLOR_GROUPS: &[&[&str]] = &[&["swatches", "color", "gradient"], &["effects"], &["links"]];

/// Production: checks first, then the placed files, the inks, the measures.
const PRODUCTION_GROUPS: &[&[&str]] = &[&["preflight"], &["links"], &["swatches"], &["info", "attributes"]];

const PRESETS: &[Preset] = &[
    Preset { name: "Page Design", control_bar: false, dock_tab: Some("properties"), groups: None, open_panel: None },
    Preset { name: "Typesetting", control_bar: true, dock_tab: None, groups: Some(TYPESETTING_GROUPS), open_panel: Some("paragraphStyles") },
    Preset { name: "Color and Images", control_bar: false, dock_tab: None, groups: Some(COLOR_GROUPS), open_panel: Some("swatches") },
    Preset { name: "Production", control_bar: true, dock_tab: None, groups: Some(PRODUCTION_GROUPS), open_panel: Some("preflight") },
    Preset { name: "Proofreading", control_bar: false, dock_tab: Some("pages"), groups: None, open_panel: Some("notes") },
];

/// Switch to the built-in workspace `name`; `false` when it is not one of Evora's.
pub fn apply(app: &mut DesignApp, name: &str) -> bool {
    let Some(preset) = PRESETS.iter().find(|p| p.name == name) else {
        return false;
    };
    if app.ui.docking.is_some() {
        app.ui.floating.clear();
    }
    app.ui.docking = None;
    app.ui.docking_hidden.clear();
    app.ui.control_bar = preset.control_bar;
    app.ui.icon_groups = preset.groups.map(|groups| groups.iter().map(|g| g.iter().map(|id| (*id).to_string()).collect()).collect());
    app.ui.dock_expanded = preset.groups.is_none();
    if let Some(tab) = preset.dock_tab {
        app.ui.dock_tab = tab.to_string();
        app.ui.dock_expanded = true;
    }
    app.ui.open_panel = preset.open_panel.map(str::to_string);
    app.ui.workspace = preset.name.to_string();
    true
}
