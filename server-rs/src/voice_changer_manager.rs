use once_cell::sync::OnceCell;
use serde::Serialize;
use serde_json::json;
use std::sync::RwLock;

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

pub struct VoiceChangerManager {
    params: VoiceChangerParams,
    settings: RwLock<VoiceChangerManagerSettings>,
    model_path: RwLock<Option<String>>,
    emit_callback: RwLock<Option<Box<dyn Fn(Vec<f32>) + Send + Sync>>>,
}

static INSTANCE: OnceCell<VoiceChangerManager> = OnceCell::new();

impl VoiceChangerManager {
    pub fn get_instance(params: VoiceChangerParams) -> &'static VoiceChangerManager {
        INSTANCE.get_or_init(|| Self {
            params,
            settings: RwLock::new(VoiceChangerManagerSettings::default()),
            model_path: RwLock::new(None),
            emit_callback: RwLock::new(None),
        })
    }

    pub fn load_model(&self, path: String) {
        let mut guard = self.model_path.write().unwrap();
        *guard = Some(path);
    }

    pub fn change_voice(&self, input: &[i16]) -> Vec<i16> {
        // placeholder implementation just echoes the input
        input.to_vec()
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
        self.get_info()
    }

    pub fn get_info(&self) -> serde_json::Value {
        let settings = self.settings.read().unwrap();
        json!({
            "status": "OK",
            "settings": &*settings,
            "modelPath": self.model_path.read().unwrap().clone(),
        })
    }

    pub fn export_to_onnx(&self) -> bool {
        // placeholder always returns false
        false
    }

    pub fn merge_models(&self, _request: &str) -> serde_json::Value {
        json!({ "status": "OK" })
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
}
