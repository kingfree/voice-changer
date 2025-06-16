use std::path::{Path, PathBuf};
use reqwest::Client;
use serde_json::Value;

use crate::voice_changer_params::VoiceChangerParams;

pub async fn download_file(client: &Client, url: &str, save_to: &Path) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Some(parent) = save_to.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let bytes = client.get(url).send().await?.bytes().await?;
    tokio::fs::write(save_to, &bytes).await?;
    Ok(())
}

pub async fn download_weight(params: &VoiceChangerParams) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = Client::new();
    let mut tasks = Vec::new();

    let mappings = vec![
        (&params.hubert_base as &str, "https://huggingface.co/ddPn08/rvc-webui-models/resolve/main/embeddings/hubert_base.pt"),
        (&params.hubert_base_jp as &str, "https://huggingface.co/rinna/japanese-hubert-base/resolve/main/fairseq/model.pt"),
        (&params.hubert_soft as &str, "https://huggingface.co/wok000/weights/resolve/main/ddsp-svc30/embedder/hubert-soft-0d54a1f4.pt"),
        (&params.nsf_hifigan as &str, "https://huggingface.co/wok000/weights/resolve/main/ddsp-svc30/nsf_hifigan_20221211/model.bin"),
        (&params.crepe_onnx_full as &str, "https://huggingface.co/wok000/weights/resolve/main/crepe/onnx/full.onnx"),
        (&params.crepe_onnx_tiny as &str, "https://huggingface.co/wok000/weights/resolve/main/crepe/onnx/tiny.onnx"),
        (&params.content_vec_500_onnx as &str, "https://huggingface.co/wok000/weights_gpl/resolve/main/content-vec/contentvec-f.onnx"),
        (&params.rmvpe as &str, "https://huggingface.co/wok000/weights/resolve/main/rmvpe/rmvpe_20231006.pt"),
        (&params.rmvpe_onnx as &str, "https://huggingface.co/wok000/weights_gpl/resolve/main/rmvpe/rmvpe_20231006.onnx"),
        (&params.whisper_tiny as &str, "https://openaipublic.azureedge.net/main/whisper/models/65147644a518d12f04e32d6f3b26facc3f8dd46e5390956a9424a650c0ce22b9/tiny.pt"),
    ];

    for (path, url) in mappings {
        if !Path::new(path).exists() {
            let p = PathBuf::from(path);
            let u = url.to_string();
            let client = client.clone();
            tasks.push(tokio::spawn(async move { download_file(&client, &u, &p).await }));
        }
    }

    // additional files relative to nsf_hifigan
    if !Path::new(&params.nsf_hifigan).exists() {
        let config_path = PathBuf::from(&params.nsf_hifigan).with_file_name("config.json");
        let onnx_path = PathBuf::from(&params.nsf_hifigan).with_file_name("nsf_hifigan.onnx");
        if !config_path.exists() {
            let client = client.clone();
            let cp = config_path.clone();
            tasks.push(tokio::spawn(async move {
                download_file(
                    &client,
                    "https://huggingface.co/wok000/weights/raw/main/ddsp-svc30/nsf_hifigan_20221211/config.json",
                    &cp,
                )
                .await
            }));
        }
        if !onnx_path.exists() {
            let client = client.clone();
            let op = onnx_path.clone();
            tasks.push(tokio::spawn(async move {
                download_file(
                    &client,
                    "https://huggingface.co/wok000/weights/resolve/main/ddsp-svc30/nsf_hifigan_onnx_20221211/nsf_hifigan.onnx",
                    &op,
                )
                .await
            }));
        }
    }

    for t in tasks {
        t.await??;
    }

    Ok(())
}

#[derive(Default)]
struct SampleModelParam {
    use_index: bool,
}

struct Sample {
    id: String,
    model_url: String,
    index_url: Option<String>,
    icon: Option<String>,
}

pub async fn download_initial_samples(mode: &str, model_dir: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if tokio::fs::metadata(model_dir).await.is_ok() {
        println!("[Voice Changer] model_dir already exists. skip download samples.");
        return Ok(());
    }

    let (json_urls, sample_models) = get_sample_json_and_model_ids(mode);
    let client = Client::new();
    let mut samples: Vec<Sample> = Vec::new();

    for url in json_urls {
        let text = client.get(url).send().await?.text().await?;
        let v: Value = serde_json::from_str(&text)?;
        for arr in v.as_object().unwrap().values() {
            if let Some(list) = arr.as_array() {
                for item in list {
                    let id = item["id"].as_str().unwrap().to_string();
                    let model_url = item["modelUrl"].as_str().unwrap().to_string();
                    let index_url = item.get("indexUrl").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let icon = item.get("icon").and_then(|v| v.as_str()).map(|s| s.to_string());
                    samples.push(Sample { id, model_url, index_url, icon });
                }
            }
        }
    }

    for (i, (sample_id, param)) in sample_models.iter().enumerate() {
        if let Some(sample) = samples.iter().find(|s| &s.id == sample_id) {
            let slot_dir = Path::new(model_dir).join(i.to_string());
            tokio::fs::create_dir_all(&slot_dir).await?;
            let model_path = slot_dir.join(Path::new(&sample.model_url).file_name().unwrap());
            download_file(&client, &sample.model_url, &model_path).await?;
            if param.use_index {
                if let Some(index_url) = &sample.index_url {
                    let index_path = slot_dir.join(Path::new(index_url).file_name().unwrap());
                    download_file(&client, index_url, &index_path).await?;
                }
            }
            if let Some(icon) = &sample.icon {
                let icon_path = slot_dir.join(Path::new(icon).file_name().unwrap());
                download_file(&client, icon, &icon_path).await?;
            }
        }
    }

    Ok(())
}

fn get_sample_json_and_model_ids(mode: &str) -> (Vec<&'static str>, Vec<(&'static str, SampleModelParam)>) {
    match mode {
        "production" => (
            vec![
                "https://huggingface.co/wok000/vcclient_model/raw/main/samples_0004_t.json",
                "https://huggingface.co/wok000/vcclient_model/raw/main/samples_0004_o.json",
                "https://huggingface.co/wok000/vcclient_model/raw/main/samples_0004_d.json",
            ],
            vec![
                ("Tsukuyomi-chan_o", SampleModelParam { use_index: false }),
                ("Amitaro_o", SampleModelParam { use_index: false }),
                ("KikotoMahiro_o", SampleModelParam { use_index: false }),
                ("TokinaShigure_o", SampleModelParam { use_index: false }),
            ],
        ),
        _ => (Vec::new(), Vec::new()),
    }
}
