//! Presets: a Tool's options saved under a name the user chooses. Rantakit
//! ships none of its own (ADR 0003).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::catalogue::ToolId;
use crate::history::{new_id, now_ms};
use crate::store::{JsonFile, Result};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub id: String,
    /// The Tool it belongs to, by `ToolId::as_str`.
    pub tool: String,
    pub name: String,
    /// The Tool's options as the frontend keeps them; only the frontend reads them.
    pub options: Value,
}

#[derive(Default, Serialize, Deserialize)]
struct PresetsFile {
    /// Oldest first, the order they are listed in.
    presets: Vec<Preset>,
}

pub struct Presets(JsonFile<PresetsFile>);

impl Presets {
    pub fn new(path: PathBuf) -> Self {
        Self(JsonFile::new(path))
    }

    /// The Presets of one Tool, oldest first, with those saved under an id the
    /// Tool had before 0.4.1: Ubah ke JPG lists those of Ubah Format Foto and
    /// PDF ke JPG, whose options the frontend reads as before.
    pub fn list(&self, tool: &str) -> Result<Vec<Preset>> {
        let mut presets = self.0.read()?.presets;
        let wanted = ToolId::parse(tool);
        presets.retain(|preset| {
            preset.tool == tool || (wanted.is_some() && ToolId::parse(&preset.tool) == wanted)
        });
        Ok(presets)
    }

    pub fn save(&self, tool: &str, name: &str, options: Value) -> Result<Preset> {
        let preset = Preset {
            id: new_id(now_ms()),
            tool: tool.into(),
            name: name.trim().into(),
            options,
        };
        self.0.update(|file| file.presets.push(preset.clone()))?;
        Ok(preset)
    }

    pub fn rename(&self, id: &str, name: &str) -> Result<()> {
        self.0.update(|file| {
            if let Some(preset) = file.presets.iter_mut().find(|p| p.id == id) {
                preset.name = name.trim().into();
            }
        })
    }

    pub fn remove(&self, id: &str) -> Result<()> {
        self.0.update(|file| file.presets.retain(|p| p.id != id))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn presets(test: &str) -> Presets {
        let dir = std::env::temp_dir()
            .join("rantakit-presets-tests")
            .join(test);
        let _ = std::fs::remove_dir_all(&dir);
        Presets::new(dir.join("presets.json"))
    }

    #[test]
    fn each_tool_lists_its_own_presets_in_the_order_saved() {
        let presets = presets("list");
        let options = json!({ "mode": "limit", "limit": 200, "unit": "KB" });
        let cpns = presets
            .save("kompres-foto", "  Pas foto CPNS  ", options.clone())
            .unwrap();
        presets.save("kompres-pdf", "Ijazah", json!({})).unwrap();
        presets.save("kompres-foto", "Beasiswa", json!({})).unwrap();

        let foto = presets.list("kompres-foto").unwrap();
        let names: Vec<&str> = foto.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["Pas foto CPNS", "Beasiswa"]);
        assert_eq!(foto[0], cpns);
        assert_eq!(foto[0].options, options);
    }

    #[test]
    fn a_folded_tool_keeps_the_presets_saved_under_its_old_ids() {
        let presets = presets("legacy");
        presets.save("ubah-format-foto", "Foto ke JPG", json!({})).unwrap();
        presets.save("pdf-ke-jpg", "Halaman ke JPG", json!({})).unwrap();
        presets.save("jpg-ke-pdf", "Scan A4", json!({})).unwrap();
        presets.save("ubah-ke-jpg", "Baru", json!({})).unwrap();

        let names = |tool| -> Vec<String> {
            presets.list(tool).unwrap().into_iter().map(|p| p.name).collect()
        };
        assert_eq!(names("ubah-ke-jpg"), ["Foto ke JPG", "Halaman ke JPG", "Baru"]);
        assert_eq!(names("ubah-ke-pdf"), ["Scan A4"]);
        assert!(names("ubah-ke-png").is_empty());
    }

    #[test]
    fn presets_can_be_renamed_and_removed() {
        let presets = presets("manage");
        let a = presets.save("kompres-foto", "A", json!({})).unwrap();
        let b = presets.save("kompres-foto", "B", json!({})).unwrap();

        presets.rename(&a.id, "Lamaran kerja").unwrap();
        presets.remove(&b.id).unwrap();

        let left = presets.list("kompres-foto").unwrap();
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].name, "Lamaran kerja");
    }
}
