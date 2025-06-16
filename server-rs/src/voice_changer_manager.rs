use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::RwLock;
use std::path::{Path, PathBuf};

use crate::voice_changer_params::VoiceChangerParams;

#[derive(Debug, Clone, Serialize)]
pub struct VoiceChangerManagerSettings {
    pub model_slot_index: i32,
    pub pass_through: bool,
    pub input_sample_rate: i32,
    pub output_sample_rate: i32,
    pub cross_fade_offset_rate: f32,
    pub cross_fade_end_rate: f32,
    pub cross_fade_overlap_size: i32,
    pub record_io: i32,
    pub performance: Vec<f32>,
}

impl Default for VoiceChangerManagerSettings {
    fn default() -> Self {
        Self {
            model_slot_index: -1,
            pass_through: false,
            input_sample_rate: 48000,
            output_sample_rate: 48000,
            cross_fade_offset_rate: 0.1,
            cross_fade_end_rate: 0.9,
            cross_fade_overlap_size: 4096,
            record_io: 0,
            performance: vec![0.0, 0.0, 0.0, 0.0],
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
    prev_audio: RwLock<Vec<i16>>,
}

static INSTANCE: OnceCell<VoiceChangerManager> = OnceCell::new();

impl VoiceChangerManager {
    pub fn get_instance(params: VoiceChangerParams) -> &'static VoiceChangerManager {
        INSTANCE.get_or_init(|| Self {
            params,
            settings: RwLock::new(VoiceChangerManagerSettings::default()),
            model_path: RwLock::new(None),
            emit_callback: RwLock::new(None),
            prev_audio: RwLock::new(Vec::new()),
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
        {
            let settings = self.settings.read().unwrap();
            if settings.pass_through {
                return input.to_vec();
            }
        }

        let (overlap, offset_rate, end_rate) = {
            let s = self.settings.read().unwrap();
            (
                s.cross_fade_overlap_size as usize,
                s.cross_fade_offset_rate,
                s.cross_fade_end_rate,
            )
        };

        let mut out = input.to_vec();
        let mut prev = self.prev_audio.write().unwrap();
        let n = if prev.len() == input.len() {
            overlap.min(input.len())
        } else {
            0
        };
        if n > 0 {
            let (prev_strength, cur_strength) = generate_strength(n, offset_rate, end_rate);
            for i in 0..n {
                let p = prev[i] as f32 * prev_strength[i];
                let c = input[i] as f32 * cur_strength[i];
                out[i] = (p + c) as i16;
            }
        }

        prev.clear();
        let keep = overlap.min(out.len());
        prev.extend_from_slice(&out[out.len() - keep..]);

        out
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
                "inputSampleRate" => {
                    if let Some(v) = val.as_i64() {
                        settings.input_sample_rate = v as i32;
                    }
                }
                "outputSampleRate" => {
                    if let Some(v) = val.as_i64() {
                        settings.output_sample_rate = v as i32;
                    }
                }
                "crossFadeOffsetRate" => {
                    if let Some(v) = val.as_f64() {
                        settings.cross_fade_offset_rate = v as f32;
                    }
                }
                "crossFadeEndRate" => {
                    if let Some(v) = val.as_f64() {
                        settings.cross_fade_end_rate = v as f32;
                    }
                }
                "crossFadeOverlapSize" => {
                    if let Some(v) = val.as_i64() {
                        settings.cross_fade_overlap_size = v as i32;
                    }
                }
                "recordIO" => {
                    if let Some(v) = val.as_i64() {
                        settings.record_io = v as i32;
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

    pub fn get_performance(&self) -> serde_json::Value {
        let settings = self.settings.read().unwrap();
        json!({ "status": "OK", "performance": settings.performance })
    }

    pub fn update_model_default(&self) -> serde_json::Value {
        self.get_info()
    }

    pub fn update_model_info(&self, _new_data: &str) -> serde_json::Value {
        self.get_info()
    }

    pub fn upload_model_assets(&self, _params: &str) -> serde_json::Value {
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

    #[cfg(test)]
    pub fn reset(&self) {
        *self.settings.write().unwrap() = VoiceChangerManagerSettings::default();
        self.prev_audio.write().unwrap().clear();
    }
}

fn generate_strength(size: usize, offset_rate: f32, end_rate: f32) -> (Vec<f32>, Vec<f32>) {
    if size == 0 {
        return (Vec::new(), Vec::new());
    }
    let offset = (size as f32 * offset_rate).round() as usize;
    let end = (size as f32 * end_rate).round() as usize;
    let mut prev = vec![0.0_f32; size];
    let mut cur = vec![0.0_f32; size];
    for i in 0..size {
        if i < offset {
            prev[i] = 1.0;
            cur[i] = 0.0;
        } else if i < end && end > offset {
            let percent = (i - offset) as f32 / (end - offset) as f32;
            prev[i] = (percent * 0.5 * std::f32::consts::PI).cos().powi(2);
            cur[i] = ((1.0 - percent) * 0.5 * std::f32::consts::PI).cos().powi(2);
        } else {
            prev[i] = 0.0;
            cur[i] = 1.0;
        }
    }
    (prev, cur)
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
            files: vec![LoadModelParamFile { name: "model.pth".into(), dir: String::new() }],
            params: serde_json::json!({}),
        };

        manager.load_model(req);

        let dst = dir_path.join("0").join("model.pth");
        assert!(dst.exists());
    }
}
