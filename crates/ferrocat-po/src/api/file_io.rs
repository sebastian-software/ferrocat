use std::fs;
use std::io::Write;
use std::path::Path;

use super::ApiError;

pub(super) fn atomic_write(path: &Path, content: &str) -> Result<(), ApiError> {
    let directory = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(directory)?;

    if path.file_name().is_none() {
        return Err(ApiError::InvalidArguments(
            "target_path must have a file name".to_owned(),
        ));
    }
    let mut temp_file = tempfile::NamedTempFile::new_in(directory)?;
    temp_file.write_all(content.as_bytes())?;
    temp_file.as_file().sync_all()?;
    temp_file.persist(path).map_err(|error| error.error)?;
    sync_directory(directory)?;
    Ok(())
}

#[cfg(unix)]
fn sync_directory(directory: &Path) -> Result<(), ApiError> {
    fs::File::open(directory)?.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn sync_directory(_directory: &Path) -> Result<(), ApiError> {
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
        let files = fs::read_dir(parent)
            .expect("read parent")
            .map(|entry| entry.expect("dir entry").file_name())
            .collect::<Vec<_>>();
        assert_eq!(files, vec![std::ffi::OsString::from("catalog.po")]);

        let root = target.ancestors().nth(2).expect("temp root").to_path_buf();
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn atomic_write_does_not_use_the_legacy_fixed_temp_name() {
        let target = unique_temp_path("atomic-write-fixed-temp").join("catalog.po");
        let parent = target.parent().expect("parent");
        fs::create_dir_all(parent).expect("create parent");
        let legacy_temp = parent.join(".catalog.po.ferrocat.tmp");
        fs::write(&legacy_temp, "sentinel").expect("write legacy temp");

        atomic_write(&target, "content").expect("write target");

        assert_eq!(fs::read_to_string(&target).expect("read target"), "content");
        assert_eq!(
            fs::read_to_string(&legacy_temp).expect("read legacy temp"),
            "sentinel"
        );

        let root = target.ancestors().nth(1).expect("temp root").to_path_buf();
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
