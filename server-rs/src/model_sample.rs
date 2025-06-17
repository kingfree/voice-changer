use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ModelSample {
    pub id: String,
    pub voice_changer_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RVCModelSample {
    #[serde(flatten)]
    pub base: ModelSample,
    pub lang: String,
    pub tag: Vec<String>,
    pub name: String,
    pub model_url: String,
    pub index_url: String,
    pub terms_of_use_url: String,
    pub icon: String,
    pub credit: String,
    pub description: String,
    pub sample_rate: i32,
    pub model_type: String,
    pub f0: bool,
}

impl Default for RVCModelSample {
    fn default() -> Self {
        Self {
            base: ModelSample {
                voice_changer_type: Some("RVC".into()),
                ..Default::default()
            },
            lang: String::new(),
            tag: Vec::new(),
            name: String::new(),
            model_url: String::new(),
            index_url: String::new(),
            terms_of_use_url: String::new(),
            icon: String::new(),
            credit: String::new(),
            description: String::new(),
            sample_rate: 48000,
            model_type: String::new(),
            f0: true,
        }
    }
}

