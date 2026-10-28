//! Evora build (patch EVORA-CIC-22, class E9 extended): Evora's order of the menus and the items the
//! Evora build drops.
//!
//! The order follows the work, from the document to the detail: the file, editing, viewing, then the
//! pages, the objects on them, their text and tables, and the windows. The help menu is dropped: the
//! outside links, the About window and the sample document belong to the host shell, which has its
//! own help. The import of another program's shortcut set is dropped too (`keys::IMPORT_OFFERED`).
//! Separators left at an edge or doubled, and emptied submenus, go.

use crate::menus::Item;

/// Evora's order of the menus (the upstream build's menu names, kept as keys).
pub const ORDER: [&str; 8] = ["File", "Edit", "View", "Layout", "Object", "Type", "Table", "Window"];

/// Menus the Evora build drops.
pub const DROPPED_MENUS: [&str; 1] = ["Help"];

/// Items the Evora build drops, wherever they are: the sample document and the shortcut set import.
pub const DROPPED_ITEMS: [&str; 2] = ["file.newSample", "window.importShortcuts"];

/// Is the item `id` dropped from the menus?
pub fn dropped(id: &str) -> bool {
    id.starts_with("help.") || DROPPED_ITEMS.contains(&id)
}

/// The menu tree in Evora's order, without the dropped menus and items. A menu the order does not
/// name (one a later upstream version adds) keeps its upstream place after the named ones.
pub fn arrange(mut tree: Vec<(&'static str, Vec<Item>)>) -> Vec<(&'static str, Vec<Item>)> {
    let mut out: Vec<(&'static str, Vec<Item>)> = Vec::with_capacity(tree.len());
    for name in ORDER {
        if let Some(i) = tree.iter().position(|(menu, _)| *menu == name) {
            out.push(tree.remove(i));
        }
    }
    out.extend(tree);
    for (_, items) in out.iter_mut() {
        prune(items);
    }
    out.retain(|(menu, items)| !DROPPED_MENUS.contains(menu) && items.iter().any(|it| !matches!(it, Item::Sep)));
    out
}

/// Drop the dropped items (in submenus too), emptied submenus, and stray separators.
fn prune(items: &mut Vec<Item>) {
    let mut kept: Vec<Item> = Vec::with_capacity(items.len());
    for it in items.drain(..) {
        match it {
            Item::Cmd { ref id, .. } if dropped(id.as_str()) => {}
            Item::Sub(name, mut children) => {
                prune(&mut children);
                if children.iter().any(|c| !matches!(c, Item::Sep)) {
                    kept.push(Item::Sub(name, children));
                }
            }
            Item::Sep => {
                if !kept.is_empty() && !matches!(kept.last(), Some(Item::Sep)) {
                    kept.push(Item::Sep);
                }
            }
            other => kept.push(other),
        }
    }
    while matches!(kept.last(), Some(Item::Sep)) {
        kept.pop();
    }
    *items = kept;
}
