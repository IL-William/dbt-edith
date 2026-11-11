//! Which theme the page wears on this machine, whether it follows the system,
//! and what the Halloween theme adds (0150, 0106).
//!
//! Three rules hold here:
//!   - the theme is the user's, not the project's: it is the same in every
//!     project, like the tour offer (0074), so `look.json` sits in the config
//!     directory and not in the per-project settings (0011).
//!   - the file holds `theme`, `follow`, `apparitions` and `boxes` and nothing
//!     else: Night, not following the system, the apparitions on and the boxes
//!     headstones when the file is missing or not its shape. A `theme` or a
//!     `boxes` the file spells otherwise reads as its default and costs the
//!     rest nothing. Without a config directory nothing can remember either: a
//!     read gives the defaults and a write is a silent no-op that still hands
//!     the value back, so the page keeps the theme for the session it was
//!     picked in.
//!   - a file written by 2.1 (`halloween`, `sound`) is read once: `halloween`
//!     becomes the Halloween theme, `sound` is dropped, and the next write
//!     leaves neither key behind.

use std::path::{Path, PathBuf};

/// Where the theme is remembered, beside `tour.json` (0011).
pub fn look_path(config_dir: &Path) -> PathBuf {
    config_dir.join("look.json")
}

/// The colours of the page. An enum, not a free string, so the page cannot
/// make the server keep a name no stylesheet has.
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Night,
    Graphite,
    Light,
    Halloween,
}

impl Theme {
    /// The name `data-theme` carries and `look.json` keeps, the one value of the
    /// look the page is served with (0170).
    pub fn id(self) -> &'static str {
        match self {
            Theme::Night => "night",
            Theme::Graphite => "graphite",
            Theme::Light => "light",
            Theme::Halloween => "halloween",
        }
    }
}

/// The shape of a lineage box under the Halloween theme. An enum, not a free string, so
/// the page cannot make the server keep a name no stylesheet draws.
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Boxes {
    #[default]
    Tombstone,
    Coffin,
    Grimoire,
}

/// A `boxes` the file holds in another spelling reads as the default, so a
/// file edited by hand or written by a later version never loses the rest.
fn boxes_or_default<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Boxes, D::Error> {
    let v = <serde_json::Value as serde::Deserialize>::deserialize(d)?;
    Ok(serde_json::from_value(v).unwrap_or_default())
}

/// A `theme` the file holds in another spelling reads as absent, so a file
/// edited by hand or written by a later version never loses the rest.
fn theme_or_none<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Theme>, D::Error> {
    let v = <serde_json::Value as serde::Deserialize>::deserialize(d)?;
    Ok(serde_json::from_value(v).ok())
}

/// What `look.json` holds. The theme is Night until the user picks one, the
/// system is not followed until they ask for it, and the apparitions are on,
/// since they are what the Halloween theme does once it is on, and are there to
/// be switched off.
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(from = "Stored")]
pub struct Kept {
    pub theme: Theme,
    /// Light while the system is light and Night once it is dark, over `theme`.
    pub follow: bool,
    pub apparitions: bool,
    pub boxes: Boxes,
}

/// What a file may hold, 2.1's keys included, before it is a `Kept`.
#[derive(serde::Deserialize)]
struct Stored {
    #[serde(default, deserialize_with = "theme_or_none")]
    theme: Option<Theme>,
    /// 2.1's switch for the one look it had.
    #[serde(default)]
    halloween: bool,
    #[serde(default)]
    follow: bool,
    /// On in a file written before the switch existed, which has no such key.
    #[serde(default = "on")]
    apparitions: bool,
    /// Headstones in a file written before the choice existed.
    #[serde(default, deserialize_with = "boxes_or_default")]
    boxes: Boxes,
}

impl From<Stored> for Kept {
    fn from(s: Stored) -> Self {
        let old = if s.halloween { Theme::Halloween } else { Theme::Night };
        Kept { theme: s.theme.unwrap_or(old), follow: s.follow, apparitions: s.apparitions, boxes: s.boxes }
    }
}

fn on() -> bool {
    true
}

impl Default for Kept {
    fn default() -> Self {
        Kept { theme: Theme::Night, follow: false, apparitions: true, boxes: Boxes::Tombstone }
    }
}

/// The theme as kept on this machine.
pub struct Look {
    /// `look.json`, or None without a config directory.
    pub path: Option<PathBuf>,
}

impl Look {
    /// The theme kept, or the defaults when there is none or it is unreadable.
    pub fn read(&self) -> Kept {
        self.path.as_deref().and_then(|p| std::fs::read(p).ok()).and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
    }

    /// Keeps `kept` and returns it. A file that cannot be written loses the
    /// theme at the next start and nothing worse, so the failure is not raised.
    pub fn write(&self, kept: Kept) -> Kept {
        if let Some(path) = &self.path {
            let body = format!("{:#}\n", serde_json::json!(kept));
            let _ = crate::settings::write_atomic(path, body.as_bytes());
        }
        kept
    }
}

/// The page with the kept theme on its `<html>` tag, so a reload opens in it
/// instead of flashing Night until `/api/look` answers (0170). The page cannot
/// do this itself: its script is not allowed inline, and `/` is served without
/// the key. Only the theme's name goes out, from the closed list, and never
/// `follow`: the server cannot know the system's scheme, so a page that follows
/// it keeps Night until the page applies the look. The swap stays inside the
/// `<html ...>` tag, so a `data-theme` later in the page is never touched.
pub fn dress<'a>(page: &'a [u8], kept: &Kept) -> std::borrow::Cow<'a, [u8]> {
    const BARE: &[u8] = b"data-theme=\"night\"";
    let plain = std::borrow::Cow::Borrowed(page);
    if kept.follow || kept.theme == Theme::Night {
        return plain;
    }
    let find = |hay: &[u8], needle: &[u8]| hay.windows(needle.len()).position(|w| w == needle);
    let Some(open) = find(page, b"<html") else { return plain };
    let Some(len) = page[open..].iter().position(|&b| b == b'>') else { return plain };
    let tag = &page[open..open + len];
    let Some(at) = find(tag, BARE).map(|i| open + i) else { return plain };
    let mut dressed = Vec::with_capacity(page.len() + 16);
    dressed.extend_from_slice(&page[..at]);
    dressed.extend_from_slice(format!("data-theme=\"{}\"", kept.theme.id()).as_bytes());
    dressed.extend_from_slice(&page[at + BARE.len()..]);
    std::borrow::Cow::Owned(dressed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dbt-edith-look-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn kept(theme: Theme, follow: bool, apparitions: bool, boxes: Boxes) -> Kept {
        Kept { theme, follow, apparitions, boxes }
    }

    #[test]
    fn the_theme_round_trips_and_holds_nothing_else() {
        let dir = temp("round");
        let look = Look { path: Some(look_path(&dir.join("config"))) };
        assert_eq!(look.read(), Kept::default(), "Night before anyone chose");
        assert!(!dir.exists(), "reading writes nothing");
        let picked = kept(Theme::Light, true, false, Boxes::Grimoire);
        assert_eq!(look.write(picked), picked, "creating the directory");
        assert_eq!(Look { path: look.path.clone() }.read(), picked, "another process reads it");
        let file: serde_json::Value = serde_json::from_slice(&std::fs::read(look.path.as_ref().unwrap()).unwrap()).unwrap();
        let keys: Vec<&String> = file.as_object().unwrap().keys().collect();
        assert_eq!(keys, ["apparitions", "boxes", "follow", "theme"], "nothing else is kept");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_page_wears_the_kept_theme_on_its_html_tag_alone() {
        let page = b"<!doctype html>\n<html lang=\"en\" data-theme=\"night\">\n<body data-theme=\"night\"></body></html>";
        let light = dress(page, &kept(Theme::Light, false, true, Boxes::Tombstone));
        assert_eq!(
            light.as_ref(),
            &b"<!doctype html>\n<html lang=\"en\" data-theme=\"light\">\n<body data-theme=\"night\"></body></html>"[..],
            "the html tag changes and the body's attribute does not"
        );
        let hallows = dress(page, &kept(Theme::Halloween, false, true, Boxes::Tombstone));
        assert!(String::from_utf8_lossy(&hallows).contains("<html lang=\"en\" data-theme=\"halloween\">"));
        for same in [kept(Theme::Night, false, true, Boxes::Tombstone), kept(Theme::Light, true, true, Boxes::Tombstone)] {
            assert!(matches!(dress(page, &same), std::borrow::Cow::Borrowed(_)), "Night and Follow leave the page as it is");
        }
        let late = b"<html lang=\"en\"><body data-theme=\"night\"></body></html>";
        assert_eq!(
            dress(late, &kept(Theme::Light, false, true, Boxes::Tombstone)).as_ref(),
            &late[..],
            "a data-theme after the tag is untouched"
        );
        let none = b"<html><body></body></html>";
        assert!(
            matches!(dress(none, &kept(Theme::Light, false, true, Boxes::Tombstone)), std::borrow::Cow::Borrowed(_)),
            "no attribute, no change"
        );
    }

    #[test]
    fn a_file_that_is_not_its_shape_reads_as_the_defaults() {
        let dir = temp("corrupt");
        let path = look_path(&dir);
        let look = Look { path: Some(path.clone()) };
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&path, "not json").unwrap();
        assert_eq!(look.read(), Kept::default());
        std::fs::write(&path, r#"{"halloween":"yes"}"#).unwrap();
        assert_eq!(look.read(), Kept::default(), "a wrong type is not half read");
        std::fs::write(&path, r#"{"theme":"light","follow":"yes"}"#).unwrap();
        assert_eq!(look.read(), Kept::default(), "nor is a follow that is no boolean");
        std::fs::write(&path, r#"{"theme":"graphite"}"#).unwrap();
        assert_eq!(look.read(), kept(Theme::Graphite, false, true, Boxes::Tombstone), "what a file lacks takes its default");
        std::fs::write(&path, r#"{"theme":"light","follow":true,"apparitions":false,"boxes":"coffin"}"#).unwrap();
        assert_eq!(look.read(), kept(Theme::Light, true, false, Boxes::Coffin));
        for odd in [r#""skull""#, "7", "null", r#""Coffin""#] {
            std::fs::write(&path, format!(r#"{{"theme":"halloween","apparitions":false,"boxes":{odd}}}"#)).unwrap();
            assert_eq!(look.read(), kept(Theme::Halloween, false, false, Boxes::Tombstone), "{odd} reads as headstone and keeps the rest");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A file 2.1 wrote keeps its look: the switch becomes the theme, and the
    /// sound, which is gone, is dropped by the next write.
    #[test]
    fn a_file_from_2_1_is_read_and_rewritten_without_its_old_keys() {
        let dir = temp("migrate");
        let path = look_path(&dir);
        let look = Look { path: Some(path.clone()) };
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&path, r#"{"halloween":true}"#).unwrap();
        assert_eq!(look.read(), kept(Theme::Halloween, false, true, Boxes::Tombstone), "the switch is the theme");
        std::fs::write(&path, r#"{"halloween":true,"sound":true,"apparitions":false,"boxes":"coffin"}"#).unwrap();
        let old = look.read();
        assert_eq!(old, kept(Theme::Halloween, false, false, Boxes::Coffin), "the sound is ignored");
        look.write(old);
        let file: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let keys: Vec<&String> = file.as_object().unwrap().keys().collect();
        assert_eq!(keys, ["apparitions", "boxes", "follow", "theme"], "neither old key is written back");
        std::fs::write(&path, r#"{"halloween":false}"#).unwrap();
        assert_eq!(look.read(), Kept::default(), "the usual look is Night");
        std::fs::write(&path, r#"{"theme":"light","halloween":true}"#).unwrap();
        assert_eq!(look.read().theme, Theme::Light, "a theme beats the old switch");
        std::fs::write(&path, r#"{"theme":"sepia","apparitions":false,"boxes":"coffin"}"#).unwrap();
        assert_eq!(look.read(), kept(Theme::Night, false, false, Boxes::Coffin), "an unknown theme is Night and costs the rest nothing");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn without_a_config_directory_nothing_is_remembered() {
        let look = Look { path: None };
        let picked = kept(Theme::Graphite, false, false, Boxes::Coffin);
        assert_eq!(look.write(picked), picked, "the value still comes back");
        assert_eq!(look.read(), Kept::default(), "but is not kept");
    }
}
