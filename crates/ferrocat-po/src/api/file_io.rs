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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::atomic_write;
    use crate::api::ApiError;

    fn unique_temp_path(name: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir()
            .join("ferrocat-tests")
            .join(format!("{name}-{nanos}"))
    }

    #[test]
    fn atomic_write_creates_missing_directories_and_overwrites_target() {
        let target = unique_temp_path("atomic-write").join("nested/catalog.po");
        atomic_write(&target, "first").expect("write first");
        assert_eq!(fs::read_to_string(&target).expect("read first"), "first");

        atomic_write(&target, "second").expect("write second");
        assert_eq!(fs::read_to_string(&target).expect("read second"), "second");

        let parent = target.parent().expect("parent");
        let temp_file = parent.join(format!(
            ".{}.ferrocat.tmp",
            target
                .file_name()
                .and_then(|name| name.to_str())
                .expect("file name")
        ));
        assert!(!temp_file.exists());

        let root = target.ancestors().nth(2).expect("temp root").to_path_buf();
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn atomic_write_rejects_paths_without_a_file_name() {
        let error = atomic_write(Path::new(""), "ignored").expect_err("invalid path");
        assert!(matches!(
            error,
            ApiError::InvalidArguments(message) if message.contains("file name")
        ));
    }
}
