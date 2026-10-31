use std::fs;
use std::io::{self, Write};
use std::path::Path;

use tempfile::NamedTempFile;

/// Writes data to a new destination without exposing a partially
/// written final file or overwriting an existing destination.
///
/// The temporary file is created in the destination directory.
/// The completed file is published using a hard link, which fails
/// if the destination already exists.
pub fn write_new_file(destination: &Path, data: &[u8]) -> io::Result<()> {
    write_new_file_with(destination, data, |temporary, bytes| {
        temporary.write_all(bytes)
    })
}

/// Creates a temporary file in the destination directory, lets
/// the caller stream data into it, and publishes it only if the
/// operation succeeds.
///
/// The destination is never overwritten. If the operation fails,
/// the temporary file is cleaned up when possible.
pub fn write_new_file_with_writer<F>(
    destination: &Path,
    write_data: F,
) -> io::Result<()>
where
    F: FnOnce(&mut NamedTempFile) -> io::Result<()>,
{
    let parent = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));

    let mut temporary = NamedTempFile::new_in(parent)?;

    // Stream into the temporary file. Do not publish it if
    // the operation fails.
    write_data(&mut temporary)?;

    temporary.as_file().sync_all()?;

    // Creating a hard link fails if the destination already exists.
    fs::hard_link(temporary.path(), destination)?;

    Ok(())
}

/// Internal implementation with an injectable write operation.
///
/// The injection point allows tests to simulate a write failure
/// without relying on a full disk or other unpredictable conditions.
fn write_new_file_with<F>(destination: &Path, data: &[u8], write_data: F) -> io::Result<()>
where
    F: FnOnce(&mut NamedTempFile, &[u8]) -> io::Result<()>,
{
    let parent = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));

    let mut temporary = NamedTempFile::new_in(parent)?;

    // If this fails, the temporary file is dropped and the
    // final destination has not been published.
    write_data(&mut temporary, data)?;

    temporary.as_file().sync_all()?;

    // This must not replace an existing destination.
    fs::hard_link(temporary.path(), destination)?;

    // Dropping the temporary file removes its temporary name.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::ErrorKind;
    use tempfile::tempdir;

    #[test]
    fn writes_exact_contents() {
        let directory = tempdir().unwrap();
        let destination = directory.path().join("output.bin");
        let expected = b"FileKit test data";

        write_new_file(&destination, expected).unwrap();

        assert_eq!(fs::read(&destination).unwrap(), expected);
    }

    #[test]
    fn refuses_to_overwrite_existing_file() {
        let directory = tempdir().unwrap();
        let destination = directory.path().join("existing.bin");

        fs::write(&destination, b"original contents").unwrap();

        let result = write_new_file(&destination, b"replacement contents");

        assert_eq!(result.unwrap_err().kind(), ErrorKind::AlreadyExists);
        assert_eq!(fs::read(&destination).unwrap(), b"original contents");
    }

    #[test]
    fn simulated_write_failure_publishes_no_destination() {
        let directory = tempdir().unwrap();
        let destination = directory.path().join("failed.bin");

        let result = write_new_file_with(
            &destination,
            b"data that must not be published",
            |_temporary, _bytes| Err(io::Error::other("simulated write failure")),
        );

        assert!(result.is_err());
        assert!(!destination.exists());

        // The failed operation should also clean up its temporary
        // file when the temporary-file cleanup succeeds.
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
    }

    #[test]
    fn missing_parent_directory_fails_without_creating_destination() {
        let directory = tempdir().unwrap();
        let destination = directory.path().join("missing").join("output.bin");

        let result = write_new_file(&destination, b"test data");

        assert!(result.is_err());
        assert!(!destination.exists());
        assert!(!destination.parent().unwrap().exists());
    }

    #[test]
    fn failed_publication_preserves_existing_contents() {
        let directory = tempdir().unwrap();
        let destination = directory.path().join("existing.bin");

        fs::write(&destination, b"keep these bytes").unwrap();

        let result = write_new_file_with(&destination, b"new bytes", |temporary, bytes| {
            temporary.write_all(bytes)
        });

        assert_eq!(result.unwrap_err().kind(), ErrorKind::AlreadyExists);
        assert_eq!(fs::read(&destination).unwrap(), b"keep these bytes");

        // Only the original destination should remain.
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }
}
