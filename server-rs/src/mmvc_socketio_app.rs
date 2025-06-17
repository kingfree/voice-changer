use axum::{http::StatusCode, response::IntoResponse, routing::get_service, Router};
use tower_http::{services::ServeDir, trace::TraceLayer};

use crate::{
    constants::{get_frontend_path, MODEL_DIR_STATIC, TMP_DIR, UPLOAD_DIR},
    mmvc_socketio_server::MMVCSocketIOServer,
    voice_changer_manager::VoiceChangerManager,
};

pub struct MMVCSocketIOApp {
    router: Router,
}

impl MMVCSocketIOApp {
    pub fn new(rest_router: Router, manager: &'static VoiceChangerManager) -> Self {
        let socket_layer = MMVCSocketIOServer::new(manager).layer();

        // static file serving similar to the Python implementation
        let frontend = get_frontend_path();
        let serve_front = |dir: std::path::PathBuf| {
            get_service(ServeDir::new(dir).append_index_html_on_directories(true))
                .handle_error(|_| async { StatusCode::INTERNAL_SERVER_ERROR.into_response() })
        };

        let router = rest_router
            .nest_service("/front", serve_front(frontend.clone()))
            .nest_service("/trainer", serve_front(frontend.clone()))
            .nest_service("/recorder", serve_front(frontend.clone()))
            .nest_service(
                format!("/{}", manager.model_dir()).as_str(),
                serve_front(std::path::PathBuf::from(manager.model_dir())),
            )
            .nest_service("/tmp", serve_front(std::path::PathBuf::from(TMP_DIR)))
            .nest_service(
                "/upload_dir",
                serve_front(std::path::PathBuf::from(UPLOAD_DIR)),
            )
            .nest_service(
                "/model_dir_static",
                serve_front(std::path::PathBuf::from(MODEL_DIR_STATIC)),
            )
            .fallback_service(serve_front(frontend))
            .layer(socket_layer)
            .layer(TraceLayer::new_for_http());

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
    use serial_test::serial;
    use tower::ServiceExt;

    use crate::{
        mmvc_rest::MMVCRest, test_util::cleanup_test_dirs, voice_changer_params::VoiceChangerParams,
    };

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
        let rest = MMVCRest::new(manager);
        (
            MMVCSocketIOApp::new(rest.router(), manager).router(),
            manager,
        )
    }

    #[tokio::test]
    #[serial]
    async fn socketio_endpoint_available() {
        let (app, manager) = app();
        let req = Request::builder()
            .uri("/socket.io/?EIO=4&transport=polling&t=0")
            .method("GET")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        manager.reset();
        cleanup_test_dirs();
    }
}
