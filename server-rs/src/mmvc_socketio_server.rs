use base64::{engine::general_purpose, Engine as _};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use socketioxide::{
    extract::{Data, SocketRef, State},
    layer::SocketIoLayer,
    SocketIo,
};

use crate::voice_changer_manager::VoiceChangerManager;

static MANAGER: OnceCell<&'static VoiceChangerManager> = OnceCell::new();

#[derive(Deserialize)]
struct VoiceModel {
    timestamp: u64,
    buffer: String,
}

#[derive(Serialize)]
struct VoiceResponse {
    timestamp: u64,
    changed_voice_base64: String,
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
                    move |socket: SocketRef, Data::<VoiceModel>(payload)| async move {
                        let bytes = general_purpose::STANDARD
                            .decode(&payload.buffer)
                            .unwrap_or_default();
                        let mut samples = Vec::with_capacity(bytes.len() / 2);
                        for chunk in bytes.chunks_exact(2) {
                            samples.push(i16::from_le_bytes([chunk[0], chunk[1]]));
                        }
                        let changed = manager.change_voice(&samples);
                        let mut out_bytes = Vec::with_capacity(changed.len() * 2);
                        for s in changed {
                            out_bytes.extend_from_slice(&s.to_le_bytes());
                        }
                        let encoded = general_purpose::STANDARD.encode(out_bytes);
                        let resp = VoiceResponse {
                            timestamp: payload.timestamp,
                            changed_voice_base64: encoded,
                        };
                        let _ = socket.emit("response", resp);
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
