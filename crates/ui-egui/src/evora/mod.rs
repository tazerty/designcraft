//! Evora build (patches EVORA-CIC-12, EVORA-CIC-13 and EVORA-CIC-22 of `apps/cicero/UPSTREAM.md` in
//! Evora, class E9): the interface Evora designed for its layout editor, by function, written by
//! Evora's own team. No workspace, shortcut or menu order is taken from another program.
//!
//! - `workspaces`: Evora's workspaces (CIC-12);
//! - `keys`: Evora's default shortcuts, which replace every upstream default (CIC-13);
//! - `menus`: Evora's order of the menus, and the items the Evora build drops (CIC-22).
//!
//! The theme (CIC-02) lives in `theme.rs`; the display override catalogue and Evora's own strings
//! (CIC-11, CIC-19) in `i18n/evora/`.

pub mod keys;
pub mod workspaces;
