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
