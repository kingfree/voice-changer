use axum::Router;

use crate::{
    voice_changer_manager::VoiceChangerManager,
    mmvc_socketio_server::MMVCSocketIOServer,
};

pub struct MMVCSocketIOApp {
    router: Router,
}

impl MMVCSocketIOApp {
    pub fn new(rest_router: Router, manager: &'static VoiceChangerManager) -> Self {
        let socket_router = MMVCSocketIOServer::new(manager).router();
        let router = rest_router.merge(socket_router);
        Self { router }
    }

    pub fn router(self) -> Router {
        self.router
    }
}
