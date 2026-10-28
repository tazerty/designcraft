//! Evora build (patch CIC-03, class E3): the language imposed by the host.
//!
//! `?lang=` (read by key, never by a substring search), then `evora:init.engineLang` and
//! `evora:lang`, choose the interface language among the engine's own (`i18n::LANGUAGES`). An Evora
//! language code maps to the engine's: `en` → `""` (the source language), `zh-Hans` → `zh`, `pt`
//! → `pt-br` (D-23: Evora's Portuguese is Brazil's), and `fr`, `de`, `es`, `ja`, `ar`, `it`, `ru`
//! as they are; an engine code is taken as it is. Evora's `hi`, `id`, `ko` and `tr` have no engine
//! table: the interface stays in English and the host shows its language banner. An unknown code
//! is ignored, without error.

/// The engine language for `code` (an Evora or an engine code), if the engine has its table.
pub fn engine_language(code: &str) -> Option<&'static str> {
    let mapped = match code {
        "en" | "" => "",
        "zh-Hans" | "zh" => "zh",
        "pt" | "pt-br" | "pt-BR" | "pt_BR" => "pt-br",
        other => other,
    };
    designcraft_ui_egui::i18n::LANGUAGES.iter().map(|(c, _)| *c).find(|c| *c == mapped)
}

/// Apply `code` to the app; `false` when the engine has no table for it (English then).
pub fn apply(app: &mut designcraft_ui_egui::DesignApp, code: Option<&str>) -> bool {
    let lang = code.and_then(engine_language);
    let _ = app.run("app.language", serde_json::json!({"lang": lang.unwrap_or("")}));
    lang.is_some() || code.is_none()
}
