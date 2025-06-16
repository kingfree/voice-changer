pub use crate::rvc::VCModel;
use crate::model_slot::ModelSlot;
use crate::voice_changer_params::VoiceChangerParams;

pub trait VCModelPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn create_model(&self, params: &VoiceChangerParams, slot: &ModelSlot) -> Box<dyn VCModel>;
}
