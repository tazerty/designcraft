//! Evora build: the native bridge to the Evora host shell (patches CIC-03, CIC-04, CIC-05, CIC-07 to
//! CIC-10 and CIC-15 of `apps/cicero/UPSTREAM.md` in Evora). Turned on by `?host=evora` only; without
//! it the web build behaves as upstream's, minus the brands (CIC-01) and the emoji (CIC-06).
//!
//! - `ehp`: the protocol (EHP v1): envelope, origin and source checks, messages;
//! - `docs`: the host's document in the engine (open, save, saved state, dirty, requests);
//! - `pick`: the engine's file pickers become the host's;
//! - `commands`: the compiled command list, the command catalogue (white list, no free parameter);
//! - `masks`: what the host build hides and refuses (one document per editor, print, links);
//! - `focus`: F6 gives the keyboard back to the host;
//! - `lang`: the language imposed by the host;
//! - `storage`: the engine keeps nothing in the browser (purge = check).

pub mod commands;
pub mod docs;
pub mod ehp;
pub mod focus;
pub mod lang;
pub mod masks;
pub mod pick;
pub mod storage;
