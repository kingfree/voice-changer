/// Trait mirroring the Python `VoiceChangerModel` protocol.
pub trait VoiceChangerModel: Send + Sync {
    /// Return the sample rate expected by the model.
    fn processing_sample_rate(&self) -> i32;
    /// Run inference on an audio buffer.
    fn inference(&self, input: &[i16]) -> Vec<i16>;
    /// Update internal settings using a key/value pair.
    fn update_settings(&mut self, key: &str, val: Value) -> bool;
    /// Return a JSON object describing current settings.
    fn get_info(&self) -> Value;
    /// Prepare model input. The default implementation simply returns the
    /// received audio and a calculated volume.
    fn generate_input(
        &mut self,
        new_data: &[i16],
        input_size: usize,
        crossfade_size: usize,
        sola_search_frame: usize,
    ) -> (Vec<i16>, Vec<i16>, Vec<i16>, usize, f32, usize);

    /// Export the model to ONNX and return the file path.
    fn export_to_onnx(&self) -> std::io::Result<String>;

    /// Retrieve the current model settings as key/value pairs.
    fn get_model_current(&self) -> Vec<Value>;
}

/// Marker trait used throughout the Rust server implementation.  It extends
/// [`VoiceChangerModel`] so concrete models only need to implement that one.
pub trait VCModel: VoiceChangerModel {}

use crate::constants::TMP_DIR;
use crate::model_slot::ModelSlot;
use crate::plugin::VCModelPlugin;
use crate::voice_changer_params::VoiceChangerParams;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;

/// Runtime settings mirrored from the Python `RVCSettings` dataclass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RvcSettings {
    pub gpu: i32,
    #[serde(rename = "dstId")]
    pub dst_id: i32,
    #[serde(rename = "f0Detector")]
    pub f0_detector: String,
    pub tran: i32,
    #[serde(rename = "silentThreshold")]
    pub silent_threshold: f32,
    #[serde(rename = "extraConvertSize")]
    pub extra_convert_size: i32,
    #[serde(rename = "indexRatio")]
    pub index_ratio: f32,
    pub protect: f32,
    #[serde(rename = "rvcQuality")]
    pub rvc_quality: i32,
    #[serde(rename = "silenceFront")]
    pub silence_front: i32,
    #[serde(rename = "modelSamplingRate")]
    pub model_sampling_rate: i32,
}

impl Default for RvcSettings {
    fn default() -> Self {
        Self {
            gpu: -9999,
            dst_id: 0,
            f0_detector: "rmvpe_onnx".into(),
            tran: 12,
            silent_threshold: 0.00001,
            extra_convert_size: 4096,
            index_ratio: 0.0,
            protect: 0.5,
            rvc_quality: 0,
            silence_front: 1,
            model_sampling_rate: 48000,
        }
    }
}

pub struct Rvc {
    sample_rate: i32,
    #[allow(dead_code)]
    path: String,
    settings: RvcSettings,
    audio_buffer: Vec<i16>,
}

impl Rvc {
    pub fn new(sample_rate: i32, path: String) -> Self {
        Self {
            sample_rate,
            path,
            settings: RvcSettings::default(),
            audio_buffer: Vec::new(),
        }
    }

    /// Update runtime settings from a JSON key/value pair.
    pub fn update_settings(&mut self, key: &str, val: Value) -> bool {
        match key {
            "gpu" => {
                if let Some(v) = val.as_i64() {
                    self.settings.gpu = v as i32;
                }
            }
            "dstId" => {
                if let Some(v) = val.as_i64() {
                    self.settings.dst_id = v as i32;
                }
            }
            "f0Detector" => {
                if let Some(v) = val.as_str() {
                    self.settings.f0_detector = v.to_string();
                }
            }
            "tran" => {
                if let Some(v) = val.as_i64() {
                    self.settings.tran = v as i32;
                }
            }
            "silentThreshold" => {
                if let Some(v) = val.as_f64() {
                    self.settings.silent_threshold = v as f32;
                }
            }
            "extraConvertSize" => {
                if let Some(v) = val.as_i64() {
                    self.settings.extra_convert_size = v as i32;
                }
            }
            "indexRatio" => {
                if let Some(v) = val.as_f64() {
                    self.settings.index_ratio = v as f32;
                }
            }
            "protect" => {
                if let Some(v) = val.as_f64() {
                    self.settings.protect = v as f32;
                }
            }
            "rvcQuality" => {
                if let Some(v) = val.as_i64() {
                    self.settings.rvc_quality = v as i32;
                }
            }
            "silenceFront" => {
                if let Some(v) = val.as_i64() {
                    self.settings.silence_front = v as i32;
                }
            }
            "modelSamplingRate" => {
                if let Some(v) = val.as_i64() {
                    self.settings.model_sampling_rate = v as i32;
                }
            }
            _ => return false,
        }
        true
    }

    /// Return settings information in JSON format.
    pub fn get_info(&self) -> Value {
        json!(self.settings)
    }

    /// Export the model to ONNX by writing a dummy file into [`TMP_DIR`].
    pub fn export_to_onnx(&self) -> std::io::Result<String> {
        let dir = Path::new(TMP_DIR);
        std::fs::create_dir_all(dir)?;
        let path = dir.join("rvc_model.onnx");
        std::fs::write(&path, b"dummy onnx")?;
        Ok(path.to_string_lossy().to_string())
    }

    /// Return key/value pairs representing current model defaults.
    pub fn get_model_current(&self) -> Vec<Value> {
        vec![
            json!({"key": "defaultTune", "val": self.settings.tran}),
            json!({"key": "defaultIndexRatio", "val": self.settings.index_ratio}),
            json!({"key": "defaultProtect", "val": self.settings.protect}),
        ]
    }
}

impl VoiceChangerModel for Rvc {
    fn processing_sample_rate(&self) -> i32 {
        self.sample_rate
    }

    fn inference(&self, input: &[i16]) -> Vec<i16> {
        input.to_vec()
    }

    fn update_settings(&mut self, key: &str, val: Value) -> bool {
        Rvc::update_settings(self, key, val)
    }

    fn get_info(&self) -> Value {
        Rvc::get_info(self)
    }

    fn generate_input(
        &mut self,
        new_data: &[i16],
        input_size: usize,
        crossfade_size: usize,
        sola_search_frame: usize,
    ) -> (Vec<i16>, Vec<i16>, Vec<i16>, usize, f32, usize) {
        // Simplified implementation mirroring the Python interface.
        let convert_size = input_size
            + crossfade_size
            + sola_search_frame
            + self.settings.extra_convert_size as usize;
        self.audio_buffer.extend_from_slice(new_data);
        if self.audio_buffer.len() > convert_size {
            let excess = self.audio_buffer.len() - convert_size;
            self.audio_buffer.drain(0..excess);
        }
        let vol_segment = self
            .audio_buffer
            .iter()
            .rev()
            .take(input_size + crossfade_size)
            .cloned()
            .collect::<Vec<_>>();
        let vol = if vol_segment.is_empty() {
            0.0
        } else {
            let mean = vol_segment
                .iter()
                .map(|v| *v as f32 * *v as f32)
                .sum::<f32>()
                / vol_segment.len() as f32;
            mean.sqrt()
        };
        (
            self.audio_buffer.clone(),
            Vec::new(),
            Vec::new(),
            convert_size,
            vol,
            input_size + crossfade_size,
        )
    }

    fn export_to_onnx(&self) -> std::io::Result<String> {
        Rvc::export_to_onnx(self)
    }

    fn get_model_current(&self) -> Vec<Value> {
        Rvc::get_model_current(self)
    }
}

impl VCModel for Rvc {}

pub struct RvcPlugin;

impl VCModelPlugin for RvcPlugin {
    fn name(&self) -> &str {
        "RVC"
    }

    fn create_model(&self, _params: &VoiceChangerParams, slot: &ModelSlot) -> Box<dyn VCModel> {
        match slot {
            ModelSlot::RVC(info) => Box::new(Rvc::new(info.sampling_rate, info.model_file.clone())),
            _ => Box::new(Rvc::new(48000, String::new())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::cleanup_test_dirs;
    use serial_test::serial;

    #[test]
    #[serial]
    fn update_and_get_info() {
        let mut r = Rvc::new(48000, "m.pth".into());
        assert_eq!(r.processing_sample_rate(), 48000);
        r.update_settings("tran", Value::from(7));
        let info = r.get_info();
        assert_eq!(info["tran"], 7);
        cleanup_test_dirs();
    }

    #[test]
    #[serial]
    fn export_to_onnx_writes_file() {
        let r = Rvc::new(48000, String::new());
        let path = r.export_to_onnx().unwrap();
        assert!(std::path::Path::new(&path).exists());
        cleanup_test_dirs();
    }

    #[test]
    #[serial]
    fn get_model_current_contains_keys() {
        let r = Rvc::new(48000, String::new());
        let cur = r.get_model_current();
        assert_eq!(cur.len(), 3);
        assert_eq!(cur[0]["key"], "defaultTune");
        cleanup_test_dirs();
    }
}
