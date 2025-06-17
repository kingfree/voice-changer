use axum::{routing::post, Json, Router};
use base64::{engine::general_purpose, Engine as _};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::voice_changer_manager::VoiceChangerManager;

static MANAGER: OnceCell<&'static VoiceChangerManager> = OnceCell::new();
static LOCK: OnceCell<Arc<Mutex<()>>> = OnceCell::new();

#[derive(Deserialize)]
struct VoiceModel {
    timestamp: u64,
    buffer: String,
}

#[derive(Serialize)]
struct TestResponse {
    timestamp: u64,
    changed_voice_base64: String,
}

async fn test(Json(payload): Json<VoiceModel>) -> Json<TestResponse> {
    let manager = MANAGER.get().expect("manager not set");
    let lock = LOCK.get().expect("lock not set");
    let bytes = general_purpose::STANDARD
        .decode(&payload.buffer)
        .unwrap_or_default();
    let mut samples = Vec::with_capacity(bytes.len() / 2);
    for chunk in bytes.chunks_exact(2) {
        samples.push(i16::from_le_bytes([chunk[0], chunk[1]]));
    }
    let _guard = lock.lock().await;
    let changed = manager.change_voice(&samples);
    drop(_guard);
    let mut out_bytes = Vec::with_capacity(changed.len() * 2);
    for s in changed {
        out_bytes.extend_from_slice(&s.to_le_bytes());
    }
    let encoded = general_purpose::STANDARD.encode(out_bytes);
    Json(TestResponse {
        timestamp: payload.timestamp,
        changed_voice_base64: encoded,
    })
}

pub struct MMVCRestVoiceChanger {
    router: Router,
}

impl MMVCRestVoiceChanger {
    pub fn new(manager: &'static VoiceChangerManager) -> Self {
        MANAGER.set(manager).ok();
        LOCK.set(Arc::new(Mutex::new(()))).ok();
        let router = Router::new().route("/test", post(test));
        Self { router }
    }

    pub fn router(self) -> Router {
        self.router
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        Router,
    };
    use serde_json::{json, Value};
    use serial_test::serial;
    use tower::ServiceExt;

    use crate::{test_util::cleanup_test_dirs, voice_changer_params::VoiceChangerParams};

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
        (MMVCRestVoiceChanger::new(manager).router(), manager)
    }

    #[tokio::test]
    #[serial]
    async fn test_endpoint_echoes_payload() {
        let (app, manager) = app();
        let samples = vec![1i16, -2i16];
        let bytes: Vec<u8> = samples.iter().flat_map(|x| x.to_le_bytes()).collect();
        let encoded = general_purpose::STANDARD.encode(&bytes);
        let payload = json!({"timestamp": 123u64, "buffer": encoded});
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/test")
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let v: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["timestamp"], 123);
        assert_eq!(v["changed_voice_base64"], encoded);
        manager.reset();
        cleanup_test_dirs();
    }
}
