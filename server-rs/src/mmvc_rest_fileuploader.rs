//! File upload and model management REST API.
//!
//! This module contains handlers roughly corresponding to the Python
//! implementation.  They are intentionally simple and documented so that
//! generated documentation can describe the available endpoints.

use axum::{
    extract::{Form, Multipart},
    routing::{get, post},
    Json, Router,
};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tokio::{fs, io::AsyncWriteExt};

use crate::voice_changer_manager::VoiceChangerManager;

/// Directory used for temporarily storing uploaded chunks.
const UPLOAD_DIR: &str = "upload_dir";
/// Directory containing saved models.
const MODEL_DIR: &str = "logs";

static MANAGER: OnceCell<&'static VoiceChangerManager> = OnceCell::new();

/// Remove any directory components from user provided file names.
fn sanitize_filename(filename: &str) -> String {
    Path::new(filename)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string()
}

/// `POST /upload_file`
///
/// Accept a multipart request containing a file and store it in
/// [`UPLOAD_DIR`].  The form fields `filename` and `file` are expected.
async fn post_upload_file(mut multipart: Multipart) -> Json<Value> {
    let mut filename: Option<String> = None;
    let mut file_bytes: Option<Vec<u8>> = None;
    while let Ok(Some(field)) = multipart.next_field().await {
        match field.name() {
            Some("filename") => {
                let text = field.text().await.unwrap_or_default();
                filename = Some(text);
            }
            Some("file") => {
                let data = field.bytes().await.unwrap_or_default().to_vec();
                file_bytes = Some(data);
            }
            _ => {}
        }
    }

    if let (Some(name), Some(data)) = (filename, file_bytes) {
        let safe_name = sanitize_filename(&name);
        let path = PathBuf::from(UPLOAD_DIR).join(&safe_name);
        if let Some(parent) = path.parent() {
            if fs::create_dir_all(parent).await.is_err() {
                return Json(json!({"status":"ERROR","msg":"create_dir"}));
            }
        }
        match fs::File::create(&path).await {
            Ok(mut f) => {
                if f.write_all(&data).await.is_err() {
                    return Json(json!({"status":"ERROR","msg":"write"}));
                }
            }
            Err(_) => return Json(json!({"status":"ERROR","msg":"open"})),
        }
        return Json(json!({"status":"OK","msg":format!("uploaded files {}", safe_name)}));
    }
    Json(json!({"status":"ERROR","msg":"uploaded file is not found."}))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConcatParams {
    /// Name of the final file to create.
    filename: String,
    /// Number of chunks that were uploaded.
    filename_chunk_num: usize,
}

/// `POST /concat_uploaded_file`
///
/// Combine previously uploaded chunks into a single file.
async fn post_concat_uploaded_file(Form(params): Form<ConcatParams>) -> Json<Value> {
    let safe_name = sanitize_filename(&params.filename);
    let target = PathBuf::from(UPLOAD_DIR).join(&safe_name);
    if let Some(parent) = target.parent() {
        if fs::create_dir_all(parent).await.is_err() {
            return Json(json!({"status":"ERROR","msg":"create_dir"}));
        }
    }
    if fs::remove_file(&target).await.is_ok() {}
    let mut out = match fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&target)
        .await
    {
        Ok(f) => f,
        Err(_) => return Json(json!({"status":"ERROR","msg":"open"})),
    };
    for i in 0..params.filename_chunk_num {
        let chunk = PathBuf::from(UPLOAD_DIR).join(format!("{}_{}", safe_name, i));
        if let Ok(mut cfile) = fs::File::open(&chunk).await {
            if let Ok(data) = fs::read(&chunk).await {
                if out.write_all(&data).await.is_err() {
                    return Json(json!({"status":"ERROR","msg":"write"}));
                }
            }
            let _ = fs::remove_file(&chunk).await;
        }
    }
    Json(json!({"status":"OK","msg":format!("concat files {}", safe_name)}))
}

/// `GET /info`
///
/// Return current server information, mirroring the Python API.
async fn get_info() -> Json<Value> {
    let manager = MANAGER.get().expect("manager not set");
    Json(manager.get_info())
}

/// `GET /performance`
///
/// Return performance counters for the current model.
async fn get_performance() -> Json<Value> {
    let manager = MANAGER.get().expect("manager not set");
    Json(manager.get_performance())
}

#[derive(Deserialize)]
struct UpdateParams {
    /// Setting key to update.
    key: String,
    /// New value as JSON or plain string.
    val: String,
}

/// `POST /update_settings`
///
/// Update global settings by key.
async fn post_update_settings(Form(params): Form<UpdateParams>) -> Json<Value> {
    let manager = MANAGER.get().expect("manager not set");
    let val: Value = serde_json::from_str(&params.val).unwrap_or(Value::String(params.val));
    Json(manager.update_settings(&params.key, val))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoadModelPayload {
    /// Target slot index.
    slot: i32,
    /// Whether to load the model using half precision.
    is_half: bool,
    /// Additional JSON parameters.
    params: String,
}

/// `POST /load_model`
///
/// Load a voice model based on uploaded assets.
async fn post_load_model(Form(payload): Form<LoadModelPayload>) -> Json<Value> {
    let manager = MANAGER.get().expect("manager not set");
    if let Ok(req) =
        serde_json::from_str::<crate::voice_changer_manager::LoadModelRequest>(&payload.params)
    {
        Json(manager.load_model(req))
    } else {
        Json(json!({"status": "ERROR", "msg": "invalid params"}))
    }
}

#[derive(Deserialize)]
struct MergeParams {
    /// JSON request string describing the merge.
    request: String,
}

/// `POST /merge_model`
///
/// Merge two models according to the provided request.
async fn post_merge_models(Form(params): Form<MergeParams>) -> Json<Value> {
    let manager = MANAGER.get().expect("manager not set");
    Json(manager.merge_models(&params.request))
}

/// `GET /onnx`
///
/// Export the current model to ONNX format if possible.
async fn get_onnx() -> Json<Value> {
    let manager = MANAGER.get().expect("manager not set");
    Json(json!({"exported": manager.export_to_onnx()}))
}

/// `POST /update_model_default`
///
/// Update the default model configuration.
async fn post_update_model_default() -> Json<Value> {
    let manager = MANAGER.get().expect("manager not set");
    Json(manager.update_model_default())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateModelInfo {
    /// JSON metadata to write.
    new_data: String,
}

/// `POST /update_model_info`
///
/// Write metadata to the current model slot.
async fn post_update_model_info(Form(params): Form<UpdateModelInfo>) -> Json<Value> {
    let manager = MANAGER.get().expect("manager not set");
    Json(manager.update_model_info(&params.new_data))
}

#[derive(Deserialize)]
struct UploadModelAssets {
    /// JSON request describing the asset move.
    params: String,
}

/// `POST /upload_model_assets`
///
/// Move uploaded model assets into the selected slot.
async fn post_upload_model_assets(Form(params): Form<UploadModelAssets>) -> Json<Value> {
    let manager = MANAGER.get().expect("manager not set");
    Json(manager.upload_model_assets(&params.params))
}

/// Sub-router providing file upload and model management endpoints.
pub struct MMVCRestFileuploader {
    router: Router,
}

impl MMVCRestFileuploader {
    /// Create a new instance configured with a [`VoiceChangerManager`].
    pub fn new(manager: &'static VoiceChangerManager) -> Self {
        MANAGER.set(manager).ok();
        let router = Router::new()
            .route("/info", get(get_info))
            .route("/performance", get(get_performance))
            .route("/upload_file", post(post_upload_file))
            .route("/concat_uploaded_file", post(post_concat_uploaded_file))
            .route("/update_settings", post(post_update_settings))
            .route("/load_model", post(post_load_model))
            .route("/onnx", get(get_onnx))
            .route("/merge_model", post(post_merge_models))
            .route("/update_model_default", post(post_update_model_default))
            .route("/update_model_info", post(post_update_model_info))
            .route("/upload_model_assets", post(post_upload_model_assets));
        Self { router }
    }

    /// Consume the object and return the configured [`Router`].
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
    use serde_json::Value;
    use tower::ServiceExt;

    use crate::mmvc_rest::MMVCRest;
    use crate::test_util::cleanup_test_dirs;
    use crate::voice_changer_params::VoiceChangerParams; // for constructing manager
    use serial_test::serial;

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
        (MMVCRestFileuploader::new(manager).router(), manager)
    }

    #[tokio::test]
    #[serial]
    async fn info_endpoint_returns_status() {
        let (app, manager) = app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/info")
                    .method("GET")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let v: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["status"], "OK");
        manager.reset();
        cleanup_test_dirs();
    }

    #[tokio::test]
    #[serial]
    async fn update_settings_endpoint_changes_value() {
        let (app, manager) = app();
        let payload = "key=modelSlotIndex&val=1";
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/update_settings")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .method("POST")
                    .body(Body::from(payload))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let v: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["settings"]["model_slot_index"], 1);
        manager.reset();
        cleanup_test_dirs();
    }

    #[tokio::test]
    #[serial]
    async fn performance_endpoint_returns_values() {
        let (app, manager) = app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/performance")
                    .method("GET")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let v: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["status"], "OK");
        assert!(v["performance"].as_array().unwrap().len() >= 3);
        manager.reset();
        cleanup_test_dirs();
    }

    #[tokio::test]
    #[serial]
    async fn onnx_endpoint_exports_model() {
        let (app, manager) = app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/onnx")
                    .method("GET")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let v: Value = serde_json::from_slice(&body).unwrap();
        assert!(v["exported"].as_bool().unwrap());
        manager.reset();
        cleanup_test_dirs();
    }

    #[tokio::test]
    #[serial]
    async fn upload_and_concat_files() {
        use hyper::header::{HeaderValue, CONTENT_TYPE};
        let (app, manager) = app();

        let boundary = "BOUNDARY";
        // upload first chunk
        let body = format!(
            "--{b}\r\nContent-Disposition: form-data; name=\"filename\"\r\n\r\nfinal.txt_0\r\n--{b}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"final.txt_0\"\r\nContent-Type: application/octet-stream\r\n\r\nhello\r\n--{b}--\r\n",
            b = boundary
        );
        let req = Request::builder()
            .uri("/upload_file")
            .method("POST")
            .header(
                CONTENT_TYPE,
                HeaderValue::from_str(&format!("multipart/form-data; boundary={}", boundary))
                    .unwrap(),
            )
            .body(Body::from(body))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // upload second chunk
        let body = format!(
            "--{b}\r\nContent-Disposition: form-data; name=\"filename\"\r\n\r\nfinal.txt_1\r\n--{b}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"final.txt_1\"\r\nContent-Type: application/octet-stream\r\n\r\nworld\r\n--{b}--\r\n",
            b = boundary
        );
        let req = Request::builder()
            .uri("/upload_file")
            .method("POST")
            .header(
                CONTENT_TYPE,
                HeaderValue::from_str(&format!("multipart/form-data; boundary={}", boundary))
                    .unwrap(),
            )
            .body(Body::from(body))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // concat chunks
        let payload = "filename=final.txt&filenameChunkNum=2";
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/concat_uploaded_file")
                    .method("POST")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(payload))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let target = std::path::Path::new(UPLOAD_DIR).join("final.txt");
        let content = tokio::fs::read_to_string(&target).await.unwrap();
        assert_eq!(content, "helloworld");
        manager.reset();
        cleanup_test_dirs();
    }

    #[tokio::test]
    #[serial]
    async fn merge_model_endpoint_creates_file() {
        use serde_json::json;
        let (app, manager) = app();
        let f1 = std::path::Path::new(crate::constants::TMP_DIR).join("ma.txt");
        let f2 = std::path::Path::new(crate::constants::TMP_DIR).join("mb.txt");
        tokio::fs::create_dir_all(crate::constants::TMP_DIR)
            .await
            .unwrap();
        tokio::fs::write(&f1, b"a").await.unwrap();
        tokio::fs::write(&f2, b"b").await.unwrap();
        let req = json!({
            "output": "res.txt",
            "files": [f1.to_str().unwrap(), f2.to_str().unwrap()]
        });
        let payload = format!("request={}", req.to_string());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/merge_model")
                    .method("POST")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(payload))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let out = std::path::Path::new(crate::constants::TMP_DIR).join("res.txt");
        assert!(out.exists());
        let content = tokio::fs::read_to_string(&out).await.unwrap();
        assert_eq!(content, "ab");
        manager.reset();
        cleanup_test_dirs();
    }

    #[tokio::test]
    async fn performance_endpoint_returns_values() {
        let app = app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/performance")
                    .method("GET")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let v: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["status"], "OK");
        assert!(v["performance"].as_array().unwrap().len() >= 3);
    }

    #[tokio::test]
    async fn onnx_endpoint_exports_model() {
        let app = app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/onnx")
                    .method("GET")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let v: Value = serde_json::from_slice(&body).unwrap();
        assert!(v["exported"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn upload_and_concat_files() {
        use hyper::header::{HeaderValue, CONTENT_TYPE};
        let app = app();

        let boundary = "BOUNDARY";
        // upload first chunk
        let body = format!(
            "--{b}\r\nContent-Disposition: form-data; name=\"filename\"\r\n\r\nfinal.txt_0\r\n--{b}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"final.txt_0\"\r\nContent-Type: application/octet-stream\r\n\r\nhello\r\n--{b}--\r\n",
            b = boundary
        );
        let req = Request::builder()
            .uri("/upload_file")
            .method("POST")
            .header(
                CONTENT_TYPE,
                HeaderValue::from_str(&format!("multipart/form-data; boundary={}", boundary))
                    .unwrap(),
            )
            .body(Body::from(body))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // upload second chunk
        let body = format!(
            "--{b}\r\nContent-Disposition: form-data; name=\"filename\"\r\n\r\nfinal.txt_1\r\n--{b}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"final.txt_1\"\r\nContent-Type: application/octet-stream\r\n\r\nworld\r\n--{b}--\r\n",
            b = boundary
        );
        let req = Request::builder()
            .uri("/upload_file")
            .method("POST")
            .header(
                CONTENT_TYPE,
                HeaderValue::from_str(&format!("multipart/form-data; boundary={}", boundary))
                    .unwrap(),
            )
            .body(Body::from(body))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // concat chunks
        let payload = "filename=final.txt&filenameChunkNum=2";
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/concat_uploaded_file")
                    .method("POST")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(payload))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let target = std::path::Path::new(UPLOAD_DIR).join("final.txt");
        let content = tokio::fs::read_to_string(&target).await.unwrap();
        assert_eq!(content, "helloworld");
    }

    #[tokio::test]
    async fn merge_model_endpoint_creates_file() {
        use serde_json::json;
        let app = app();
        let f1 = std::path::Path::new(crate::constants::TMP_DIR).join("ma.txt");
        let f2 = std::path::Path::new(crate::constants::TMP_DIR).join("mb.txt");
        tokio::fs::create_dir_all(crate::constants::TMP_DIR)
            .await
            .unwrap();
        tokio::fs::write(&f1, b"a").await.unwrap();
        tokio::fs::write(&f2, b"b").await.unwrap();
        let req = json!({
            "output": "res.txt",
            "files": [f1.to_str().unwrap(), f2.to_str().unwrap()]
        });
        let payload = format!("request={}", req.to_string());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/merge_model")
                    .method("POST")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(payload))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let out = std::path::Path::new(crate::constants::TMP_DIR).join("res.txt");
        assert!(out.exists());
        let content = tokio::fs::read_to_string(&out).await.unwrap();
        assert_eq!(content, "ab");
    }
}
