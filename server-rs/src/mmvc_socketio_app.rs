use axum::{http::StatusCode, response::IntoResponse, routing::get_service, Router};
use tower_http::services::ServeDir;

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
            .layer(socket_layer)
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
            .fallback_service(serve_front(frontend));

        Self { router }
    }

    pub fn router(self) -> Router {
        self.router
    }
}
