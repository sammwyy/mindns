use std::path::PathBuf;

pub fn get_path(raw_path: &str) -> PathBuf {
    if raw_path.starts_with(".") {
        let current_dir = std::env::current_dir().unwrap();
        let path = current_dir.join(raw_path);
        path
    } else {
        PathBuf::from(raw_path)
    }
}
