use axum_server::tls_rustls::RustlsConfig;
use clap::Parser;
use rcgen::generate_simple_self_signed;

use std::{net::SocketAddr, path::Path};
use tracing_subscriber::EnvFilter;

mod voice_changer_params;
use voice_changer_params::VoiceChangerParams;
mod model_slot;
mod plugin;
mod rvc;
mod voice_changer;
mod voice_changer_params_manager;
use voice_changer_params_manager::VoiceChangerParamsManager;
mod voice_changer_manager;
use voice_changer_manager::VoiceChangerManager;
mod mmvc_rest;
mod mmvc_socketio_app;
mod mmvc_socketio_server;
use mmvc_rest::MMVCRest;
use mmvc_socketio_app::MMVCSocketIOApp;
mod downloader;
use downloader::{download_initial_samples, download_weight};
mod constants;

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

async fn local_server(args: Args) {
    let vc_params = VoiceChangerParams {
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

    // download required weights and sample models
    if let Err(e) = download_weight(&vc_params).await {
        eprintln!("failed to download weights: {e}");
    }
    if let Err(e) = download_initial_samples(&args.sample_mode, &args.model_dir).await {
        eprintln!("failed to download samples: {e}");
    }

    VoiceChangerParamsManager::get_instance().set_params(vc_params.clone());
    let manager = VoiceChangerManager::get_instance(vc_params.clone());
    let rest = MMVCRest::new(manager);
    let socket_app = MMVCSocketIOApp::new(rest.router(), manager);
    let app = socket_app.router();

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

#[tokio::main]
async fn main() {
    let args = Args::parse();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(args.log_level.clone()))
        .init();
    local_server(args).await;
}
