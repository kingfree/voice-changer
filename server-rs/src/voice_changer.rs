use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::RwLock;

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
}

impl VoiceChanger {
    pub fn new() -> Self {
        Self {
            settings: RwLock::new(VoiceChangerSettings::default()),
            prev_audio: RwLock::new(Vec::new()),
        }
    }

    pub fn change_voice(&self, input: &[i16]) -> Vec<i16> {
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

    pub fn update_settings(&self, key: &str, val: Value) -> bool {
        let mut s = self.settings.write().unwrap();
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
        self.settings.read().unwrap().clone()
    }

    #[cfg(test)]
    pub fn reset(&self) {
        *self.settings.write().unwrap() = VoiceChangerSettings::default();
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
