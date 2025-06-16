use axum::{routing::get, extract::ws::{WebSocket, WebSocketUpgrade, Message}, Router, response::IntoResponse};
use futures_util::StreamExt;

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    while let Some(Ok(msg)) = socket.next().await {
        match msg {
            Message::Text(text) => {
                if socket.send(Message::Text(text)).await.is_err() {
                    return;
                }
            }
            Message::Binary(bin) => {
                if socket.send(Message::Binary(bin)).await.is_err() {
                    return;
                }
            }
            Message::Close(_) => return,
            _ => {}
        }
    }
}

pub struct MMVCSocketIOApp {
    router: Router,
}

impl MMVCSocketIOApp {
    pub fn new(rest_router: Router) -> Self {
        let router = rest_router.route("/ws", get(ws_handler));
        Self { router }
    }

    pub fn router(self) -> Router {
        self.router
    }
}
