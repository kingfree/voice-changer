use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use crate::constants::STORED_SETTING_FILE;
use crate::voice_changer::VoiceChanger;

use crate::voice_changer_params::VoiceChangerParams;

#[derive(Debug, Clone, Serialize)]
pub struct VoiceChangerManagerSettings {
    pub model_slot_index: i32,
    pub pass_through: bool,
}

impl Default for VoiceChangerManagerSettings {
    fn default() -> Self {
        Self {
            model_slot_index: -1,
            pass_through: false,
        }
    }
}

impl VoiceChangerManagerSettings {
    fn update_from_map(&mut self, map: &HashMap<String, Value>) {
        if let Some(v) = map.get("modelSlotIndex").and_then(|v| v.as_i64()) {
            self.model_slot_index = v as i32;
        }
        if let Some(v) = map.get("passThrough").and_then(|v| v.as_bool()) {
            self.pass_through = v;
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoadModelParamFile {
    pub name: String,
    pub dir: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadModelRequest {
    pub voice_changer_type: String,
    pub slot: i32,
    pub is_sample_mode: bool,
    pub sample_id: String,
    pub files: Vec<LoadModelParamFile>,
    pub params: serde_json::Value,
}

pub struct VoiceChangerManager {
    params: VoiceChangerParams,
    settings: RwLock<VoiceChangerManagerSettings>,
    model_path: RwLock<Option<String>>,
    emit_callback: RwLock<Option<Box<dyn Fn(Vec<f32>) + Send + Sync>>>,
    voice_changer: VoiceChanger,
    stored_setting: RwLock<HashMap<String, Value>>,
}

static INSTANCE: OnceCell<VoiceChangerManager> = OnceCell::new();

impl VoiceChangerManager {
    pub fn get_instance(params: VoiceChangerParams) -> &'static VoiceChangerManager {
        INSTANCE.get_or_init(|| {
            let mut m = Self {
                params,
                settings: RwLock::new(VoiceChangerManagerSettings::default()),
                model_path: RwLock::new(None),
                emit_callback: RwLock::new(None),
                voice_changer: VoiceChanger::new(),
                stored_setting: RwLock::new(HashMap::new()),
            };
            m.load_stored_settings();
            m
        })
    }

    pub fn model_dir(&self) -> &str {
        &self.params.model_dir
    }

    pub fn load_model(&self, params: LoadModelRequest) -> serde_json::Value {
        let slot_dir = Path::new(&self.params.model_dir).join(params.slot.to_string());
        std::fs::create_dir_all(&slot_dir).ok();

        let mut first: Option<PathBuf> = None;
        for f in params.files {
            let src = Path::new("upload_dir").join(&f.dir).join(&f.name);
            let dst_dir = slot_dir.join(&f.dir);
            if std::fs::create_dir_all(&dst_dir).is_ok() {
                let dst = dst_dir.join(&f.name);
                let moved = std::fs::rename(&src, &dst).or_else(|_| {
                    std::fs::copy(&src, &dst).and_then(|_| std::fs::remove_file(&src))
                });
                if moved.is_ok() && first.is_none() {
                    first = Some(dst);
                }
            }
        }
        if let Some(p) = first {
            *self.model_path.write().unwrap() = Some(p.to_string_lossy().to_string());
        }

        self.get_info()
    }

    pub fn change_voice(&self, input: &[i16]) -> Vec<i16> {
        if self.settings.read().unwrap().pass_through {
            return input.to_vec();
        }
        self.voice_changer.change_voice(input)
    }

    pub fn clear_prev_audio(&self) {
        self.voice_changer.clear_prev_audio();
    }

    pub fn update_settings(&self, key: &str, val: serde_json::Value) -> serde_json::Value {
        {
            let mut settings = self.settings.write().unwrap();
            match key {
                "modelSlotIndex" => {
                    if let Some(v) = val.as_i64() {
                        settings.model_slot_index = v as i32;
                    }
                }
                "passThrough" => {
                    if let Some(v) = val.as_bool() {
                        settings.pass_through = v;
                    }
                }
                _ => {}
            }
        }
        self.voice_changer.update_settings(key, val.clone());
        self.store_setting(key, &val);
        self.get_info()
    }

    pub fn get_info(&self) -> serde_json::Value {
        let settings = self.settings.read().unwrap();
        let vc_info = self.voice_changer.get_info();
        json!({
            "status": "OK",
            "settings": &*settings,
            "voiceChanger": vc_info,
            "modelPath": self.model_path.read().unwrap().clone(),
        })
    }

    pub fn export_to_onnx(&self) -> bool {
        self.voice_changer.export_to_onnx()
    }

    pub fn merge_models(&self, request: &str) -> serde_json::Value {
        self.voice_changer.merge_models(request);
        self.get_info()
    }

    pub fn get_performance(&self) -> serde_json::Value {
        let perf = self.voice_changer.get_info().performance;
        json!({ "status": "OK", "performance": perf })
    }

    pub fn update_model_default(&self) -> serde_json::Value {
        self.voice_changer.update_model_default();
        self.get_info()
    }

    pub fn update_model_info(&self, new_data: &str) -> serde_json::Value {
        self.voice_changer.update_model_info(new_data);
        self.get_info()
    }

    pub fn upload_model_assets(&self, params: &str) -> serde_json::Value {
        self.voice_changer.upload_model_assets(params);
        self.get_info()
    }
}

impl VoiceChangerManager {
    pub fn set_emit_to<F>(&self, cb: F)
    where
        F: Fn(Vec<f32>) + Send + Sync + 'static,
    {
        let mut lock = self.emit_callback.write().unwrap();
        *lock = Some(Box::new(cb));
    }

    pub fn emit_performance(&self, perf: Vec<f32>) {
        if let Some(cb) = &*self.emit_callback.read().unwrap() {
            cb(perf);
        }
    }

    fn store_setting(&self, key: &str, val: &Value) {
        let mut map = self.stored_setting.write().unwrap();
        map.insert(key.to_string(), val.clone());
        if let Ok(text) = serde_json::to_string(&*map) {
            let _ = std::fs::write(STORED_SETTING_FILE, text);
        }
    }

    fn load_stored_settings(&mut self) {
        if let Ok(text) = std::fs::read_to_string(STORED_SETTING_FILE) {
            if let Ok(map) = serde_json::from_str::<HashMap<String, Value>>(&text) {
                for (k, v) in &map {
                    self.update_settings(k, v.clone());
                }
                *self.stored_setting.write().unwrap() = map;
            }
        }
    }

    #[cfg(test)]
    pub fn reset(&self) {
        *self.settings.write().unwrap() = VoiceChangerManagerSettings::default();
        self.voice_changer.reset();
        self.stored_setting.write().unwrap().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_model_moves_files() {
        let dir_path = Path::new("m");
        std::fs::create_dir_all(dir_path).unwrap();
        let upload_dir = Path::new("upload_dir");
        std::fs::create_dir_all(upload_dir).unwrap();
        let src = upload_dir.join("model.pth");
        std::fs::write(&src, b"dummy").unwrap();

        let params = VoiceChangerParams {
            model_dir: dir_path.to_str().unwrap().into(),
            content_vec_500: String::new(),
            content_vec_500_onnx: String::new(),
            content_vec_500_onnx_on: false,
            hubert_base: String::new(),
            hubert_base_jp: String::new(),
            hubert_soft: String::new(),
            nsf_hifigan: String::new(),
            sample_mode: String::new(),
            crepe_onnx_full: String::new(),
            crepe_onnx_tiny: String::new(),
            rmvpe: String::new(),
            rmvpe_onnx: String::new(),
            whisper_tiny: String::new(),
        };
        let manager = VoiceChangerManager::get_instance(params);
        #[cfg(test)]
        manager.reset();

        let req = LoadModelRequest {
            voice_changer_type: "RVC".into(),
            slot: 0,
            is_sample_mode: false,
            sample_id: String::new(),
            files: vec![LoadModelParamFile {
                name: "model.pth".into(),
                dir: String::new(),
            }],
            params: serde_json::json!({}),
        };

        manager.load_model(req);

        let dst = dir_path.join("0").join("model.pth");
        assert!(dst.exists());
    }
}
