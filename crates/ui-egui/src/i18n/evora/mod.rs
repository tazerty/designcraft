//! Evora build (patches EVORA-CIC-11 and EVORA-CIC-19 of `apps/cicero/UPSTREAM.md` in Evora, classes
//! E9 and E8): the display override catalogue and Evora's own strings, which `i18n::tr` consults
//! before the upstream tables.
//!
//! - `overrides`: what the interface must not show, replaced at display (E9, CIC-11): a term of the
//!   clean team's list (Evora's CR-06 list) or vocabulary the Evora build renames. A key with an
//!   English (`""`) override never shows its upstream text: in a language without its own override,
//!   an untranslated key falls back to the English override, never to the key;
//! - `strings`: the strings the Evora build adds (workspaces, brightness levels) and the French
//!   complement of the keys the upstream French table lacks (E8, CIC-19), in the languages Evora serves
//!   to the engine (fr, de, es, it, ja, zh, ar, pt-br, ru), written by Evora from the English source.
//!
//! Both tables are `(key, language, text)` rows: the CI gate « termes » reads the first column as the
//! source key, never shown as such.

use std::{collections::HashMap, sync::OnceLock};

mod overrides;
mod strings;

type Rows = &'static [(&'static str, &'static str, &'static str)];
type ByLanguage = HashMap<&'static str, HashMap<&'static str, &'static str>>;

fn index(rows: Rows) -> ByLanguage {
    let mut map: ByLanguage = HashMap::new();
    for &(key, lang, text) in rows {
        map.entry(lang).or_default().insert(key, text);
    }
    map
}

/// The override of `s` in `lang` (`""`: English), if the catalogue holds one.
pub(super) fn display(lang: &str, s: &str) -> Option<&'static str> {
    static OVERRIDES: OnceLock<ByLanguage> = OnceLock::new();
    OVERRIDES.get_or_init(|| index(overrides::TABLE)).get(lang).and_then(|m| m.get(s)).copied()
}

/// Evora's own text of `s` in `lang`, if it has one.
pub(super) fn string(lang: &str, s: &str) -> Option<&'static str> {
    static STRINGS: OnceLock<ByLanguage> = OnceLock::new();
    STRINGS.get_or_init(|| index(strings::TABLE)).get(lang).and_then(|m| m.get(s)).copied()
}

/// The keys of both catalogues (overrides first), once each.
pub(super) fn keys() -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for &(key, _, _) in overrides::TABLE.iter().chain(strings::TABLE.iter()) {
        if !out.contains(&key) {
            out.push(key);
        }
    }
    out
}

/// The keys of Evora's own strings that have a text in `lang`.
pub(super) fn own_keys(lang: &str) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for &(key, l, _) in strings::TABLE {
        if l == lang && !out.contains(&key) {
            out.push(key);
        }
    }
    out
}
