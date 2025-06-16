use axum::{routing::{get, post}, Router, Json};
use serde::{Deserialize, Serialize};
use clap::Parser;
use std::net::SocketAddr;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Port to listen on
    #[arg(short = 'p', long, default_value_t = 18888)]
    port: u16,

    /// Host address to bind
    #[arg(long, default_value = "127.0.0.1")]
    host: String,
}

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
    // In the Python implementation this would run the voice changer.
    // Here we simply echo back the input buffer.
    Json(TestResponse {
        timestamp: payload.timestamp,
        changed_voice_base64: payload.buffer,
    })
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let app = Router::new()
        .route("/api/hello", get(hello))
        .route("/test", post(test));

    let addr = SocketAddr::new(args.host.parse().unwrap(), args.port);
    println!("Starting server on http://{}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
