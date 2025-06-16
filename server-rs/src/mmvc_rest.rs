//! REST API exposing a minimal set of voice changer endpoints.
//!
//! This module implements a subset of the Python server REST API.  The
//! endpoints are primarily used for unit testing and as a simple example for
//! clients.  Each handler is documented so `cargo doc` can generate API
//! documentation for the HTTP routes.

use axum::{
    routing::{get, post},
    Json, Router,
};
#[path = "mmvc_rest_fileuploader.rs"]
mod mmvc_rest_fileuploader;
use crate::voice_changer_manager::VoiceChangerManager;
use base64::{engine::general_purpose, Engine as _};
use mmvc_rest_fileuploader::MMVCRestFileuploader;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

static MANAGER: OnceCell<&'static VoiceChangerManager> = OnceCell::new();
static LOCK: OnceCell<Arc<Mutex<()>>> = OnceCell::new();

/// Response for [`hello`] endpoint.
#[derive(Serialize)]
struct Hello {
    /// Static greeting string returned to the client.
    result: &'static str,
}

/// `GET /api/hello`
///
/// Return a simple greeting verifying that the server is running.
async fn hello() -> Json<Hello> {
    Json(Hello { result: "Index" })
}

/// Request payload for [`test()`] endpoint.
#[derive(Deserialize)]
struct VoiceModel {
    /// Arbitrary timestamp used by the caller.
    timestamp: u64,
    /// Little endian 16‑bit PCM samples encoded as base64.
    buffer: String,
}

/// Response payload for [`test()`].
#[derive(Serialize)]
struct TestResponse {
    /// Echoed timestamp from the request.
    timestamp: u64,
    /// Converted voice samples encoded as base64.
    changed_voice_base64: String,
}

/// `POST /test`
///
/// Accepts base64 encoded audio samples and returns the modified voice.
async fn test(Json(payload): Json<VoiceModel>) -> Json<TestResponse> {
    let manager = MANAGER.get().expect("manager not set");
    let lock = LOCK.get().expect("lock not set");
    let bytes = match general_purpose::STANDARD.decode(&payload.buffer) {
        Ok(b) => b,
        Err(_) => Vec::new(),
    };
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

/// REST API application.
///
/// Use [`MMVCRest::new`] to create a router that exposes the HTTP endpoints
/// defined in this module.
pub struct MMVCRest {
    router: Router,
}

impl MMVCRest {
    /// Construct a new [`MMVCRest`] using the provided manager instance.
    pub fn new(manager: &'static VoiceChangerManager) -> Self {
        MANAGER.set(manager).ok();
        LOCK.set(Arc::new(Mutex::new(()))).ok();
        let file_router = MMVCRestFileuploader::new(manager).router();
        let router = Router::new()
            .route("/api/hello", get(hello))
            .route("/test", post(test))
            .merge(file_router);
        Self { router }
    }

    /// Consume the object and return the underlying [`Router`].
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
    use tower::ServiceExt; // for `oneshot`

    use crate::voice_changer_params::VoiceChangerParams;
    fn app() -> Router {
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
        MMVCRest::new(manager).router()
    }

    #[tokio::test]
    async fn hello_endpoint_returns_index() {
        let app = app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/hello")
                    .method("GET")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let v: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["result"], "Index");
    }

    #[tokio::test]
    async fn test_endpoint_echoes_payload() {
        let app = app();
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
    }
}
