pub trait VCModel: Send + Sync {
    fn processing_sample_rate(&self) -> i32;
    fn inference(&self, input: &[i16]) -> Vec<i16>;
}

use crate::model_slot::{ModelSlot, RVCModelSlot};
use crate::plugin::VCModelPlugin;
use crate::voice_changer_params::VoiceChangerParams;

pub struct Rvc {
    sample_rate: i32,
    #[allow(dead_code)]
    path: String,
}

impl Rvc {
    pub fn new(sample_rate: i32, path: String) -> Self {
        Self { sample_rate, path }
    }
}

impl VCModel for Rvc {
    fn processing_sample_rate(&self) -> i32 {
        self.sample_rate
    }

    fn inference(&self, input: &[i16]) -> Vec<i16> {
        input.to_vec()
    }
}

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
