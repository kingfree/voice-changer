pub use crate::rvc::VCModel;
use crate::voice_changer_params::VoiceChangerParams;

pub trait VCModelPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn create_model(&self, params: &VoiceChangerParams, path: &str) -> Box<dyn VCModel>;
}
