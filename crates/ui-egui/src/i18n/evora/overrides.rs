//! Evora build (patch EVORA-CIC-11, class E9): the display override catalogue. `(key, language,
//! text)`: the key is the upstream source string (never shown); `""` is English. Every key has an
//! English row, which an untranslated key falls back to (`i18n::tr`). The CI gate « termes » checks
//! the served texts, in every language, against the clean team's list (Evora's CR-06 list).

pub(super) const TABLE: &[(&str, &str, &str)] = &[
    // The page-adding option of a document's main text frame (vocabulary renamed by the Evora build).
    (
        "Add and remove pages as the primary text frame's story grows and shrinks",
        "",
        "Add and remove pages as the main text frame's story grows and shrinks",
    ),
    (
        "Add and remove pages as the primary text frame's story grows and shrinks",
        "es",
        "Añadir y quitar páginas según crece o se reduce el texto del marco principal",
    ),
    // The import of another program's shortcut set (not offered by the Evora build: `keys`).
    ("Import InDesign Set…", "", "Import a Set…"),
    ("Import InDesign Set…", "fr", "Importer un jeu…"),
    ("Import InDesign Shortcut Set", "", "Import a Shortcut Set"),
    ("Import InDesign Shortcut Set", "fr", "Importer un jeu de raccourcis"),
    // A placed image's own clipping path, as the Text Wrap panel names it.
    ("Photoshop Path", "", "Embedded Clipping Path"),
    ("Photoshop Path", "fr", "Tracé de détourage incorporé"),
    ("Photoshop Path", "de", "Eingebetteter Beschneidungspfad"),
    ("Photoshop Path", "es", "Trazado de recorte incrustado"),
    ("Photoshop Path", "it", "Tracciato di ritaglio incorporato"),
    ("Photoshop Path", "ja", "埋め込みクリッピングパス"),
    ("Photoshop Path", "zh", "嵌入的剪切路径"),
    ("Photoshop Path", "ar", "مسار القص المضمّن"),
    ("Photoshop Path", "pt-br", "Demarcador de recorte incorporado"),
    ("Photoshop Path", "ru", "Встроенный обтравочный контур"),
    // The text frame a new document's pages flow into (vocabulary renamed by the Evora build).
    ("Primary Text Frame", "", "Main Text Frame"),
    ("Primary Text Frame", "es", "Marco de texto principal"),
];
