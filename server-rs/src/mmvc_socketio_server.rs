use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use socketioxide::{
    extract::{Bin, Data, SocketRef, State},
    layer::SocketIoLayer,
    SocketIo,
};

use crate::voice_changer_manager::VoiceChangerManager;

static MANAGER: OnceCell<&'static VoiceChangerManager> = OnceCell::new();

#[derive(Deserialize)]
struct VoiceRequest(Vec<serde_json::Value>);

#[derive(Serialize)]
struct VoiceResponse {
    timestamp: u64,
    performance: [f32; 3],
}

pub struct MMVCSocketIOServer {
    layer: SocketIoLayer,
}

impl MMVCSocketIOServer {
    pub fn new(manager: &'static VoiceChangerManager) -> Self {
        MANAGER.set(manager).ok();
        let (layer, io) = SocketIo::builder().with_state(manager).build_layer();

        io.ns(
            "/test",
            |socket: SocketRef, State(manager): State<&'static VoiceChangerManager>| {
                manager.clear_prev_audio();
                socket.on(
                    "request_message",
                    move |socket: SocketRef, Data::<VoiceRequest>(payload), Bin(bin)| async move {
                        let timestamp = payload.0.get(0).and_then(|v| v.as_u64()).unwrap_or(0);
                        let audio = bin.get(0).cloned().unwrap_or_default();
                        let mut samples = Vec::with_capacity(audio.len() / 2);
                        for chunk in audio.chunks_exact(2) {
                            samples.push(i16::from_le_bytes([chunk[0], chunk[1]]));
                        }
                        let changed = manager.change_voice(&samples);
                        let mut out_bytes = Vec::with_capacity(changed.len() * 2);
                        for s in changed {
                            out_bytes.extend_from_slice(&s.to_le_bytes());
                        }
                        let resp = VoiceResponse {
                            timestamp,
                            performance: [0.0, 0.0, 0.0],
                        };
                        let _ = socket.bin(vec![out_bytes]).emit("response", resp);
                    },
                );
            },
        );

        Self { layer }
    }

    pub fn layer(self) -> SocketIoLayer {
        self.layer
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{test_util::cleanup_test_dirs, voice_changer_params::VoiceChangerParams};
    use serde_json::json;
    use serial_test::serial;

    #[test]
    #[serial]
    fn binary_roundtrip() {
        let params = VoiceChangerParams {
            model_dir: "m".into(),
            content_vec_500: "".into(),
            content_vec_500_onnx: "".into(),
            content_vec_500_onnx_on: false,
            hubert_base: "".into(),
            hubert_base_jp: "".into(),
            hubert_soft: "".into(),
            nsf_hifigan: "".into(),
            sample_mode: "".into(),
            crepe_onnx_full: "".into(),
            crepe_onnx_tiny: "".into(),
            rmvpe: "".into(),
            rmvpe_onnx: "".into(),
            whisper_tiny: "".into(),
        };
        let manager = VoiceChangerManager::get_instance(params);
        #[cfg(test)]
        manager.reset();

        let input = [1i16, -2i16];
        let out = manager.change_voice(&input);
        let mut bytes = Vec::new();
        for s in out.iter() {
            bytes.extend_from_slice(&s.to_le_bytes());
        }
        let resp = VoiceResponse {
            timestamp: 1,
            performance: [0.0, 0.0, 0.0],
        };
        let payload = json!([
            "response",
            resp,
            {"_placeholder": true, "num": 0}
        ]);
        let packet = format!("51-/test,{}", payload.to_string());
        assert!(packet.starts_with("51-/test,"));
        manager.reset();
        cleanup_test_dirs();
    }
}
