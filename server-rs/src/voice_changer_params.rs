use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct VoiceChangerParams {
    pub model_dir: String,
    pub content_vec_500: String,
    pub content_vec_500_onnx: String,
    pub content_vec_500_onnx_on: bool,
    pub hubert_base: String,
    pub hubert_base_jp: String,
    pub hubert_soft: String,
    pub nsf_hifigan: String,
    pub sample_mode: String,
    pub crepe_onnx_full: String,
    pub crepe_onnx_tiny: String,
    pub rmvpe: String,
    pub rmvpe_onnx: String,
    pub whisper_tiny: String,
}
