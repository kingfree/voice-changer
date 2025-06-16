use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};

async fn openapi() -> Response {
    static SPEC: &str = include_str!("../openapi.json");
    Response::builder()
        .header("Content-Type", "application/json")
        .body(SPEC.into())
        .unwrap()
}

async fn version() -> &'static str {
    "0.1"
}

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            Message::Binary(bin) => {
                println!("Received {} bytes", bin.len());
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
}

pub fn app() -> Router {
    Router::new()
        .route("/version", get(version))
        .route("/ws/audio", get(ws_handler))
        .route("/openapi.json", get(openapi))
}
