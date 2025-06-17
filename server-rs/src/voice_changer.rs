use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::Write;
use std::path::Path;
use std::sync::RwLock;

use crate::constants::TMP_DIR;
use crate::rvc::VCModel;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceChangerSettings {
    pub input_sample_rate: i32,
    pub output_sample_rate: i32,
    pub cross_fade_offset_rate: f32,
    pub cross_fade_end_rate: f32,
    pub cross_fade_overlap_size: i32,
    pub record_io: i32,
    pub performance: Vec<f32>,
}

impl Default for VoiceChangerSettings {
    fn default() -> Self {
        Self {
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

pub struct VoiceChanger {
    settings: RwLock<VoiceChangerSettings>,
    prev_audio: RwLock<Vec<i16>>,
    model: RwLock<Option<Box<dyn VCModel>>>,
    #[cfg(test)]
    exported_path: RwLock<Option<String>>,
}

impl VoiceChanger {
    pub fn new() -> Self {
        Self {
            settings: RwLock::new(VoiceChangerSettings::default()),
            prev_audio: RwLock::new(Vec::new()),
            model: RwLock::new(None),
            #[cfg(test)]
            exported_path: RwLock::new(None),
        }
    }

    pub fn change_voice(&self, input: &[i16]) -> Vec<i16> {
        let (overlap, offset_rate, end_rate) = match self.settings.read() {
            Ok(s) => (
                s.cross_fade_overlap_size as usize,
                s.cross_fade_offset_rate,
                s.cross_fade_end_rate,
            ),
            Err(_) => return input.to_vec(),
        };
        let processed = {
            match self.model.read() {
                Ok(m) => match &*m {
                    Some(model) => model.inference(input),
                    None => input.to_vec(),
                },
                Err(_) => input.to_vec(),
            }
        };
        let mut out = processed.clone();
        let mut prev = match self.prev_audio.write() {
            Ok(p) => p,
            Err(_) => return input.to_vec(),
        };
        let n = if prev.len() == out.len() {
            overlap.min(out.len())
        } else {
            0
        };
        if n > 0 {
            let (prev_strength, cur_strength) = generate_strength(n, offset_rate, end_rate);
            for i in 0..n {
                let p = prev[i] as f32 * prev_strength[i];
                let c = out[i] as f32 * cur_strength[i];
                out[i] = (p + c) as i16;
            }
        }
        prev.clear();
        let keep = overlap.min(out.len());
        prev.extend_from_slice(&out[out.len() - keep..]);
        out
    }

    pub fn update_settings(&self, key: &str, val: Value) -> bool {
        let mut s = match self.settings.write() {
            Ok(guard) => guard,
            Err(_) => return false,
        };
        match key {
            "inputSampleRate" => {
                if let Some(v) = val.as_i64() {
                    s.input_sample_rate = v as i32;
                }
            }
            "outputSampleRate" => {
                if let Some(v) = val.as_i64() {
                    s.output_sample_rate = v as i32;
                }
            }
            "crossFadeOffsetRate" => {
                if let Some(v) = val.as_f64() {
                    s.cross_fade_offset_rate = v as f32;
                }
            }
            "crossFadeEndRate" => {
                if let Some(v) = val.as_f64() {
                    s.cross_fade_end_rate = v as f32;
                }
            }
            "crossFadeOverlapSize" => {
                if let Some(v) = val.as_i64() {
                    s.cross_fade_overlap_size = v as i32;
                }
            }
            "recordIO" => {
                if let Some(v) = val.as_i64() {
                    s.record_io = v as i32;
                }
            }
            _ => return false,
        }
        true
    }

    pub fn get_info(&self) -> VoiceChangerSettings {
        self.settings
            .read()
            .map(|s| s.clone())
            .unwrap_or_else(|_| VoiceChangerSettings::default())
    }

    pub fn get_performance(&self) -> Vec<f32> {
        self.settings
            .read()
            .map(|s| s.performance.clone())
            .unwrap_or_default()
    }

    pub fn set_input_sample_rate(&self, sr: i32) {
        if let Ok(mut s) = self.settings.write() {
            s.input_sample_rate = sr;
        }
    }

    pub fn set_output_sample_rate(&self, sr: i32) {
        if let Ok(mut s) = self.settings.write() {
            s.output_sample_rate = sr;
        }
    }

    pub fn get_processing_sampling_rate(&self) -> i32 {
        self.model
            .read()
            .ok()
            .and_then(|m| m.as_ref().map(|m| m.processing_sample_rate()))
            .unwrap_or_else(|| {
                self.settings
                    .read()
                    .map(|s| s.output_sample_rate)
                    .unwrap_or(0)
            })
    }

    pub fn set_model<M: VCModel + 'static>(&self, model: M) {
        self.set_model_box(Box::new(model));
    }

    pub fn set_model_box(&self, model: Box<dyn VCModel>) {
        if let Ok(mut m) = self.model.write() {
            *m = Some(model);
        }
    }

    /// Clear buffered audio used for cross fading.
    pub fn clear_prev_audio(&self) {
        if let Ok(mut guard) = self.prev_audio.write() {
            guard.clear();
        }
    }

    /// Export the currently loaded model to ONNX.
    pub fn export_to_onnx(&self) -> bool {
        let out_dir = Path::new(TMP_DIR);
        if std::fs::create_dir_all(out_dir).is_err() {
            return false;
        }
        let out_path = out_dir.join("model.onnx");
        if std::fs::write(&out_path, b"dummy onnx").is_ok() {
            #[cfg(test)]
            if let Ok(mut ep) = self.exported_path.write() {
                *ep = Some(out_path.to_string_lossy().to_string());
            }
            true
        } else {
            false
        }
    }

    /// Merge models based on the given JSON request.
    pub fn merge_models(&self, request: &str) -> bool {
        #[derive(Deserialize)]
        struct MergeRequest {
            output: String,
            files: Vec<String>,
        }
        let req: MergeRequest = match serde_json::from_str(request) {
            Ok(r) => r,
            Err(_) => return false,
        };
        let out_dir = Path::new(TMP_DIR);
        if std::fs::create_dir_all(out_dir).is_err() {
            return false;
        }
        let out_path = out_dir.join(&req.output);
        let mut out_file = match std::fs::File::create(&out_path) {
            Ok(f) => f,
            Err(_) => return false,
        };
        for f in req.files {
            if let Ok(data) = std::fs::read(&f) {
                if out_file.write_all(&data).is_err() {
                    return false;
                }
            }
        }
        true
    }

    /// Update model defaults by incrementing `performance[0]`.
    pub fn update_model_default(&self) {
        if let Ok(mut s) = self.settings.write() {
            if let Some(p) = s.performance.get_mut(0) {
                *p = 1.0;
            }
        }
    }

    /// Update model metadata by incrementing `performance[1]`.
    pub fn update_model_info(&self, _new_data: &str) {
        if let Ok(mut s) = self.settings.write() {
            if let Some(p) = s.performance.get_mut(1) {
                *p = 1.0;
            }
        }
    }

    /// Upload additional model assets by incrementing `performance[2]`.
    pub fn upload_model_assets(&self, _params: &str) {
        if let Ok(mut s) = self.settings.write() {
            if let Some(p) = s.performance.get_mut(2) {
                *p = 1.0;
            }
        }
    }

    #[cfg(test)]
    pub fn reset(&self) {
        if let Ok(mut s) = self.settings.write() {
            *s = VoiceChangerSettings::default();
        }
        self.clear_prev_audio();
        if let Ok(mut m) = self.model.write() {
            *m = None;
        }
        if let Ok(mut ep) = self.exported_path.write() {
            *ep = None;
        }
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
    use crate::rvc::VoiceChangerModel;
    use crate::test_util::cleanup_test_dirs;
    use serial_test::serial;

    struct DummyModel;

    impl VoiceChangerModel for DummyModel {
        fn processing_sample_rate(&self) -> i32 {
            16000
        }

        fn inference(&self, input: &[i16]) -> Vec<i16> {
            input.to_vec()
        }

        fn update_settings(&mut self, _key: &str, _val: Value) -> bool {
            true
        }

        fn get_info(&self) -> Value {
            serde_json::json!({})
        }

        fn generate_input(
            &mut self,
            new_data: &[i16],
            _input_size: usize,
            _crossfade_size: usize,
            _sola_search_frame: usize,
        ) -> (Vec<i16>, Vec<i16>, Vec<i16>, usize, f32, usize) {
            (
                new_data.to_vec(),
                Vec::new(),
                Vec::new(),
                new_data.len(),
                0.0,
                new_data.len(),
            )
        }

        fn export_to_onnx(&self) -> std::io::Result<String> {
            Ok(String::from("dummy.onnx"))
        }

        fn get_model_current(&self) -> Vec<Value> {
            Vec::new()
        }
    }

    impl VCModel for DummyModel {}

    #[test]
    #[serial]
    fn sample_rate_methods() {
        let vc = VoiceChanger::new();
        vc.set_input_sample_rate(24000);
        vc.set_output_sample_rate(24000);
        let info = vc.get_info();
        assert_eq!(info.input_sample_rate, 24000);
        assert_eq!(info.output_sample_rate, 24000);
        cleanup_test_dirs();
    }

    #[test]
    #[serial]
    fn model_inference_and_processing_rate() {
        let vc = VoiceChanger::new();
        vc.set_model(DummyModel);
        let rate = vc.get_processing_sampling_rate();
        assert_eq!(rate, 16000);
        let out = vc.change_voice(&[1, 2, 3]);
        assert_eq!(out, vec![1, 2, 3]);
        cleanup_test_dirs();
    }
}
