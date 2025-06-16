use once_cell::sync::OnceCell;
use std::sync::RwLock;

use crate::voice_changer_params::VoiceChangerParams;

pub struct VoiceChangerParamsManager {
    params: RwLock<Option<VoiceChangerParams>>,
}

static INSTANCE: OnceCell<VoiceChangerParamsManager> = OnceCell::new();

impl VoiceChangerParamsManager {
    pub fn get_instance() -> &'static VoiceChangerParamsManager {
        INSTANCE.get_or_init(|| VoiceChangerParamsManager {
            params: RwLock::new(None),
        })
    }

    pub fn set_params(&self, params: VoiceChangerParams) {
        let mut guard = self.params.write().unwrap();
        *guard = Some(params);
    }

    pub fn params(&self) -> Option<VoiceChangerParams> {
        self.params.read().unwrap().clone()
    }
}
