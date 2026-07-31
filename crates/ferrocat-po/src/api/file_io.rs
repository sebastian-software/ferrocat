use std::fs;
use std::io;
use std::io::Write;
use std::path::Path;

use super::{ApiError, WriteDurability};

pub(super) fn atomic_write(path: &Path, content: &str) -> Result<(), ApiError> {
    atomic_write_with_durability(path, content, WriteDurability::Full)
}

pub(super) fn atomic_write_with_durability(
    path: &Path,
    content: &str,
    durability: WriteDurability,
) -> Result<(), ApiError> {
    atomic_write_with_sync(
        path,
        content,
        durability,
        fs::File::sync_all,
        sync_directory,
    )
}

fn atomic_write_with_sync<FileSync, DirectorySync>(
    path: &Path,
    content: &str,
    durability: WriteDurability,
    file_sync: FileSync,
    directory_sync: DirectorySync,
) -> Result<(), ApiError>
where
    FileSync: FnOnce(&fs::File) -> io::Result<()>,
    DirectorySync: FnOnce(&Path) -> io::Result<()>,
{
    let should_sync = match durability {
        WriteDurability::Full => true,
        WriteDurability::Rename => false,
    };
    let directory = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(directory).map_err(|error| ApiError::io_with_path(directory, error))?;

    if path.file_name().is_none() {
        return Err(ApiError::InvalidArguments(
            "target_path must have a file name".to_owned(),
        ));
    }
    let mut temp_file = tempfile::NamedTempFile::new_in(directory)
        .map_err(|error| ApiError::io_with_path(directory, error))?;
    temp_file
        .write_all(content.as_bytes())
        .map_err(|error| ApiError::io_with_path(path, error))?;
    if should_sync {
        file_sync(temp_file.as_file()).map_err(|error| ApiError::io_with_path(path, error))?;
    }
    temp_file
        .persist(path)
        .map_err(|error| ApiError::io_with_path(path, error.error))?;
    if should_sync {
        directory_sync(directory).map_err(|error| ApiError::io_with_path(directory, error))?;
    }
    Ok(())
}

#[cfg(unix)]
fn sync_directory(directory: &Path) -> io::Result<()> {
    fs::File::open(directory).and_then(|file| file.sync_all())
}

#[cfg(not(unix))]
fn sync_directory(_directory: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::fs;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{atomic_write, atomic_write_with_sync};
    use crate::api::{ApiError, WriteDurability};

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

    #[test]
    fn atomic_write_requests_syncs_only_for_full_durability() {
        let target = unique_temp_path("atomic-write-durability").join("catalog.po");
        let file_syncs = Cell::new(0);
        let directory_syncs = Cell::new(0);
        let file_sync = |_: &fs::File| {
            file_syncs.set(file_syncs.get() + 1);
            Ok(())
        };
        let directory_sync = |_: &Path| {
            directory_syncs.set(directory_syncs.get() + 1);
            Ok(())
        };

        atomic_write_with_sync(
            &target,
            "full",
            WriteDurability::Full,
            file_sync,
            directory_sync,
        )
        .expect("full durability write");

        assert_eq!(file_syncs.get(), 1);
        assert_eq!(directory_syncs.get(), 1);

        atomic_write_with_sync(
            &target,
            "rename",
            WriteDurability::Rename,
            file_sync,
            directory_sync,
        )
        .expect("rename durability write");

        assert_eq!(file_syncs.get(), 1);
        assert_eq!(directory_syncs.get(), 1);
        assert_eq!(fs::read_to_string(&target).expect("read target"), "rename");

        let parent = target.parent().expect("parent");
        let files = fs::read_dir(parent)
            .expect("read parent")
            .map(|entry| entry.expect("dir entry").file_name())
            .collect::<Vec<_>>();
        assert_eq!(files, vec![std::ffi::OsString::from("catalog.po")]);

        let root = target.ancestors().nth(1).expect("temp root").to_path_buf();
        fs::remove_dir_all(root).expect("cleanup");
    }
}
