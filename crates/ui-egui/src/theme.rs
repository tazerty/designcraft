//! Design tokens (Evora's three interface brightness levels: light, dark and high contrast; patch
//! EVORA-CIC-02 of `apps/cicero/UPSTREAM.md` in Evora), fonts (Inter) and egui style.

use std::sync::Arc;

use egui::{Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Stroke, TextStyle, Visuals};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Brightness {
    Dark,
    /// Evora build: read as `Dark` (an intermediate level of the upstream build, kept for old settings).
    MediumDark,
    /// Evora build: read as `Light` (idem).
    MediumLight,
    #[default]
    Light,
    /// Accessibility: black chrome, white text, yellow selection and focus.
    HighContrast,
}

impl Brightness {
    /// Evora's three levels (EVORA-CIC-02), in the menu's order.
    pub const ALL: [Brightness; 3] = [Brightness::Light, Brightness::Dark, Brightness::HighContrast];
    pub fn label(self) -> &'static str {
        match self {
            Brightness::Dark => "Dark",
            Brightness::MediumDark => "Dark",
            Brightness::MediumLight => "Light",
            Brightness::Light => "Light",
            Brightness::HighContrast => "High Contrast",
        }
    }
    pub fn id(self) -> &'static str {
        match self {
            Brightness::Dark => "dark",
            Brightness::MediumDark => "mediumDark",
            Brightness::MediumLight => "mediumLight",
            Brightness::Light => "light",
            Brightness::HighContrast => "highContrast",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        // Evora build: the upstream build's intermediate levels are read as Evora's dark and light.
        if s.eq_ignore_ascii_case("mediumDark") {
            return Some(Brightness::Dark);
        }
        if s.eq_ignore_ascii_case("mediumLight") {
            return Some(Brightness::Light);
        }
        Self::ALL.into_iter().find(|b| b.id().eq_ignore_ascii_case(s) || b.label().eq_ignore_ascii_case(s))
    }
}

/// Every colour the UI uses. Widgets never hard-code colours.
#[derive(Clone, Copy, Debug)]
pub struct Tokens {
    pub dark: bool,
    pub app_bar: Color32,
    pub panel: Color32,
    pub panel_darker: Color32,
    pub input: Color32,
    pub input_border: Color32,
    pub divider: Color32,
    pub text: Color32,
    pub text_strong: Color32,
    pub text_dim: Color32,
    pub text_disabled: Color32,
    pub icon: Color32,
    pub hover: Color32,
    pub tool_active: Color32,
    pub accent: Color32,
    pub accent_strong: Color32,
    pub row_selected: Color32,
    pub pasteboard: Color32,
    pub ruler: Color32,
    pub ruler_tick: Color32,
    pub tab_inactive: Color32,
    pub button: Color32,
    pub radius: u8,
    /// 1 pt chrome edges.
    pub border: Color32,
    /// Outline of fields and buttons.
    pub field_border: Color32,
    /// Active tool / toggle well.
    pub well: Color32,
    pub well_rim: Color32,
    pub ruler_text: Color32,
    pub section_divider: Color32,
    /// Panel tab strips, doc-tab bar, dock headers.
    pub tab_strip: Color32,
    /// Smart guides and spacing marks drawn over the page.
    pub smart_guide: Color32,
    /// Measurement labels on the canvas (smart dimensions, gaps).
    pub measure_bg: Color32,
    pub measure_text: Color32,
    /// Live-corner widget and corner diamonds.
    pub live_corner: Color32,
    /// The Windows caption Close button when hovered, and its glyph.
    pub caption_close: Color32,
    pub caption_close_text: Color32,
}

fn hex(s: u32) -> Color32 {
    Color32::from_rgb((s >> 16) as u8, (s >> 8) as u8, s as u8)
}

impl Tokens {
    /// Evora's tokens (EVORA-CIC-02): the zinc neutrals and the indigo accent of Evora's design system
    /// (`apps/cicero/CONCEPTION-PRO.md` §7.6 in Evora), in three brightness levels. No value of the
    /// upstream build is kept; its two intermediate levels read as dark and light.
    pub fn for_brightness(b: Brightness) -> Self {
        let light = Tokens {
            dark: false,
            app_bar: hex(0xffffff),
            panel: hex(0xfafafa),
            panel_darker: hex(0xf4f4f5),
            input: hex(0xffffff),
            input_border: hex(0x8a8a93),
            divider: hex(0xe4e4e7),
            text: hex(0x27272a),
            text_strong: hex(0x18181b),
            text_dim: hex(0x63636b),
            text_disabled: hex(0xa1a1aa),
            icon: hex(0x3f3f46),
            hover: hex(0xf4f4f5),
            tool_active: hex(0xeef2ff),
            accent: hex(0x4f46e5),
            accent_strong: hex(0x4f46e5),
            row_selected: hex(0xe0e7ff),
            pasteboard: hex(0xe4e4e7),
            ruler: hex(0xffffff),
            ruler_tick: hex(0x71717a),
            tab_inactive: hex(0xf4f4f5),
            button: hex(0xffffff),
            radius: 4,
            border: hex(0xe4e4e7),
            field_border: hex(0x8a8a93),
            well: hex(0xeef2ff),
            well_rim: hex(0xc7d2fe),
            ruler_text: hex(0x3f3f46),
            section_divider: hex(0xe4e4e7),
            tab_strip: hex(0xf4f4f5),
            smart_guide: hex(0x10b981),
            measure_bg: Color32::from_rgba_unmultiplied(24, 24, 27, 230),
            measure_text: Color32::WHITE,
            live_corner: hex(0xf59e0b),
            caption_close: hex(0xdc2626),
            caption_close_text: Color32::WHITE,
        };
        let dark = Tokens {
            dark: true,
            app_bar: hex(0x18181b),
            panel: hex(0x18181b),
            panel_darker: hex(0x09090b),
            input: hex(0x09090b),
            input_border: hex(0x71717a),
            divider: hex(0x27272a),
            text: hex(0xe4e4e7),
            text_strong: hex(0xfafafa),
            text_dim: hex(0xa1a1aa),
            text_disabled: hex(0x52525b),
            icon: hex(0xd4d4d8),
            hover: hex(0x27272a),
            tool_active: hex(0x1e1b3a),
            accent: hex(0x818cf8),
            accent_strong: hex(0x4f46e5),
            row_selected: hex(0x1e1b4b),
            pasteboard: hex(0x0c0c0e),
            ruler: hex(0x18181b),
            ruler_tick: hex(0x71717a),
            tab_inactive: hex(0x18181b),
            button: hex(0x27272a),
            radius: 4,
            border: hex(0x27272a),
            field_border: hex(0x71717a),
            well: hex(0x1e1b4b),
            well_rim: hex(0x3730a3),
            ruler_text: hex(0xd4d4d8),
            section_divider: hex(0x27272a),
            tab_strip: hex(0x09090b),
            smart_guide: hex(0x34d399),
            measure_bg: Color32::from_rgba_unmultiplied(39, 39, 42, 235),
            measure_text: Color32::WHITE,
            live_corner: hex(0xfbbf24),
            caption_close: hex(0xdc2626),
            caption_close_text: Color32::WHITE,
        };
        match b {
            Brightness::Light | Brightness::MediumLight => light,
            Brightness::Dark | Brightness::MediumDark => dark,
            // Black chrome and white text, amber focus and highlights; filled controls keep white text.
            Brightness::HighContrast => Tokens {
                dark: true,
                app_bar: hex(0x09090b),
                panel: hex(0x09090b),
                panel_darker: hex(0x000000),
                input: hex(0x000000),
                input_border: hex(0xffffff),
                divider: hex(0xa1a1aa),
                text: hex(0xffffff),
                text_strong: hex(0xffffff),
                text_dim: hex(0xe4e4e7),
                text_disabled: hex(0xa1a1aa),
                icon: hex(0xffffff),
                hover: hex(0x27272a),
                tool_active: hex(0x3f3f46),
                accent: hex(0xfcd34d),
                accent_strong: hex(0x4338ca),
                row_selected: hex(0x3730a3),
                pasteboard: hex(0x18181b),
                ruler: hex(0x000000),
                ruler_tick: hex(0xffffff),
                tab_inactive: hex(0x09090b),
                button: hex(0x000000),
                radius: 4,
                border: hex(0xffffff),
                field_border: hex(0xffffff),
                well: hex(0x3f3f46),
                well_rim: hex(0xfcd34d),
                ruler_text: hex(0xffffff),
                section_divider: hex(0xa1a1aa),
                tab_strip: hex(0x000000),
                smart_guide: hex(0x34d399),
                measure_bg: Color32::from_rgba_unmultiplied(0, 0, 0, 240),
                measure_text: Color32::WHITE,
                live_corner: hex(0xfcd34d),
                caption_close: hex(0xdc2626),
                caption_close_text: Color32::WHITE,
            },
        }
    }
}

/// Install the UI fonts for the interface language `lang` (it orders the CJK fallbacks).
/// Returns false while the installed fonts are still being cataloged: the built-in faces are
/// installed without waiting, and the caller installs again once they are (see
/// [`system_fonts_ready`]) to add the installed faces for the scripts they lack.
pub fn install_fonts(ctx: &egui::Context, lang: &str) -> bool {
    let craft = designcraft_fonts::CRAFT_FONTS;
    let mut defs = font_definitions(craft, lang);
    let system = system_ui_fonts(craft);
    if let Some(system) = system {
        add_system_fallbacks(&mut defs, system, lang);
    }
    ctx.set_fonts(defs);
    system.is_some()
}

/// Whether [`install_fonts`] would find the installed fonts cataloged.
pub fn system_fonts_ready() -> bool {
    designcraft_fonts::FontDb::global().system_fonts_cataloged()
}

/// An installed font the UI falls back to for a script the built-in faces lack.
pub(crate) struct SystemFont {
    /// The ISO 15924 code of the script it is for ("Jpan", "Kore", "Taml"…).
    script: &'static str,
    family: String,
    /// The face as egui takes it, on the UI face's baseline. Shared: installing the fonts again
    /// (another interface language) doesn't copy the file.
    data: Arc<FontData>,
}

impl SystemFont {
    pub(crate) fn new(script: &'static str, family: String, bytes: Vec<u8>, index: u32) -> Self {
        let tweak = baseline_tweak(&bytes, index);
        SystemFont { script, family, data: Arc::new(FontData { index, tweak, ..FontData::from_owned(bytes) }) }
    }
}

/// Installed UI faces (sans serif) per script, with a character every face of the script has:
/// the first listed family that is installed and covers the character, macOS families first,
/// then Windows, then Linux. One family can serve several scripts (Nirmala UI covers the Indic
/// scripts); it is added once.
const SYSTEM_UI_FONTS: &[(&str, char, &[&str])] = &[
    (
        "Jpan",
        'あ',
        &[
            "Hiragino Sans",
            "Hiragino Kaku Gothic ProN",
            "Yu Gothic UI",
            "Yu Gothic",
            "Meiryo",
            "MS Gothic",
            "Noto Sans CJK JP",
            "Noto Sans JP",
            "Source Han Sans JP",
            "IPAexGothic",
            "Droid Sans Fallback",
        ],
    ),
    ("Hans", '中', &["PingFang SC", "Hiragino Sans GB", "Microsoft YaHei", "Noto Sans CJK SC", "Source Han Sans SC", "WenQuanYi Micro Hei"]),
    // Traditional Chinese and Korean: font menus list those families under their native names.
    ("Hant", '中', &["PingFang TC", "Microsoft JhengHei", "Noto Sans CJK TC", "Source Han Sans TC"]),
    ("Kore", '한', &["Apple SD Gothic Neo", "Malgun Gothic", "Noto Sans CJK KR", "Noto Sans KR", "Source Han Sans K", "NanumGothic", "UnDotum"]),
    // Scripts the built-in faces lack, after the CJK ones (text in the Story Editor, font names).
    ("Taml", 'த', &["Tamil Sangam MN", "Tamil MN", "Nirmala UI", "Latha", "Noto Sans Tamil", "Noto Sans Tamil UI", "Lohit Tamil"]),
    ("Deva", 'क', &["Kohinoor Devanagari", "Devanagari Sangam MN", "Nirmala UI", "Mangal", "Noto Sans Devanagari", "Lohit Devanagari"]),
    ("Beng", 'ক', &["Kohinoor Bangla", "Bangla Sangam MN", "Nirmala UI", "Vrinda", "Noto Sans Bengali", "Lohit Bengali"]),
    ("Gujr", 'ક', &["Kohinoor Gujarati", "Gujarati Sangam MN", "Nirmala UI", "Shruti", "Noto Sans Gujarati", "Lohit Gujarati"]),
    ("Guru", 'ਕ', &["Gurmukhi Sangam MN", "Gurmukhi MN", "Nirmala UI", "Raavi", "Noto Sans Gurmukhi", "Lohit Gurmukhi"]),
    ("Knda", 'ಕ', &["Kannada Sangam MN", "Nirmala UI", "Tunga", "Noto Sans Kannada", "Lohit Kannada"]),
    ("Mlym", 'ക', &["Malayalam Sangam MN", "Nirmala UI", "Kartika", "Noto Sans Malayalam", "Lohit Malayalam"]),
    ("Telu", 'క', &["Kohinoor Telugu", "Telugu Sangam MN", "Nirmala UI", "Gautami", "Noto Sans Telugu", "Lohit Telugu"]),
    ("Orya", 'କ', &["Oriya Sangam MN", "Nirmala UI", "Kalinga", "Noto Sans Oriya", "Lohit Odia"]),
    ("Sinh", 'ක', &["Sinhala Sangam MN", "Nirmala UI", "Iskoola Pota", "Noto Sans Sinhala", "LKLUG"]),
    ("Thai", 'ก', &["Thonburi", "Leelawadee UI", "Tahoma", "Noto Sans Thai", "Loma"]),
    ("Khmr", 'ក', &["Khmer Sangam MN", "Leelawadee UI", "Khmer UI", "Noto Sans Khmer", "Khmer OS"]),
    ("Laoo", 'ກ', &["Lao Sangam MN", "Leelawadee UI", "Lao UI", "Noto Sans Lao", "Phetsarath OT"]),
    ("Mymr", 'က', &["Myanmar Sangam MN", "Myanmar Text", "Noto Sans Myanmar", "Padauk"]),
    ("Hebr", 'א', &["Arial Hebrew", "Segoe UI", "Noto Sans Hebrew", "DejaVu Sans"]),
    ("Armn", 'Ա', &["Mshtakan", "Segoe UI", "Sylfaen", "Noto Sans Armenian", "DejaVu Sans"]),
    ("Geor", 'ა', &["Segoe UI", "Sylfaen", "Noto Sans Georgian", "DejaVu Sans"]),
    ("Ethi", 'አ', &["Kefa", "Ebrima", "Nyala", "Noto Sans Ethiopic", "Abyssinica SIL"]),
];

/// The installed faces for the scripts `craft` (the built-in craft-fonts) doesn't cover, so the
/// interface shows Japanese and Chinese without the craft-fonts build input, and Korean,
/// Traditional Chinese (font names in the font menus), Indic, Southeast Asian and other scripts
/// in any build. egui can't find system fonts itself; the document font scan (not on the web)
/// does.
fn system_ui_fonts(craft: &[designcraft_fonts::CraftFont]) -> Option<&'static [SystemFont]> {
    static FACES: std::sync::OnceLock<Vec<SystemFont>> = std::sync::OnceLock::new();
    let db = designcraft_fonts::FontDb::global();
    // Never wait for the font scan here (the first frame): it runs in the background, and the
    // faces are read once, when it is done.
    // (Tests wait instead, so that the fonts don't change in the middle of one.)
    if !db.system_fonts_cataloged() && !cfg!(test) {
        db.scan_in_background();
        return None;
    }
    Some(FACES.get_or_init(|| installed_ui_fonts(db, craft)))
}

/// The faces of [`SYSTEM_UI_FONTS`] that `db` has, for the scripts `craft` doesn't cover.
fn installed_ui_fonts(db: &designcraft_fonts::FontDb, craft: &[designcraft_fonts::CraftFont]) -> Vec<SystemFont> {
    let mut faces: Vec<SystemFont> = Vec::new();
    for (script, sample, families) in SYSTEM_UI_FONTS {
        if craft.iter().any(|f| f.scripts.contains(script)) {
            continue;
        }
        let found = families.iter().find_map(|family| {
            if !db.has_family(family) {
                return None;
            }
            let face = db.face(family, "Regular");
            // `face` falls back to another family when this one can't be read: keep only the real one.
            (face.family.eq_ignore_ascii_case(family) && !face.data().is_empty() && face.covers(*sample)).then_some((family, face))
        });
        let Some((family, face)) = found else { continue };
        if !faces.iter().any(|f| f.family.eq_ignore_ascii_case(family)) {
            faces.push(SystemFont::new(script, (*family).to_string(), face.data().to_vec(), face.index()));
        }
    }
    faces
}

/// The UI face every family starts with (Inter, the face of Evora's interface, EVORA-CIC-02; the
/// semibold shares its line metrics).
const UI_FONT: &[u8] = include_bytes!("../../../assets/fonts/Inter-Regular.ttf");

/// The tweak that puts a fallback face's baseline on the UI face's. egui centres a fallback's
/// line height in the primary's instead (`pos.y = ascent + (line height − its line height) / 2`),
/// so a CJK face with a taller ascent or a larger line gap would sit above or below the Latin
/// text beside it. Shifts are in ems of the font size.
pub(crate) fn baseline_tweak(face: &[u8], index: u32) -> egui::FontTweak {
    let metrics = |data: &[u8], i: u32| {
        designcraft_fonts::line_metrics(data, i).map(|(upem, ascent, descent, gap)| (ascent / upem, (ascent - descent + gap) / upem))
    };
    let shift = match (metrics(UI_FONT, 0), metrics(face, index)) {
        (Some((ap, hp)), Some((af, hf))) => ap - (af + 0.5 * (hp - hf)),
        _ => 0.0,
    };
    egui::FontTweak { y_offset_factor: if shift.is_finite() { shift.clamp(-0.5, 0.5) } else { 0.0 }, ..Default::default() }
}

/// The script whose letterforms the interface language `lang` prefers for Han characters, Kana
/// and Hangul: "Jpan", "Hans", "Hant" or "Kore". Languages without a CJK preference use Japanese
/// forms, the app's default CJK face.
fn cjk_script(lang: &str) -> &'static str {
    match lang {
        "zh" | "zh-hans" | "zh-cn" => "Hans",
        "zh-hant" | "zh-tw" | "zh-hk" => "Hant",
        "ko" => "Kore",
        _ => "Jpan",
    }
}

/// Add installed faces `system` to every family in the interface language's order: the faces of
/// its script (`cjk_script`) go before the built-in CJK faces, so Han characters take that
/// language's forms (Traditional Chinese, Korean); the other CJK faces go at the end, then the
/// other scripts' faces.
pub(crate) fn add_system_fallbacks(fonts: &mut FontDefinitions, system: &[SystemFont], lang: &str) {
    let name = |f: &SystemFont| format!("system-{}", f.family);
    for f in system {
        fonts.font_data.insert(name(f), f.data.clone());
    }
    let first = cjk_script(lang);
    let preferred: Vec<String> = system.iter().filter(|f| f.script == first).map(name).collect();
    const CJK: [&str; 4] = ["Jpan", "Hans", "Hant", "Kore"];
    let rest: Vec<String> = CJK
        .iter()
        .filter(|s| **s != first)
        .flat_map(|s| system.iter().filter(move |f| f.script == *s))
        .chain(system.iter().filter(|f| !CJK.contains(&f.script)))
        .map(name)
        .collect();
    for (family, stack) in fonts.families.iter_mut() {
        // The Arabic families keep their own order (see `font_definitions`).
        let arabic = matches!(family, FontFamily::Name(n) if n.starts_with("arabic"));
        let at = if arabic { stack.len() } else { stack.iter().position(|n| n.starts_with("craft-")).unwrap_or(stack.len()) };
        stack.splice(at..at, preferred.iter().cloned());
        stack.extend(rest.iter().cloned());
    }
}

/// The UI fonts: the app's own, then the craft-fonts faces from `craft` (empty without
/// `CRAFT_FONTS_DIR`) as fallbacks at the end of every family: Japanese (BIZ UDPGothic first) and
/// Simplified Chinese, in the interface language's order, then Arabic and Georgian. The `arabic` families
/// put the Arabic face first for right-to-left runs. egui has no system-font discovery, so
/// without craft-fonts CJK, Arabic and Georgian UI text has no glyphs.
pub(crate) fn font_definitions(craft: &'static [designcraft_fonts::CraftFont], lang: &str) -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    let add = |fonts: &mut FontDefinitions, name: &str, data: &'static [u8]| {
        fonts.font_data.insert(name.into(), Arc::new(FontData::from_static(data)));
    };
    // Evora (EVORA-CIC-02): Inter, the face of Evora's interface.
    add(&mut fonts, "ui", UI_FONT);
    add(&mut fonts, "ui-semibold", include_bytes!("../../../assets/fonts/Inter-SemiBold.ttf"));
    // Evora (EVORA-CIC-02-b): the signs Inter lacks (the menus' small triangles…) from the previous
    // interface face, second in every family.
    add(&mut fonts, "ui-signs", include_bytes!("../../../assets/fonts/SourceSans3-Regular.ttf"));
    add(&mut fonts, "mono", include_bytes!("../../../assets/fonts/JetBrainsMono-Regular.ttf"));
    fonts.families.entry(FontFamily::Proportional).or_default().insert(0, "ui".into());
    fonts.families.entry(FontFamily::Proportional).or_default().insert(1, "ui-signs".into());
    fonts.families.entry(FontFamily::Monospace).or_default().insert(0, "mono".into());
    fonts.families.insert(FontFamily::Name("semibold".into()), vec!["ui-semibold".into(), "ui".into(), "ui-signs".into()]);
    let pick = |script: &str| -> Vec<&designcraft_fonts::CraftFont> { craft.iter().filter(|f| f.scripts.contains(&script)).collect() };
    let mut japanese = pick("Jpan");
    // BIZ UDPGothic (the UI face) first, Regular before Bold; the semibold family prefers Bold.
    japanese.sort_by_key(|f| (f.family != "BIZ UDPGothic", f.style != "Regular"));
    let chinese = pick("Hans");
    let arabic = pick("Arab");
    let georgian = pick("Geor");
    let name = |f: &designcraft_fonts::CraftFont| format!("craft-{}-{}", f.family, f.style);
    for f in japanese.iter().chain(&chinese) {
        // CJK faces sit on the Latin baseline (see `baseline_tweak`).
        fonts.font_data.insert(name(f), Arc::new(FontData { tweak: baseline_tweak(f.bytes, 0), ..FontData::from_static(f.bytes) }));
    }
    for f in arabic.iter().chain(&georgian) {
        add(&mut fonts, &name(f), f.bytes);
    }
    // Han characters take the forms of the interface language: Chinese faces first for Chinese.
    let zh = matches!(cjk_script(lang), "Hans" | "Hant");
    for (family, stack) in fonts.families.iter_mut() {
        let bold = *family == FontFamily::Name("semibold".into());
        let mut ja = japanese.clone();
        if bold {
            ja.sort_by_key(|f| (f.family != "BIZ UDPGothic", f.style != "Bold"));
        }
        let (first, second) = if zh { (&chinese, &ja) } else { (&ja, &chinese) };
        stack.extend(first.iter().chain(second).chain(&arabic).chain(&georgian).map(|f| name(f)));
    }
    // Arabic runs (rtl.rs): the Arabic face first, so letters and spaces shape as one run.
    for family in ["arabic", "arabic-semibold"] {
        let ui = if family == "arabic" { "ui" } else { "ui-semibold" };
        let stack = arabic
            .iter()
            .map(|f| name(f))
            .chain([ui.to_string(), "ui".to_string(), "ui-signs".to_string()])
            .chain(japanese.iter().chain(&chinese).chain(&georgian).map(|f| name(f)));
        let mut seen = std::collections::HashSet::new();
        fonts.families.insert(FontFamily::Name(family.into()), stack.filter(|n| seen.insert(n.clone())).collect());
    }
    fonts
}

pub fn semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("semibold".into()))
}

pub fn apply(ctx: &egui::Context, t: &Tokens) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::NULL, *t));
    let text_styles: std::collections::BTreeMap<TextStyle, FontId> = [
        (TextStyle::Small, FontId::proportional(10.0)),
        (TextStyle::Body, FontId::proportional(11.0)),
        (TextStyle::Button, FontId::proportional(11.0)),
        (TextStyle::Heading, FontId::new(15.0, FontFamily::Name("semibold".into()))),
        (TextStyle::Monospace, FontId::monospace(11.5)),
    ]
    .into();
    let mut v = if t.dark { Visuals::dark() } else { Visuals::light() };
    let r = CornerRadius::same(t.radius);
    v.panel_fill = t.panel;
    v.window_fill = t.panel;
    v.extreme_bg_color = t.input;
    v.faint_bg_color = t.panel_darker;
    v.window_stroke = Stroke::new(1.0, t.divider);
    // Evora (EVORA-CIC-02): rounded windows and menus, text selection on the selected-row tint, filled
    // buttons with a field outline, the accent on hover and press.
    v.window_corner_radius = CornerRadius::same(8);
    v.menu_corner_radius = CornerRadius::same(8);
    v.selection.bg_fill = t.row_selected;
    v.selection.stroke = Stroke::new(1.0, t.text_strong);
    v.hyperlink_color = t.accent;
    v.override_text_color = Some(t.text);
    craft_ui::theme::WidgetPalette { noninteractive: t.panel, inactive: t.button, hovered: t.hover, active: t.tool_active, open: t.hover }.apply_to(
        &mut v.widgets,
        r,
        Stroke::new(1.0, t.text),
    );
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, t.divider);
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, t.field_border);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, t.accent);
    v.widgets.active.bg_stroke = Stroke::new(1.0, t.accent);
    v.widgets.inactive.weak_bg_fill = t.button;
    v.widgets.inactive.bg_fill = t.input;
    v.popup_shadow =
        egui::epaint::Shadow { offset: [0, 8], blur: 24, spread: 0, color: Color32::from_black_alpha(if t.dark { 140 } else { 40 }) };
    ctx.set_visuals(v);
    ctx.global_style_mut(|s| {
        s.text_styles = text_styles.clone();
        s.spacing.item_spacing = egui::vec2(6.0, 4.0);
        s.spacing.button_padding = egui::vec2(6.0, 2.0);
        s.spacing.interact_size = egui::vec2(20.0, 21.0);
        s.spacing.menu_margin = egui::Margin::same(4);
        s.animation_time = 0.06;
    });
}

impl Tokens {
    /// Tokens stored by [`apply`].
    pub fn get(ctx: &egui::Context) -> Tokens {
        ctx.data(|d| d.get_temp::<Tokens>(egui::Id::NULL)).unwrap_or_else(|| Tokens::for_brightness(Brightness::Dark))
    }
}

#[cfg(test)]
mod japanese_font_tests {
    fn families() -> [egui::FontFamily; 3] {
        [egui::FontFamily::Proportional, egui::FontFamily::Monospace, egui::FontFamily::Name("semibold".into())]
    }

    fn ctx_with(craft: &'static [designcraft_fonts::CraftFont]) -> egui::Context {
        let ctx = egui::Context::default();
        ctx.set_fonts(super::font_definitions(craft, "ja"));
        let mut output = ctx.run_ui(egui::RawInput::default(), |_| {});
        output.textures_delta.clear();
        ctx
    }

    #[test]
    fn japanese_ui_glyphs_are_available_in_every_family() {
        if designcraft_fonts::CRAFT_FONTS.is_empty() {
            eprintln!("skipped: built without craft-fonts (set CRAFT_FONTS_DIR to a checkout)");
            return;
        }
        let ctx = ctx_with(designcraft_fonts::CRAFT_FONTS);
        ctx.fonts_mut(|fonts| {
            for family in families() {
                let font = egui::FontId::new(13.0, family);
                // Real glyphs (no tofu) for every character.
                for ch in "日本語の文字縦書き横書き組み方向ルビ圏点".chars() {
                    assert!(fonts.has_glyph(&font, ch), "missing {ch} in {font:?}");
                }
            }
        });
        // The UI face is BIZ UDPGothic, ahead of the Mincho faces.
        let defs = super::font_definitions(designcraft_fonts::CRAFT_FONTS, "ja");
        let stack = &defs.families[&egui::FontFamily::Proportional];
        assert_eq!(stack.iter().find(|n| n.starts_with("craft-")).map(String::as_str), Some("craft-BIZ UDPGothic-Regular"));
    }

    /// Without craft-fonts, an installed Japanese face gives the interface its glyphs (here a
    /// synthetic stand-in, so the test doesn't depend on the machine's fonts).
    #[test]
    fn installed_fonts_give_japanese_glyphs_without_craft_fonts() {
        let bytes = designcraft_fonts::testing::font_with("DC UI Japanese", &['日', '本', '語']).unwrap();
        let system = [super::SystemFont::new("Jpan", "DC UI Japanese".into(), bytes, 0)];
        let mut defs = super::font_definitions(&[], "ja");
        super::add_system_fallbacks(&mut defs, &system, "ja");
        for family in families() {
            assert_eq!(defs.families[&family].last().map(String::as_str), Some("system-DC UI Japanese"), "{family:?}");
        }
        let ctx = egui::Context::default();
        ctx.set_fonts(defs);
        let mut output = ctx.run_ui(egui::RawInput::default(), |_| {});
        output.textures_delta.clear();
        ctx.fonts_mut(|fonts| {
            let font = egui::FontId::new(13.0, egui::FontFamily::Proportional);
            for ch in "日本語".chars() {
                assert!(fonts.has_glyph(&font, ch), "missing {ch}");
            }
        });
    }

    /// A fallback face with other line metrics lands on the UI face's baseline: egui puts a glyph
    /// at `ascent + (primary line height − its line height) / 2`, plus the tweak.
    #[test]
    fn fallback_faces_share_the_ui_baseline() {
        let ui = designcraft_fonts::line_metrics(super::UI_FONT, 0).unwrap();
        // A synthetic face with a taller ascent and a line gap (Hiragino-like proportions).
        let base = designcraft_fonts::testing::font_with("DC Tall", &['日']).unwrap();
        let tall = designcraft_fonts::testing::with_table_u16(base, b"hhea", 4, 1200u16).unwrap();
        let tall = designcraft_fonts::testing::with_table_u16(tall, b"hhea", 8, 500u16).unwrap();
        let face = designcraft_fonts::line_metrics(&tall, 0).unwrap();
        let tweak = super::baseline_tweak(&tall, 0);
        let em = |m: (f32, f32, f32, f32)| (m.1 / m.0, (m.1 - m.2 + m.3) / m.0);
        let ((ap, hp), (af, hf)) = (em(ui), em(face));
        let baseline = af + 0.5 * (hp - hf) + tweak.y_offset_factor;
        assert!((baseline - ap).abs() < 1e-4, "{baseline} vs {ap}");
        assert!(tweak.y_offset_factor.abs() > 0.01, "the synthetic face needs a shift: {}", tweak.y_offset_factor);
        // The UI face itself needs no shift.
        assert!(super::baseline_tweak(super::UI_FONT, 0).y_offset_factor.abs() < 1e-6);
    }

    /// Korean font names (the font menus show Korean families by their Hangul names) get glyphs
    /// from an installed Korean face, after the Japanese and Chinese ones.
    #[test]
    fn installed_korean_faces_give_hangul_glyphs() {
        let face = |family: &str, chars: &[char], script| {
            super::SystemFont::new(script, family.into(), designcraft_fonts::testing::font_with(family, chars).unwrap(), 0)
        };
        let system = [face("DC UI Korean", &['한', '글'], "Kore"), face("DC UI Japanese", &['日'], "Jpan")];
        let mut defs = super::font_definitions(&[], "ja");
        super::add_system_fallbacks(&mut defs, &system, "ja");
        let stack = &defs.families[&egui::FontFamily::Proportional];
        let at = |n: &str| stack.iter().position(|x| x == n).unwrap();
        assert!(at("system-DC UI Japanese") < at("system-DC UI Korean"));
        let ctx = egui::Context::default();
        ctx.set_fonts(defs);
        let mut output = ctx.run_ui(egui::RawInput::default(), |_| {});
        output.textures_delta.clear();
        ctx.fonts_mut(|fonts| {
            let font = egui::FontId::new(13.0, egui::FontFamily::Proportional);
            assert!(fonts.has_glyph(&font, '한') && fonts.has_glyph(&font, '글'));
        });
    }

    /// Han characters take the interface language's forms: its script's installed face comes
    /// before the built-in CJK faces and the other scripts' faces.
    #[test]
    fn interface_language_orders_the_cjk_faces() {
        let face =
            |family: &str, script| super::SystemFont::new(script, family.into(), designcraft_fonts::testing::font_with(family, &['日']).unwrap(), 0);
        let system = [face("DC Ja", "Jpan"), face("DC Hans", "Hans"), face("DC Hant", "Hant"), face("DC Ko", "Kore")];
        for (lang, first) in [("ja", "DC Ja"), ("zh", "DC Hans"), ("zh-hant", "DC Hant"), ("ko", "DC Ko"), ("", "DC Ja")] {
            let mut defs = super::font_definitions(&[], lang);
            // Stand-in for a built-in CJK face (craft-fonts may be absent in this build).
            defs.families.entry(egui::FontFamily::Proportional).or_default().push("craft-Stand-in".into());
            super::add_system_fallbacks(&mut defs, &system, lang);
            let stack = &defs.families[&egui::FontFamily::Proportional];
            let at = |n: &str| stack.iter().position(|x| x == n).unwrap();
            let first_at = at(&format!("system-{first}"));
            assert!(first_at < at("craft-Stand-in"), "{lang}: {stack:?}");
            for f in &system {
                let name = format!("system-{}", f.family);
                assert!(first_at <= at(&name), "{lang}: {stack:?}");
                // Another language shares the face's data instead of copying the font file.
                assert!(std::sync::Arc::ptr_eq(&defs.font_data[&name], &f.data), "{lang}: {name}");
            }
        }
    }

    /// Tamil, Devanagari and Thai text get glyphs from installed faces, found by family and
    /// checked for coverage, after the CJK faces. Synthetic fonts in a folder of their own stand
    /// in for the machine's.
    #[test]
    fn installed_faces_give_other_scripts_glyphs_after_the_cjk_faces() {
        let dir = std::env::temp_dir().join(format!("dc-ui-script-fonts-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let fonts = [
            ("Noto Sans CJK JP", &['あ', '日'][..]),
            // Listed first for Tamil but without Tamil glyphs: passed over.
            ("Tamil Sangam MN", &['a'][..]),
            // One face for two scripts is added once.
            ("Nirmala UI", &['த', 'க', 'क'][..]),
            ("Noto Sans Thai", &['ก'][..]),
        ];
        for (i, (family, chars)) in fonts.iter().enumerate() {
            std::fs::write(dir.join(format!("font{i}.ttf")), designcraft_fonts::testing::font_with(family, chars).unwrap()).unwrap();
        }
        let db = designcraft_fonts::FontDb::with_font_dirs(vec![dir.clone()]);
        let system = super::installed_ui_fonts(&db, &[]);
        let _ = std::fs::remove_dir_all(&dir);
        let picked: Vec<(&str, &str)> = system.iter().map(|f| (f.script, f.family.as_str())).collect();
        assert_eq!(picked, [("Jpan", "Noto Sans CJK JP"), ("Taml", "Nirmala UI"), ("Thai", "Noto Sans Thai")]);

        let mut defs = super::font_definitions(&[], "ja");
        defs.families.entry(egui::FontFamily::Proportional).or_default().push("craft-Stand-in".into());
        super::add_system_fallbacks(&mut defs, &system, "ja");
        let stack = &defs.families[&egui::FontFamily::Proportional];
        let at = |n: &str| stack.iter().position(|x| x == n).unwrap();
        assert!(at("system-Noto Sans CJK JP") < at("craft-Stand-in"), "{stack:?}");
        assert!(at("craft-Stand-in") < at("system-Nirmala UI") && at("system-Nirmala UI") < at("system-Noto Sans Thai"), "{stack:?}");
        defs.families.entry(egui::FontFamily::Proportional).or_default().retain(|n| n != "craft-Stand-in");
        let ctx = egui::Context::default();
        ctx.set_fonts(defs);
        let mut output = ctx.run_ui(egui::RawInput::default(), |_| {});
        output.textures_delta.clear();
        ctx.fonts_mut(|fonts| {
            let font = egui::FontId::new(13.0, egui::FontFamily::Proportional);
            for ch in ['த', 'क', 'ก'] {
                assert!(fonts.has_glyph(&font, ch), "missing {ch}");
            }
        });
    }

    #[test]
    fn ui_works_without_craft_fonts() {
        let ctx = ctx_with(&[]);
        ctx.fonts_mut(|fonts| {
            // (egui's has_glyph reports false for characters of the face that also supplies the
            // replacement glyph, as Source Sans Semibold does in the semibold stack.)
            let body = egui::FontId::new(13.0, egui::FontFamily::Proportional);
            assert!(fonts.has_glyphs(&body, "Cicero"));
            for family in families() {
                let font = egui::FontId::new(13.0, family);
                let galley = fonts.layout_no_wrap("日本語 Cicero".into(), font, egui::Color32::WHITE);
                assert!(galley.size().x > 0.0);
            }
        });
        // And the real installer works with whatever this build has.
        super::install_fonts(&egui::Context::default(), "ja");
    }
}

#[cfg(test)]
mod ukrainian_font_tests {
    #[test]
    fn every_catalog_cyrillic_glyph_exists_in_the_bundled_ui_fonts() {
        // Check the actual installed face data, without craft-fonts or system fonts.
        // egui's has_glyph can report false for the face providing its replacement glyph.
        let definitions = super::font_definitions(&[], "uk");
        let db = designcraft_fonts::FontDb::with_font_dirs(Vec::new());
        db.set_system_fallback(false);
        let characters: std::collections::HashSet<_> = crate::i18n::ukrainian_catalog()
            .iter()
            .flat_map(|text| text.chars())
            .chain("ҐґЄєІіЇї".chars())
            .filter(|c| matches!(*c, '\u{0400}'..='\u{052f}'))
            .collect();
        for (name, family, style) in
            [("ui", "Inter", "Regular"), ("ui-semibold", "Inter", "SemiBold"), ("mono", "JetBrains Mono", "Regular")]
        {
            let data = &definitions.font_data[name].font;
            let face = db.face(family, style);
            assert_eq!(face.data(), data.as_ref(), "configured UI face {name}");
            for character in &characters {
                assert_ne!(face.glyph_for(*character), 0, "missing {character} in {name}");
            }
        }
    }
}

#[cfg(test)]
mod russian_font_tests {
    #[test]
    fn every_catalog_cyrillic_glyph_exists_in_the_bundled_ui_fonts() {
        // Same check as for Ukrainian: the installed face data only, without craft-fonts or system fonts.
        let definitions = super::font_definitions(&[], "ru");
        let db = designcraft_fonts::FontDb::with_font_dirs(Vec::new());
        db.set_system_fallback(false);
        let characters: std::collections::HashSet<_> = crate::i18n::russian_catalog()
            .iter()
            .flat_map(|text| text.chars())
            .chain("АБВГДЕЁЖЗИЙКЛМНОПРСТУФХЦЧШЩЪЫЬЭЮЯабвгдеёжзийклмнопрстуфхцчшщъыьэюя".chars())
            .filter(|c| matches!(*c, '\u{0400}'..='\u{052f}'))
            .collect();
        for (name, family, style) in
            [("ui", "Inter", "Regular"), ("ui-semibold", "Inter", "SemiBold"), ("mono", "JetBrains Mono", "Regular")]
        {
            let data = &definitions.font_data[name].font;
            let face = db.face(family, style);
            assert_eq!(face.data(), data.as_ref(), "configured UI face {name}");
            for character in &characters {
                assert_ne!(face.glyph_for(*character), 0, "missing {character} in {name}");
            }
        }
    }
}
