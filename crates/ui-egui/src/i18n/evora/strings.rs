//! Evora build (patches EVORA-CIC-19, class E8, and EVORA-CIC-02, CIC-12): Evora's own strings.
//! `(key, language, text)`, written by Evora from the English source (no translation of another
//! program consulted): the strings the Evora build adds, in the languages Evora serves to the engine,
//! then the French complement of the strings the upstream French tables lack (CIC-19).

pub(super) const TABLE: &[(&str, &str, &str)] = &[
    // Evora's brightness levels (CIC-02), in the languages whose upstream table lacks them.
    ("Light", "es", "Claro"),
    ("Light", "it", "Chiaro"),
    ("Light", "zh", "浅色"),
    ("Light", "pt-br", "Claro"),
    ("Dark", "es", "Oscuro"),
    ("Dark", "it", "Scuro"),
    ("Dark", "zh", "深色"),
    ("Dark", "pt-br", "Escuro"),
    ("High Contrast", "es", "Alto contraste"),
    ("High Contrast", "it", "Contrasto elevato"),
    ("High Contrast", "zh", "高对比度"),
    ("High Contrast", "pt-br", "Alto contraste"),
];
