use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use crate::constants::{MAX_SLOT_NUM, MODEL_DIR_STATIC, UPLOAD_DIR};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelSlot {
    pub slot_index: i32,
    pub voice_changer_type: Option<String>,
    pub name: String,
    pub description: String,
    pub credit: String,
    pub terms_of_use_url: String,
    pub icon_file: String,
    pub speakers: HashMap<i32, String>,
}

impl Default for ModelSlot {
    fn default() -> Self {
        Self {
            slot_index: -1,
            voice_changer_type: None,
            name: String::new(),
            description: String::new(),
            credit: String::new(),
            terms_of_use_url: String::new(),
            icon_file: String::new(),
            speakers: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RVCModelSlot {
    #[serde(flatten)]
    pub base: ModelSlot,
    pub model_file: String,
    pub index_file: String,
    pub default_tune: i32,
    pub default_index_ratio: i32,
    pub default_protect: f32,
    pub is_onnx: bool,
    pub model_type: String,
    pub sampling_rate: i32,
    pub f0: bool,
    pub emb_channels: i32,
    pub emb_output_layer: i32,
    pub use_final_proj: bool,
    pub deprecated: bool,
    pub embedder: String,
    pub sample_id: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ModelSlotEntry {
    RVC(RVCModelSlot),
    Empty,
}

fn set_slot_index(entry: &mut ModelSlotEntry, idx: i32) {
    if let ModelSlotEntry::RVC(r) = entry {
        r.base.slot_index = idx;
    }
}

fn load_static_slot(name: &str) -> Option<ModelSlotEntry> {
    let path = Path::new(MODEL_DIR_STATIC).join(name).join("params.json");
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

impl Default for RVCModelSlot {
    fn default() -> Self {
        Self {
            base: {
                let mut b = ModelSlot::default();
                b.voice_changer_type = Some("RVC".into());
                b.speakers.insert(0, "target".into());
                b
            },
            model_file: String::new(),
            index_file: String::new(),
            default_tune: 0,
            default_index_ratio: 0,
            default_protect: 0.5,
            is_onnx: false,
            model_type: String::new(),
            sampling_rate: 48000,
            f0: true,
            emb_channels: 256,
            emb_output_layer: 9,
            use_final_proj: true,
            deprecated: false,
            embedder: "hubert_base".into(),
            sample_id: String::new(),
            version: "v2".into(),
        }
    }
}

pub struct ModelSlotManager {
    model_dir: String,
    slots: RwLock<Vec<ModelSlotEntry>>,
}

impl ModelSlotManager {
    pub fn new(model_dir: String) -> Self {
        let mgr = Self {
            model_dir,
            slots: RwLock::new(Vec::new()),
        };
        mgr.reload_slots();
        mgr
    }

    fn reload_slots(&self) {
        let mut vec = Vec::new();
        for i in 0..MAX_SLOT_NUM {
            vec.push(self.load_model_slot(i).unwrap_or(ModelSlotEntry::Empty));
        }
        if let Ok(mut guard) = self.slots.write() {
            *guard = vec;
        }
    }

    fn slot_path(&self, slot: usize) -> PathBuf {
        Path::new(&self.model_dir).join(slot.to_string())
    }

    pub fn save_model_slot(&self, slot: usize, info: &ModelSlotEntry) -> std::io::Result<()> {
        let dir = self.slot_path(slot);
        fs::create_dir_all(&dir)?;
        let path = dir.join("params.json");
        let text = serde_json::to_string(info).unwrap();
        fs::write(path, text)?;
        if let Ok(mut slots) = self.slots.write() {
            if slot >= slots.len() {
                slots.resize(slot + 1, ModelSlotEntry::Empty);
            }
            if let Some(s) = slots.get_mut(slot) {
                *s = info.clone();
            }
        }
        Ok(())
    }

    pub fn load_model_slot(&self, slot: usize) -> Option<ModelSlotEntry> {
        let path = self.slot_path(slot).join("params.json");
        let text = fs::read_to_string(path).ok()?;
        serde_json::from_str(&text).ok()
    }

    pub fn update_model_info(&self, new_data: &str) -> std::io::Result<()> {
        let v: Value = serde_json::from_str(new_data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let slot = v
            .get("slot")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "slot missing"))?
            as usize;
        let key = v
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "key missing"))?;
        let val = v.get("val").cloned().unwrap_or(Value::Null);
        let mut info = self
            .load_model_slot(slot)
            .unwrap_or(ModelSlotEntry::RVC(RVCModelSlot::default()));
        match &mut info {
            ModelSlotEntry::RVC(r) => match key {
                "modelFile" | "model_file" => {
                    if let Some(s) = val.as_str() {
                        r.model_file = s.to_string();
                    }
                }
                "samplingRate" | "sampling_rate" => {
                    if let Some(n) = val.as_i64() {
                        r.sampling_rate = n as i32;
                    }
                }
                _ => {}
            },
            _ => {}
        }
        self.save_model_slot(slot, &info)?;
        self.reload_slots();
        Ok(())
    }

    pub fn store_model_assets(&self, params: &str) -> std::io::Result<()> {
        let v: Value = serde_json::from_str(params)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let slot = v
            .get("slot")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "slot missing"))?
            as usize;
        let file = v
            .get("file")
            .and_then(|v| v.as_str())
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "file missing"))?;
        let name = v
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "name missing"))?;
        let src = Path::new(UPLOAD_DIR).join(file);
        let dst_dir = self.slot_path(slot);
        fs::create_dir_all(&dst_dir)?;
        let dst = dst_dir.join(file);
        if fs::rename(&src, &dst).is_err() {
            fs::copy(&src, &dst)?;
            let _ = fs::remove_file(&src);
        }
        let mut info = self.load_model_slot(slot).unwrap_or(ModelSlotEntry::Empty);
        match &mut info {
            ModelSlotEntry::RVC(r) => match name {
                "modelFile" | "model_file" => r.model_file = file.to_string(),
                "indexFile" | "index_file" => r.model_file = file.to_string(),
                _ => {}
            },
            _ => {}
        }
        self.save_model_slot(slot, &info)?;
        self.reload_slots();
        Ok(())
    }

    /// Return all slot information, optionally reloading from disk.
    pub fn get_all_slot_info(&self, reload: bool) -> Vec<ModelSlotEntry> {
        if reload {
            self.reload_slots();
        }
        let mut out = self
            .slots
            .read()
            .map(|v| {
                v.iter()
                    .enumerate()
                    .map(|(i, s)| {
                        let mut e = s.clone();
                        set_slot_index(&mut e, i as i32);
                        e
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|_| Vec::new());
        if let Some(s) = load_static_slot("Beatrice-JVS") {
            out.push(s);
        }
        out
    }

    /// Retrieve a single slot by index.
    pub fn get_slot_info(&self, slot: usize) -> Option<ModelSlotEntry> {
        self.slots
            .read()
            .ok()
            .and_then(|v| v.get(slot).cloned())
            .map(|mut e| {
                set_slot_index(&mut e, slot as i32);
                e
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::cleanup_test_dirs;
    use serial_test::serial;
    use tempfile::tempdir;

    #[test]
    #[serial]
    fn save_and_load() {
        let dir = tempdir().unwrap();
        let manager = ModelSlotManager::new(dir.path().to_str().unwrap().to_string());
        let info = ModelSlotEntry::RVC(RVCModelSlot {
            base: ModelSlot::default(),
            model_file: "a.pth".into(),
            sampling_rate: 48000,
            ..RVCModelSlot::default()
        });
        manager.save_model_slot(0, &info).unwrap();
        let loaded = manager.load_model_slot(0).unwrap();
        match loaded {
            ModelSlotEntry::RVC(r) => {
                assert_eq!(r.model_file, "a.pth");
                assert_eq!(r.sampling_rate, 48000);
            }
            _ => panic!("invalid slot"),
        }
        cleanup_test_dirs();
    }

    #[test]
    #[serial]
    fn update_model_info_modifies_file() {
        let dir = tempdir().unwrap();
        let manager = ModelSlotManager::new(dir.path().to_str().unwrap().to_string());
        let info = ModelSlotEntry::RVC(RVCModelSlot {
            base: ModelSlot::default(),
            model_file: "a.pth".into(),
            sampling_rate: 48000,
            ..RVCModelSlot::default()
        });
        manager.save_model_slot(0, &info).unwrap();
        let data = serde_json::json!({"slot":0,"key":"samplingRate","val":44100}).to_string();
        manager.update_model_info(&data).unwrap();
        let loaded = manager.load_model_slot(0).unwrap();
        match loaded {
            ModelSlotEntry::RVC(r) => {
                assert_eq!(r.sampling_rate, 44100);
            }
            _ => panic!("invalid"),
        }
        cleanup_test_dirs();
    }
}
