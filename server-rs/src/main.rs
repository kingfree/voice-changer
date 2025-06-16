use axum::{routing::{get, post}, Router, Json};
use serde::{Deserialize, Serialize};
use clap::Parser;
use std::{net::SocketAddr, path::Path};
use axum_server::tls_rustls::RustlsConfig;
use rcgen::generate_simple_self_signed;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Port to listen on
    #[arg(short = 'p', long, default_value_t = 18888)]
    port: u16,

    /// Host address to bind
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Enable HTTPS
    #[arg(long, default_value_t = false)]
    https: bool,

    /// Path to TLS private key
    #[arg(long, default_value = "ssl.key")]
    https_key: String,

    /// Path to TLS certificate
    #[arg(long, default_value = "ssl.cert")]
    https_cert: String,

    /// Generate self-signed certificate when using HTTPS
    #[arg(long, default_value_t = true)]
    https_self_signed: bool,
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

    if args.https {
        // generate self-signed certificate if requested
        if args.https_self_signed
            && (!Path::new(&args.https_key).exists() || !Path::new(&args.https_cert).exists())
        {
            let cert = generate_simple_self_signed(["localhost".into()]).unwrap();
            std::fs::write(&args.https_key, cert.key_pair.serialize_pem()).unwrap();
            std::fs::write(&args.https_cert, cert.cert.pem()).unwrap();
        }

        println!("Starting HTTPS server on https://{}", addr);
        let config = RustlsConfig::from_pem_file(&args.https_cert, &args.https_key)
            .await
            .expect("failed to load certs");
        axum_server::bind_rustls(addr, config)
            .serve(app.into_make_service())
            .await
            .unwrap();
    } else {
        println!("Starting server on http://{}", addr);
        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .await
            .unwrap();
    }
}
