use std::path::PathBuf;

/// Return the path to the frontend assets directory.
///
/// When the `MEIPASS` environment variable is set (used when the
/// application is packaged similarly to the Python version), the
/// assets are expected to be located in `MEIPASS/dist`. Otherwise the
/// default development path `../client/demo/dist` is used.
pub fn get_frontend_path() -> PathBuf {
    match std::env::var("MEIPASS") {
        Ok(p) => PathBuf::from(p).join("dist"),
        Err(_) => PathBuf::from("../client/demo/dist"),
    }
}

/// Directory used for temporary files.
pub const TMP_DIR: &str = "tmp_dir";
/// Directory used for uploaded files.
pub const UPLOAD_DIR: &str = "upload_dir";
/// Directory containing static models.
pub const MODEL_DIR_STATIC: &str = "model_dir_static";
/// File to persist settings.
pub const STORED_SETTING_FILE: &str = "stored_setting.json";

/// Directory containing uploaded model artifacts.
pub const MODEL_DIR: &str = "logs";

/// Directory used to store generated SSL certificates.
pub const SSL_KEY_DIR: &str = "keys";

/// Executable name for the optional native client on Windows.
pub fn native_client_file_win() -> PathBuf {
    match std::env::var("MEIPASS") {
        Ok(p) => PathBuf::from(p).join("voice-changer-native-client.exe"),
        Err(_) => PathBuf::from("voice-changer-native-client"),
    }
}

/// Executable path for the optional native client on macOS.
pub fn native_client_file_mac() -> PathBuf {
    match std::env::var("MEIPASS") {
        Ok(p) => PathBuf::from(p)
            .join("voice-changer-native-client.app")
            .join("Contents")
            .join("MacOS")
            .join("voice-changer-native-client"),
        Err(_) => PathBuf::from("voice-changer-native-client"),
    }
}

/// Path to the bundled Hubert ONNX model used by some configurations.
pub fn hubert_onnx_model_path() -> PathBuf {
    match std::env::var("MEIPASS") {
        Ok(p) => PathBuf::from(p)
            .join("model_hubert")
            .join("hubert_simple.onnx"),
        Err(_) => PathBuf::from("model_hubert/hubert_simple.onnx"),
    }
}

/// List of sample rates supported by the server audio device code.
pub const SERVER_DEVICE_SAMPLE_RATES: &[i32] = &[16000, 32000, 44100, 48000, 96000, 192000];

/// Directory name reserved for bundled RVC models.
pub const RVC_MODEL_DIRNAME: &str = "rvc";

/// Maximum number of dynamic model slots that can be created.
pub const MAX_SLOT_NUM: usize = 500;
