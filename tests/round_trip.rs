use filekit::{
    decode_file,
    decode_reader,
    encode_file,
    encode_reader,
    write_new_file_with_writer,
    FileKitError,
};

use std::io;
use tempfile::tempdir;

// A file should survive a complete protect-and-restore cycle.
#[test]
fn round_trip_text() {
    let original = b"Hello from Rust!";

    let protected = encode_file(original, "test passphrase").expect("protection should succeed");

    let restored = decode_file(&protected, "test passphrase").expect("restoration should succeed");

    assert_eq!(restored, original);
}

// The library must handle arbitrary bytes, not just text.
#[test]
fn round_trip_binary_data() {
    let original: Vec<u8> = (0..=255).collect();

    let protected =
        encode_file(&original, "another test passphrase").expect("protection should succeed");

    let restored =
        decode_file(&protected, "another test passphrase").expect("restoration should succeed");

    assert_eq!(restored, original);
}

// Empty files are valid inputs.
#[test]
fn round_trip_empty_file() {
    let original: &[u8] = &[];

    let protected = encode_file(original, "empty-file test").expect("protection should succeed");

    let restored = decode_file(&protected, "empty-file test").expect("restoration should succeed");

    assert_eq!(restored, original);
}

// A wrong passphrase must never return plaintext.
#[test]
fn rejects_wrong_passphrase() {
    let protected =
        encode_file(b"secret data", "correct passphrase").expect("protection should succeed");

    let result = decode_file(&protected, "incorrect passphrase");

    assert!(matches!(result, Err(FileKitError::ProcessingFailed)));
}

// Modifying the encrypted bytes must invalidate authentication.
#[test]
fn rejects_modified_ciphertext() {
    let mut protected =
        encode_file(b"secret data", "test passphrase").expect("protection should succeed");

    // Change the final byte, which belongs to the authentication tag.
    let last = protected.len() - 1;
    protected[last] ^= 1;

    let result = decode_file(&protected, "test passphrase");

    assert!(matches!(result, Err(FileKitError::ProcessingFailed)));
}

// Modifying the FKT1 header must invalidate authentication.
#[test]
fn rejects_modified_fkt1_header() {
    let mut protected =
        encode_file(b"secret data", "test passphrase")
            .expect("protection should succeed");

    // FKT1 layout:
    // Bytes 0..4   = format marker
    // Bytes 4..8   = iteration count
    // Bytes 8..24  = salt
    // Bytes 24..36 = nonce
    //
    // Change one salt byte without changing the format marker
    // or iteration count.
    protected[8] ^= 1;

    let result = decode_file(&protected, "test passphrase");

    // The modified header must cause authentication to fail.
    assert!(matches!(
        result,
        Err(FileKitError::ProcessingFailed)
    ));
}

// Random or malformed input must not be treated as a valid file.
#[test]
fn rejects_invalid_format() {
    // Long enough to contain a header and authentication tag,
    // but it does not have the expected format marker.
    let invalid = vec![0u8; 64];

    let result = decode_file(&invalid, "test passphrase");

    assert!(matches!(result, Err(FileKitError::InvalidFormat)));
}

// A truncated FKT1 file must be rejected safely.
#[test]
fn rejects_truncated_fkt1_file() {
    // Create input that is shorter than the minimum valid FKT1 file.
    let truncated = vec![0u8; 10];

    let result = decode_file(&truncated, "test passphrase");

    // The decoder must report insufficient input, not panic.
    assert!(matches!(
        result,
        Err(FileKitError::FileTooShort)
    ));
}

// Reject empty passphrases explicitly.
#[test]
fn rejects_empty_passphrase() {
    let result = encode_file(b"some data", "");

    assert!(matches!(result, Err(FileKitError::EmptyPassphrase)));
}

// Each call should produce a different file because the salt
// and nonce are freshly generated.
#[test]
fn each_protected_file_has_fresh_random_values() {
    let first = encode_file(b"same data", "same passphrase").expect("protection should succeed");

    let second = encode_file(b"same data", "same passphrase").expect("protection should succeed");

    assert_ne!(first, second);

    assert_eq!(
        decode_file(&first, "same passphrase").unwrap(),
        b"same data"
    );

    assert_eq!(
        decode_file(&second, "same passphrase").unwrap(),
        b"same data"
    );
}

// Failed FKT2 authentication must never publish partial plaintext.
#[test]
fn failed_streaming_restore_does_not_publish_partial_output() {
    let directory = tempdir().expect("temporary directory should be created");

    let destination = directory.path().join("restored.bin");

    // Create a valid FKT2 protected stream.
    let mut protected = Vec::new();

    encode_reader(
    &b"secret data that must not be published"[..],
    &mut protected,
    "test passphrase",
    )

    .expect("stream encryption should succeed");

    // Corrupt the final authentication tag.
    // The data record remains valid, so plaintext can be written
    // to the temporary file before final authentication fails.
    let last = protected.len() - 1;
    protected[last] ^= 1;

    // Attempt to restore the protected data using safe output.
    let result = write_new_file_with_writer(&destination, |temporary| {
        decode_reader(
            protected.as_slice(),
            temporary,
            "test passphrase",
        )
        .map_err(|error| io::Error::other(error.to_string()))
    });

    // The restoration must fail.
    assert!(
        result.is_err(),
        "restoration must fail when final authentication is invalid"
    );

    // Partial plaintext must never appear at the final destination.
    assert!(
        !destination.exists(),
        "failed restoration must not publish a destination file"
    );

    // The temporary directory should contain no leftover temporary file.
    assert_eq!(
        std::fs::read_dir(directory.path())
            .expect("directory should be readable")
            .count(),
        0,
        "temporary output should be cleaned up"
    );
}