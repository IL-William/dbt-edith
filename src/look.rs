//! Whether the page wears its Halloween look on this machine, and whether that
//! look makes sounds (0106).
//!
//! Two rules hold here:
//!   - the look is the user's, not the project's: it is the same in every
//!     project, like the tour offer (0074), so `look.json` sits in the config
//!     directory and not in the per-project settings (0011).
//!   - the file holds `halloween`, `sound`, `apparitions` and `boxes` and
//!     nothing else: the look and the sound are off, the apparitions on and the
//!     boxes tombstones when the file is missing or not its shape. A `boxes`
//!     the file spells otherwise reads as tombstone and costs the rest nothing. Without a config directory nothing
//!     can remember either: a read gives the defaults and a write is a silent
//!     no-op that still hands the value back, so the page keeps the look for the
//!     session it was switched on in.

use std::path::{Path, PathBuf};

/// Where the look is remembered, beside `tour.json` (0011).
pub fn look_path(config_dir: &Path) -> PathBuf {
    config_dir.join("look.json")
}

/// The shape of a lineage box under the look. An enum, not a free string, so
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

/// What `look.json` holds. The look is switched on by the user and the sound
/// is muted until they ask for it; the apparitions are on, since they are
/// what the look does once it is on, and are there to be switched off.
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Kept {
    #[serde(default)]
    pub halloween: bool,
    #[serde(default)]
    pub sound: bool,
    /// On in a file written before the switch existed, which has no such key.
    #[serde(default = "on")]
    pub apparitions: bool,
    /// Tombstone in a file written before the choice existed.
    #[serde(default, deserialize_with = "boxes_or_default")]
    pub boxes: Boxes,
}

fn on() -> bool {
    true
}

impl Default for Kept {
    fn default() -> Self {
        Kept { halloween: false, sound: false, apparitions: true, boxes: Boxes::Tombstone }
    }
}

/// The look as kept on this machine.
pub struct Look {
    /// `look.json`, or None without a config directory.
    pub path: Option<PathBuf>,
}

impl Look {
    /// The look kept, or the defaults when there is none or it is unreadable.
    pub fn read(&self) -> Kept {
        self.path
            .as_deref()
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    /// Keeps `kept` and returns it. A file that cannot be written loses the
    /// look at the next start and nothing worse, so the failure is not raised.
    pub fn write(&self, kept: Kept) -> Kept {
        if let Some(path) = &self.path {
            let body = format!("{:#}\n", serde_json::json!(kept));
            let _ = crate::settings::write_atomic(path, body.as_bytes());
        }
        kept
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dbt-edith-look-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn the_look_round_trips_and_holds_nothing_else() {
        let dir = temp("round");
        let look = Look { path: Some(look_path(&dir.join("config"))) };
        assert_eq!(look.read(), Kept::default(), "off before anyone chose");
        assert!(!dir.exists(), "reading writes nothing");
        let on = Kept { halloween: true, sound: true, apparitions: false, boxes: Boxes::Grimoire };
        assert_eq!(look.write(on), on, "creating the directory");
        assert_eq!(Look { path: look.path.clone() }.read(), on, "another process reads it");
        let kept: serde_json::Value = serde_json::from_slice(&std::fs::read(look.path.as_ref().unwrap()).unwrap()).unwrap();
        let keys: Vec<&String> = kept.as_object().unwrap().keys().collect();
        assert_eq!(keys, ["apparitions", "boxes", "halloween", "sound"], "nothing else is kept");
        let _ = std::fs::remove_dir_all(&dir);
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
        std::fs::write(&path, r#"{"halloween":true}"#).unwrap();
        assert_eq!(
            look.read(),
            Kept { halloween: true, sound: false, apparitions: true, boxes: Boxes::Tombstone },
            "a file from before the apparitions switch has them on"
        );
        std::fs::write(&path, r#"{"halloween":true,"sound":true,"apparitions":false}"#).unwrap();
        assert_eq!(
            look.read(),
            Kept { halloween: true, sound: true, apparitions: false, boxes: Boxes::Tombstone },
            "a file from before the boxes choice has tombstones"
        );
        std::fs::write(&path, r#"{"halloween":true,"sound":true,"apparitions":false,"boxes":"coffin"}"#).unwrap();
        assert_eq!(look.read().boxes, Boxes::Coffin);
        for odd in [r#""skull""#, "7", "null", r#""Coffin""#] {
            std::fs::write(&path, format!(r#"{{"halloween":true,"sound":true,"apparitions":false,"boxes":{odd}}}"#)).unwrap();
            assert_eq!(
                look.read(),
                Kept { halloween: true, sound: true, apparitions: false, boxes: Boxes::Tombstone },
                "{odd} reads as tombstone and keeps the rest"
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn without_a_config_directory_nothing_is_remembered() {
        let look = Look { path: None };
        let on = Kept { halloween: true, sound: false, apparitions: false, boxes: Boxes::Coffin };
        assert_eq!(look.write(on), on, "the value still comes back");
        assert_eq!(look.read(), Kept::default(), "but is not kept");
    }
}
