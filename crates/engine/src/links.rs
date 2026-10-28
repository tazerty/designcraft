//! Community and project links of the upstream (Help menu, About, start screen, CLI, MCP).
//!
//! Evora build (patch EVORA-CIC-05, exception X-L5-01 of `apps/cicero/UPSTREAM.md` in Evora): the
//! Evora build serves no outside link and no upstream brand. Hiding the links at display level
//! (CIC-05, L6) leaves their text in the served WebAssembly, which the E1 gate counts: the
//! constants are therefore empty here, and `get` answers `None` for an empty link.

use serde_json::{Value, json};

/// The app's slug upstream (kept: identifiers, not shown).
pub const APP: &str = "designcraft";
/// The upstream community chat (none in the Evora build).
pub const DISCORD: &str = "";
/// The upstream website (none in the Evora build).
pub const WEBSITE: &str = "";
/// The app's page on the upstream website (none in the Evora build).
pub const APP_PAGE: &str = "";
/// The source repository (none in the Evora build).
pub const GITHUB: &str = "";
/// Where to report bugs upstream (none in the Evora build).
pub const ISSUES: &str = "";

/// All links as JSON (the `app.links` command).
pub fn all() -> Value {
    json!({
        "discord": DISCORD,
        "website": WEBSITE,
        "appPage": APP_PAGE,
        "github": GITHUB,
        "issues": ISSUES,
    })
}

/// A link by key (`discord`, `website`, `appPage`, `github`, `issues`); `None` for an empty one.
pub fn get(key: &str) -> Option<&'static str> {
    let link = match key {
        "discord" => DISCORD,
        "website" => WEBSITE,
        "appPage" => APP_PAGE,
        "github" => GITHUB,
        "issues" => ISSUES,
        _ => return None,
    };
    (!link.is_empty()).then_some(link)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Evora build (X-L5-01): no outside link is served.
    #[test]
    fn evora_build_serves_no_outside_link() {
        for k in ["discord", "website", "appPage", "github", "issues"] {
            assert_eq!(get(k), None, "{k}");
            assert_eq!(all()[k], "");
        }
    }
}
