use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelSlot {
    RVC(RVCModelSlot),
    Empty,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RVCModelSlot {
    pub model_file: String,
    pub sampling_rate: i32,
}

impl Default for RVCModelSlot {
    fn default() -> Self {
        Self {
            model_file: String::new(),
            sampling_rate: 48000,
        }
    }
}
