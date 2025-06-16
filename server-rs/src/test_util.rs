#[cfg(test)]
pub fn cleanup_test_dirs() {
    let _ = std::fs::remove_dir_all("m");
    let _ = std::fs::remove_dir_all("upload_dir");
    let _ = std::fs::remove_dir_all("tmp_dir");
}
