use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    Router,
};
use base64::{engine::general_purpose, Engine as _};
use futures_util::{SinkExt, StreamExt};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

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

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(socket: WebSocket) {
    let manager = MANAGER.get().expect("manager not set");
    manager.clear_prev_audio();
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
    let tx_clone = tx.clone();

    manager.set_emit_to(move |perf| {
        let msg = Message::Text(serde_json::json!({ "perf": perf }).to_string());
        let _ = tx_clone.send(msg);
    });

    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            Message::Text(text) => {
                if let Ok(payload) = serde_json::from_str::<VoiceModel>(&text) {
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
                    let msg = Message::Text(serde_json::to_string(&resp).unwrap());
                    if tx.send(msg).is_err() {
                        break;
                    }
                } else if tx.send(Message::Text(text)).is_err() {
                    break;
                }
            }
            Message::Binary(bin) => {
                if tx.send(Message::Binary(bin)).is_err() {
                    break;
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    send_task.abort();
}

pub struct MMVCSocketIOServer {
    router: Router,
}

impl MMVCSocketIOServer {
    pub fn new(manager: &'static VoiceChangerManager) -> Self {
        MANAGER.set(manager).ok();
        let router = Router::new().route("/ws", get(ws_handler));
        Self { router }
    }

    pub fn router(self) -> Router {
        self.router
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::Router;
    use futures_util::StreamExt;
    use serde_json::{json, Value};
    use serial_test::serial;
    use tokio_tungstenite::connect_async;
    use tokio_tungstenite::tungstenite::Message as WsMessage;

    use crate::voice_changer_params::VoiceChangerParams;

    async fn start_server(app: Router) -> std::net::SocketAddr {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::Server::from_tcp(listener.into_std().unwrap())
                .unwrap()
                .serve(app.into_make_service())
                .await
                .unwrap();
        });
        // small delay to ensure server starts
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        addr
    }

    use crate::test_util::cleanup_test_dirs;
    fn app() -> (Router, &'static VoiceChangerManager) {
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
        (MMVCSocketIOServer::new(manager).router(), manager)
    }

    #[tokio::test]
    #[serial]
    async fn websocket_returns_changed_voice() {
        let (app, manager) = app();
        let addr = start_server(app).await;
        let url = format!("ws://{}/ws", addr);
        let (mut ws_stream, _) = connect_async(url).await.unwrap();

        let samples = vec![1i16, -2i16];
        let bytes: Vec<u8> = samples.iter().flat_map(|x| x.to_le_bytes()).collect();
        let encoded = general_purpose::STANDARD.encode(&bytes);
        let payload = json!({"timestamp":123u64,"buffer":encoded}).to_string();
        ws_stream.send(WsMessage::Text(payload)).await.unwrap();

        if let Some(Ok(WsMessage::Text(resp))) = ws_stream.next().await {
            let v: Value = serde_json::from_str(&resp).unwrap();
            assert_eq!(v["timestamp"], 123);
            assert_eq!(v["changed_voice_base64"], encoded);
        } else {
            panic!("no response");
        }

        manager.reset();
        cleanup_test_dirs();
    }
}
