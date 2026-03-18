use std::fs;
use std::path::Path;

use super::ApiError;

pub(super) fn atomic_write(path: &Path, content: &str) -> Result<(), ApiError> {
    let directory = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(directory)?;

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            ApiError::InvalidArguments("target_path must have a file name".to_owned())
        })?;
    let temp_path = directory.join(format!(".{file_name}.ferrocat.tmp"));
    fs::write(&temp_path, content)?;
    fs::rename(&temp_path, path)?;
    Ok(())
}
