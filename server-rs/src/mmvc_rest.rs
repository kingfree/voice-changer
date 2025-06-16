use axum::{routing::{get, post}, Router, Json};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct Hello {
    result: &'static str,
}

async fn hello() -> Json<Hello> {
    Json(Hello { result: "Index" })
}

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
    // Placeholder implementation: echo back the payload
    Json(TestResponse {
        timestamp: payload.timestamp,
        changed_voice_base64: payload.buffer,
    })
}

pub struct MMVCRest {
    router: Router,
}

impl MMVCRest {
    pub fn new() -> Self {
        let router = Router::new()
            .route("/api/hello", get(hello))
            .route("/test", post(test));
        Self { router }
    }

    pub fn router(self) -> Router {
        self.router
    }
}
