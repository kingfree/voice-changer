use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

use crate::constants::UPLOAD_DIR;
use crate::model_slot::{ModelSlot, RVCModelSlot};

pub struct ModelSlotManager {
    model_dir: String,
}

impl ModelSlotManager {
    pub fn new(model_dir: String) -> Self {
        Self { model_dir }
    }

    fn slot_path(&self, slot: usize) -> PathBuf {
        Path::new(&self.model_dir).join(slot.to_string())
    }

    pub fn save_model_slot(&self, slot: usize, info: &ModelSlot) -> std::io::Result<()> {
        let dir = self.slot_path(slot);
        fs::create_dir_all(&dir)?;
        let path = dir.join("params.json");
        let text = serde_json::to_string(info).unwrap();
        fs::write(path, text)
    }

    pub fn load_model_slot(&self, slot: usize) -> Option<ModelSlot> {
        let path = self.slot_path(slot).join("params.json");
        let text = fs::read_to_string(path).ok()?;
        serde_json::from_str(&text).ok()
    }

    pub fn update_model_info(&self, new_data: &str) -> std::io::Result<()> {
        let v: Value = serde_json::from_str(new_data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let slot = v.get("slot").and_then(|v| v.as_u64()).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::Other, "slot missing")
        })? as usize;
        let key = v
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "key missing"))?;
        let val = v.get("val").cloned().unwrap_or(Value::Null);
        let mut info = self.load_model_slot(slot).unwrap_or(ModelSlot::Empty);
        match &mut info {
            ModelSlot::RVC(r) => match key {
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
        self.save_model_slot(slot, &info)
    }

    pub fn store_model_assets(&self, params: &str) -> std::io::Result<()> {
        let v: Value = serde_json::from_str(params)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let slot = v.get("slot").and_then(|v| v.as_u64()).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::Other, "slot missing")
        })? as usize;
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
        let mut info = self.load_model_slot(slot).unwrap_or(ModelSlot::Empty);
        match &mut info {
            ModelSlot::RVC(r) => match name {
                "modelFile" | "model_file" => r.model_file = file.to_string(),
                "indexFile" | "index_file" => r.model_file = file.to_string(),
                _ => {}
            },
            _ => {}
        }
        self.save_model_slot(slot, &info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn save_and_load() {
        let dir = tempdir().unwrap();
        let manager = ModelSlotManager::new(dir.path().to_str().unwrap().to_string());
        let info = ModelSlot::RVC(RVCModelSlot { model_file: "a.pth".into(), sampling_rate: 48000 });
        manager.save_model_slot(0, &info).unwrap();
        let loaded = manager.load_model_slot(0).unwrap();
        match loaded {
            ModelSlot::RVC(r) => {
                assert_eq!(r.model_file, "a.pth");
                assert_eq!(r.sampling_rate, 48000);
            }
            _ => panic!("invalid slot"),
        }
    }

    #[test]
    fn update_model_info_modifies_file() {
        let dir = tempdir().unwrap();
        let manager = ModelSlotManager::new(dir.path().to_str().unwrap().to_string());
        let info = ModelSlot::RVC(RVCModelSlot { model_file: "a.pth".into(), sampling_rate: 48000 });
        manager.save_model_slot(0, &info).unwrap();
        let data = serde_json::json!({"slot":0,"key":"samplingRate","val":44100}).to_string();
        manager.update_model_info(&data).unwrap();
        let loaded = manager.load_model_slot(0).unwrap();
        match loaded {
            ModelSlot::RVC(r) => {
                assert_eq!(r.sampling_rate, 44100);
            }
            _ => panic!("invalid"),
        }
    }

    #[test]
    fn store_model_assets_moves_file() {
        let dir = tempdir().unwrap();
        let upload_dir = Path::new(UPLOAD_DIR);
        fs::create_dir_all(upload_dir).unwrap();
        let file_path = upload_dir.join("test.txt");
        fs::write(&file_path, b"data").unwrap();
        let manager = ModelSlotManager::new(dir.path().to_str().unwrap().to_string());
        manager.save_model_slot(0, &ModelSlot::RVC(RVCModelSlot::default())).unwrap();
        let params = serde_json::json!({"slot":0,"file":"test.txt","name":"modelFile"}).to_string();
        manager.store_model_assets(&params).unwrap();
        assert!(dir.path().join("0").join("test.txt").exists());
        let loaded = manager.load_model_slot(0).unwrap();
        match loaded {
            ModelSlot::RVC(r) => assert_eq!(r.model_file, "test.txt"),
            _ => panic!("invalid"),
        }
    }
}

