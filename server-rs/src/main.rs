use axum::{
    routing::{get, post},
    Json, Router,
};
use axum_server::tls_rustls::RustlsConfig;
use clap::Parser;
use rcgen::generate_simple_self_signed;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, path::Path};

mod voice_changer_params;
use voice_changer_params::VoiceChangerParams;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Log level
    #[arg(long, default_value = "error")]
    log_level: String,

    /// Port to listen on
    #[arg(short = 'p', long, default_value_t = 18888)]
    port: u16,

    /// Host address to bind
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// List of allowed origins
    #[arg(long, value_name = "ORIGIN")]
    allowed_origins: Vec<String>,

    /// Enable HTTPS
    #[arg(long, default_value_t = false)]
    https: bool,

    /// Test connect target (for HTTPS IP detection)
    #[arg(long, default_value = "8.8.8.8")]
    test_connect: String,

    /// Path to TLS private key
    #[arg(long, default_value = "ssl.key")]
    https_key: String,

    /// Path to TLS certificate
    #[arg(long, default_value = "ssl.cert")]
    https_cert: String,

    /// Generate self-signed certificate when using HTTPS
    #[arg(long, default_value_t = true)]
    https_self_signed: bool,

    /// Path to model directory
    #[arg(long, default_value = "model_dir")]
    model_dir: String,

    /// RVC sample mode
    #[arg(long, default_value = "production")]
    sample_mode: String,

    #[arg(long, default_value = "pretrain/checkpoint_best_legacy_500.pt")]
    content_vec_500: String,

    #[arg(long, default_value = "pretrain/content_vec_500.onnx")]
    content_vec_500_onnx: String,

    #[arg(long, default_value_t = true)]
    content_vec_500_onnx_on: bool,

    #[arg(long, default_value = "pretrain/hubert_base.pt")]
    hubert_base: String,

    #[arg(long, default_value = "pretrain/rinna_hubert_base_jp.pt")]
    hubert_base_jp: String,

    #[arg(long, default_value = "pretrain/hubert/hubert-soft-0d54a1f4.pt")]
    hubert_soft: String,

    #[arg(long, default_value = "pretrain/whisper_tiny.pt")]
    whisper_tiny: String,

    #[arg(long, default_value = "pretrain/nsf_hifigan/model")]
    nsf_hifigan: String,

    #[arg(long, default_value = "pretrain/crepe_onnx_full.onnx")]
    crepe_onnx_full: String,

    #[arg(long, default_value = "pretrain/crepe_onnx_tiny.onnx")]
    crepe_onnx_tiny: String,

    #[arg(long, default_value = "pretrain/rmvpe.pt")]
    rmvpe: String,

    #[arg(long, default_value = "pretrain/rmvpe.onnx")]
    rmvpe_onnx: String,
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

    let _vc_params = VoiceChangerParams {
        model_dir: args.model_dir.clone(),
        content_vec_500: args.content_vec_500.clone(),
        content_vec_500_onnx: args.content_vec_500_onnx.clone(),
        content_vec_500_onnx_on: args.content_vec_500_onnx_on,
        hubert_base: args.hubert_base.clone(),
        hubert_base_jp: args.hubert_base_jp.clone(),
        hubert_soft: args.hubert_soft.clone(),
        nsf_hifigan: args.nsf_hifigan.clone(),
        sample_mode: args.sample_mode.clone(),
        crepe_onnx_full: args.crepe_onnx_full.clone(),
        crepe_onnx_tiny: args.crepe_onnx_tiny.clone(),
        rmvpe: args.rmvpe.clone(),
        rmvpe_onnx: args.rmvpe_onnx.clone(),
        whisper_tiny: args.whisper_tiny.clone(),
    };
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
