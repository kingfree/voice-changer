use axum::{routing::{get, post}, Router, Json, extract::{Multipart, Form}};
use serde::{Serialize, Deserialize};
use serde_json::{json, Value};
use tokio::{fs, io::AsyncWriteExt};
use once_cell::sync::OnceCell;
use std::path::{Path, PathBuf};

use crate::voice_changer_manager::VoiceChangerManager;

const UPLOAD_DIR: &str = "upload_dir";
const MODEL_DIR: &str = "logs";

static MANAGER: OnceCell<&'static VoiceChangerManager> = OnceCell::new();

fn sanitize_filename(filename: &str) -> String {
    Path::new(filename)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string()
}

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
struct ConcatParams {
    filename: String,
    filenameChunkNum: usize,
}

async fn post_concat_uploaded_file(Form(params): Form<ConcatParams>) -> Json<Value> {
    let safe_name = sanitize_filename(&params.filename);
    let target = PathBuf::from(UPLOAD_DIR).join(&safe_name);
    if let Some(parent) = target.parent() {
        if fs::create_dir_all(parent).await.is_err() {
            return Json(json!({"status":"ERROR","msg":"create_dir"}));
        }
    }
    if fs::remove_file(&target).await.is_ok() {}
    let mut out = match fs::OpenOptions::new().create(true).append(true).open(&target).await {
        Ok(f) => f,
        Err(_) => return Json(json!({"status":"ERROR","msg":"open"})),
    };
    for i in 0..params.filenameChunkNum {
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

async fn get_info() -> Json<Value> {
    let manager = MANAGER.get().expect("manager not set");
    Json(manager.get_info())
}

async fn get_performance() -> Json<Value> {
    let manager = MANAGER.get().expect("manager not set");
    Json(manager.get_performance())
}

#[derive(Deserialize)]
struct UpdateParams {
    key: String,
    val: String,
}

async fn post_update_settings(Form(params): Form<UpdateParams>) -> Json<Value> {
    let manager = MANAGER.get().expect("manager not set");
    let val: Value = serde_json::from_str(&params.val).unwrap_or(Value::String(params.val));
    Json(manager.update_settings(&params.key, val))
}

#[derive(Deserialize)]
struct LoadModelParams {
    slot: i32,
    isHalf: bool,
    params: String,
}

async fn post_load_model(Form(_params): Form<LoadModelParams>) -> Json<Value> {
    let manager = MANAGER.get().expect("manager not set");
    // placeholder implementation
    manager.load_model("dummy".to_string());
    Json(manager.get_info())
}

#[derive(Deserialize)]
struct MergeParams {
    request: String,
}

async fn post_merge_models(Form(params): Form<MergeParams>) -> Json<Value> {
    let manager = MANAGER.get().expect("manager not set");
    Json(manager.merge_models(&params.request))
}

async fn get_onnx() -> Json<Value> {
    let manager = MANAGER.get().expect("manager not set");
    Json(json!({"exported": manager.export_to_onnx()}))
}

async fn post_update_model_default() -> Json<Value> {
    let manager = MANAGER.get().expect("manager not set");
    Json(manager.update_model_default())
}

#[derive(Deserialize)]
struct UpdateModelInfo {
    newData: String,
}

async fn post_update_model_info(Form(params): Form<UpdateModelInfo>) -> Json<Value> {
    let manager = MANAGER.get().expect("manager not set");
    Json(manager.update_model_info(&params.newData))
}

#[derive(Deserialize)]
struct UploadModelAssets {
    params: String,
}

async fn post_upload_model_assets(Form(params): Form<UploadModelAssets>) -> Json<Value> {
    let manager = MANAGER.get().expect("manager not set");
    Json(manager.upload_model_assets(&params.params))
}

pub struct MMVCRestFileuploader {
    router: Router,
}

impl MMVCRestFileuploader {
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

    pub fn router(self) -> Router { self.router }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::{Request, StatusCode}, Router};
    use tower::ServiceExt;
    use serde_json::Value;

    use crate::voice_changer_params::VoiceChangerParams;
    use crate::mmvc_rest::MMVCRest; // for constructing manager

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
        MMVCRestFileuploader::new(manager).router()
    }

    #[tokio::test]
    async fn info_endpoint_returns_status() {
        let app = app();
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
    }

    #[tokio::test]
    async fn update_settings_endpoint_changes_value() {
        let app = app();
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
    }
}

