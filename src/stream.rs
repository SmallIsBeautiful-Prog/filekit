use std::io::{self, Read, Write};

use aes_gcm::{
    aead::{Aead, KeyInit, Payload},
    Aes256Gcm, Nonce,
};
use getrandom::getrandom;
use pbkdf2::pbkdf2_hmac;
use sha2::Sha256;
use zeroize::Zeroizing;

use crate::error::FileKitError;

// ------------------------------------------------------------
// FKT2 streaming file format
// ------------------------------------------------------------

const MAGIC: &[u8; 4] = b"FKT2";

const DEFAULT_ITERATIONS: u32 = 600_000;
const MIN_ITERATIONS: u32 = 100_000;
const MAX_ITERATIONS: u32 = 2_000_000;

const KEY_LEN: usize = 32;
const SALT_LEN: usize = 16;
const NONCE_PREFIX_LEN: usize = 8;
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;

const HEADER_LEN: usize = 4 + 4 + SALT_LEN + NONCE_PREFIX_LEN;
const RECORD_HEADER_LEN: usize = 1 + 4;

const DATA_RECORD: u8 = 0;
const FINAL_RECORD: u8 = 1;

/// Maximum plaintext bytes processed in one data chunk: 4 MiB.
pub const STREAM_CHUNK_SIZE: usize = 4 * 1024 * 1024;

// ------------------------------------------------------------
// Key derivation
// ------------------------------------------------------------

fn derive_key(
    passphrase: &str,
    salt: &[u8],
    iterations: u32,
) -> Zeroizing<[u8; KEY_LEN]> {
    let mut key = Zeroizing::new([0u8; KEY_LEN]);

    pbkdf2_hmac::<Sha256>(
        passphrase.as_bytes(),
        salt,
        iterations,
        &mut key[..],
    );

    key
}

// ------------------------------------------------------------
// Nonce and authentication helpers
// ------------------------------------------------------------

fn make_nonce(prefix: &[u8; NONCE_PREFIX_LEN], counter: u32) -> [u8; NONCE_LEN] {
    let mut nonce = [0u8; NONCE_LEN];

    nonce[..NONCE_PREFIX_LEN].copy_from_slice(prefix);
    nonce[NONCE_PREFIX_LEN..].copy_from_slice(&counter.to_be_bytes());

    nonce
}

fn make_aad(
    header: &[u8; HEADER_LEN],
    counter: u32,
    record_header: &[u8; RECORD_HEADER_LEN],
) -> Vec<u8> {
    let mut aad = Vec::with_capacity(HEADER_LEN + 4 + RECORD_HEADER_LEN);

    aad.extend_from_slice(header);
    aad.extend_from_slice(&counter.to_be_bytes());
    aad.extend_from_slice(record_header);

    aad
}

fn next_counter(counter: u32) -> Result<u32, FileKitError> {
    counter
        .checked_add(1)
        .ok_or(FileKitError::ProcessingFailed)
}

// ------------------------------------------------------------
// Reading helpers
// ------------------------------------------------------------

/// Reads up to buffer.len() bytes, stopping at EOF.
///
/// Returns the number of bytes read. A short result means EOF
/// was reached before the buffer was filled.
fn read_chunk<R: Read>(
    reader: &mut R,
    buffer: &mut [u8],
) -> io::Result<usize> {
    let mut filled = 0;

    while filled < buffer.len() {
        match reader.read(&mut buffer[filled..]) {
            Ok(0) => break,
            Ok(count) => filled += count,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }

    Ok(filled)
}

/// Reads exactly one record header.
///
/// Returns false only if EOF occurs before any header byte.
/// EOF in the middle of a header is an incomplete file.
fn read_record_header<R: Read>(
    reader: &mut R,
    record_header: &mut [u8; RECORD_HEADER_LEN],
) -> Result<bool, FileKitError> {
    let mut first = [0u8; 1];

    loop {
        match reader.read(&mut first) {
            Ok(0) => return Ok(false),
            Ok(1) => break,
            Ok(_) => unreachable!(),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error.into()),
        }
    }

    record_header[0] = first[0];

    reader
        .read_exact(&mut record_header[1..])
        .map_err(|_| FileKitError::FileTooShort)?;

    Ok(true)
}

// ------------------------------------------------------------
// Streaming encryption
// ------------------------------------------------------------

/// Encrypts a stream of bytes using the FKT2 format.
///
/// The input is read in chunks of at most 4 MiB. The output
/// contains an authenticated header, authenticated data records,
/// and an authenticated final record.
///
/// This function does not create files on disk. The caller
/// supplies the input reader and output writer.
///
/// Important: if an error occurs, the writer may already contain
/// partial output. For disk files, use a temporary file and only
/// publish it after this function returns successfully.
pub fn encode_reader<R: Read, W: Write>(
    mut reader: R,
    mut writer: W,
    passphrase: &str,
) -> Result<(), FileKitError> {
    if passphrase.is_empty() {
        return Err(FileKitError::EmptyPassphrase);
    }

    let mut salt = [0u8; SALT_LEN];
    getrandom(&mut salt)
        .map_err(|_| FileKitError::RandomGenerationFailed)?;

    let mut nonce_prefix = [0u8; NONCE_PREFIX_LEN];
    getrandom(&mut nonce_prefix)
        .map_err(|_| FileKitError::RandomGenerationFailed)?;

    let mut header = [0u8; HEADER_LEN];

    header[..4].copy_from_slice(MAGIC);
    header[4..8].copy_from_slice(&DEFAULT_ITERATIONS.to_be_bytes());
    header[8..8 + SALT_LEN].copy_from_slice(&salt);
    header[8 + SALT_LEN..].copy_from_slice(&nonce_prefix);

    let key = derive_key(passphrase, &salt, DEFAULT_ITERATIONS);

    let cipher = Aes256Gcm::new_from_slice(&key[..])
        .map_err(|_| FileKitError::ProcessingFailed)?;

    writer.write_all(&header)?;

    let mut counter = 0u32;
    let mut buffer = Zeroizing::new(vec![0u8; STREAM_CHUNK_SIZE]);

    loop {
        let count = read_chunk(&mut reader, &mut buffer)?;

        if count == 0 {
            break;
        }

        // Counter values must never repeat under the same key
        // and nonce prefix. Reserve a later counter for the
        // authenticated final record.
        if counter == u32::MAX {
            return Err(FileKitError::ProcessingFailed);
        }

        let record_header = [
            DATA_RECORD,
            ((count as u32) >> 24) as u8,
            ((count as u32) >> 16) as u8,
            ((count as u32) >> 8) as u8,
            count as u8,
        ];

        let nonce_bytes = make_nonce(&nonce_prefix, counter);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let aad = make_aad(&header, counter, &record_header);

        let ciphertext = cipher
            .encrypt(
                nonce,
                Payload {
                    msg: &buffer[..count],
                    aad: &aad,
                },
            )
            .map_err(|_| FileKitError::ProcessingFailed)?;

        writer.write_all(&record_header)?;
        writer.write_all(&ciphertext)?;

        counter = next_counter(counter)?;

        if count < STREAM_CHUNK_SIZE {
            // We reached EOF while filling this chunk.
            break;
        }
    }

    // The final record authenticates the fact that the stream
    // ended here. Without it, truncation could go undetected.
    let final_header = [FINAL_RECORD, 0, 0, 0, 0];

    let final_nonce_bytes = make_nonce(&nonce_prefix, counter);
    let final_nonce = Nonce::from_slice(&final_nonce_bytes);

    let final_aad = make_aad(&header, counter, &final_header);

    let final_tag = cipher
        .encrypt(
            final_nonce,
            Payload {
                msg: &[],
                aad: &final_aad,
            },
        )
        .map_err(|_| FileKitError::ProcessingFailed)?;

    writer.write_all(&final_header)?;
    writer.write_all(&final_tag)?;
    writer.flush()?;

    Ok(())
}

// ------------------------------------------------------------
// Streaming decryption
// ------------------------------------------------------------

/// Decrypts and authenticates an FKT2 stream.
///
/// Each data record is authenticated before its plaintext is
/// passed to the output writer. The final record authenticates
/// the end of the file.
///
/// This function does not create files on disk. If a later
/// record fails authentication, earlier plaintext may already
/// have been written. For disk files, write to a temporary file
/// and publish it only after this function returns successfully.
pub fn decode_reader<R: Read, W: Write>(
    mut reader: R,
    mut writer: W,
    passphrase: &str,
) -> Result<(), FileKitError> {
    if passphrase.is_empty() {
        return Err(FileKitError::EmptyPassphrase);
    }

    let mut header = [0u8; HEADER_LEN];

    reader
        .read_exact(&mut header)
        .map_err(|_| FileKitError::FileTooShort)?;

    if &header[..4] != MAGIC {
        return Err(FileKitError::InvalidFormat);
    }

    let iterations = u32::from_be_bytes([
        header[4],
        header[5],
        header[6],
        header[7],
    ]);

    if !(MIN_ITERATIONS..=MAX_ITERATIONS).contains(&iterations) {
        return Err(FileKitError::InvalidWorkFactor);
    }

    let salt_start = 8;
    let salt_end = salt_start + SALT_LEN;

    let salt: &[u8; SALT_LEN] = header[salt_start..salt_end]
        .try_into()
        .map_err(|_| FileKitError::InvalidFormat)?;

    let prefix: &[u8; NONCE_PREFIX_LEN] = header[salt_end..]
        .try_into()
        .map_err(|_| FileKitError::InvalidFormat)?;

    let key = derive_key(passphrase, salt, iterations);

    let cipher = Aes256Gcm::new_from_slice(&key[..])
        .map_err(|_| FileKitError::ProcessingFailed)?;

    let mut counter = 0u32;
    let mut saw_short_chunk = false;
    let mut ciphertext = vec![0u8; STREAM_CHUNK_SIZE + TAG_LEN];

    loop {
        let mut record_header = [0u8; RECORD_HEADER_LEN];

        if !read_record_header(&mut reader, &mut record_header)? {
            return Err(FileKitError::FileTooShort);
        }

        let record_type = record_header[0];

        let length = u32::from_be_bytes([
            record_header[1],
            record_header[2],
            record_header[3],
            record_header[4],
        ]) as usize;

        let nonce_bytes = make_nonce(prefix, counter);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let aad = make_aad(&header, counter, &record_header);

        match record_type {
            DATA_RECORD => {
				// The encoder reserves the maximum counter value
                // for the authenticated final record.
                if counter == u32::MAX {
                    return Err(FileKitError::InvalidFormat);
                }
                if length == 0 || length > STREAM_CHUNK_SIZE {
                    return Err(FileKitError::InvalidFormat);
                }

                // Once a short chunk appears, the next record
                // must be the final record.
                if saw_short_chunk {
                    return Err(FileKitError::InvalidFormat);
                }

                let ciphertext_len = length + TAG_LEN;

                reader
                    .read_exact(&mut ciphertext[..ciphertext_len])
                    .map_err(|_| FileKitError::FileTooShort)?;

                // Wrap decrypted plaintext in Zeroizing so the
                // buffer is zeroized when it goes out of scope,
                // including if a later operation returns an error.
                let plaintext = Zeroizing::new(
                    cipher
                        .decrypt(
                            nonce,
                            Payload {
                                msg: &ciphertext[..ciphertext_len],
                                aad: &aad,
                            },
                        )
                        .map_err(|_| FileKitError::ProcessingFailed)?,
                );

                if plaintext.len() != length {
                    return Err(FileKitError::ProcessingFailed);
                }

                writer.write_all(&plaintext)?;

                saw_short_chunk = length < STREAM_CHUNK_SIZE;
                counter = next_counter(counter)?;
            }

            FINAL_RECORD => {
                if length != 0 {
                    return Err(FileKitError::InvalidFormat);
                }

                let mut final_tag = [0u8; TAG_LEN];

                reader
                    .read_exact(&mut final_tag)
                    .map_err(|_| FileKitError::FileTooShort)?;

                // The final plaintext should be empty. Use the
                // same wrapper for consistent buffer handling.
                let plaintext = Zeroizing::new(
                    cipher
                        .decrypt(
                            nonce,
                            Payload {
                                msg: &final_tag,
                                aad: &aad,
                            },
                        )
                        .map_err(|_| FileKitError::ProcessingFailed)?,
                );

                if !plaintext.is_empty() {
                    return Err(FileKitError::ProcessingFailed);
                }

                // A valid final record must be the actual end
                // of the input stream. Reject trailing bytes.
                let mut trailing = [0u8; 1];

                loop {
                    match reader.read(&mut trailing) {
                        Ok(0) => break,
                        Ok(_) => return Err(FileKitError::InvalidFormat),
                        Err(error)
                            if error.kind() == io::ErrorKind::Interrupted =>
                        {
                            continue;
                        }
                        Err(error) => return Err(error.into()),
                    }
                }

                writer.flush()?;
                return Ok(());
            }

            _ => return Err(FileKitError::InvalidFormat),
        }
    }
}

// ------------------------------------------------------------
// Tests
// ------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(data: &[u8]) {
        let mut protected = Vec::new();

        encode_reader(data, &mut protected, "test passphrase").unwrap();

        assert_eq!(&protected[..4], MAGIC);

        let mut restored = Vec::new();

        decode_reader(
            protected.as_slice(),
            &mut restored,
            "test passphrase",
        )
        .unwrap();

        assert_eq!(restored, data);
    }

    #[test]
    fn round_trips_empty_file() {
        round_trip(b"");
    }

    #[test]
    fn round_trips_small_file() {
        round_trip(b"FileKit streaming test");
    }

    #[test]
    fn round_trips_exact_chunk() {
        let data = vec![42u8; STREAM_CHUNK_SIZE];
        round_trip(&data);
    }

    #[test]
    fn round_trips_multiple_chunks() {
        let data = vec![73u8; STREAM_CHUNK_SIZE * 2 + 123];
        round_trip(&data);
    }

    #[test]
    fn rejects_wrong_passphrase() {
        let mut protected = Vec::new();

        encode_reader(b"secret data".as_slice(), &mut protected, "correct")
            .unwrap();

        let result = decode_reader(
            protected.as_slice(),
            Vec::new(),
            "incorrect",
        );

        assert!(result.is_err());
    }

    #[test]
    fn rejects_truncated_stream() {
        let mut protected = Vec::new();

        encode_reader(b"secret data".as_slice(), &mut protected, "passphrase")
            .unwrap();

        protected.pop();

        let result = decode_reader(
            protected.as_slice(),
            Vec::new(),
            "passphrase",
        );

        assert!(result.is_err());
    }

    #[test]
    fn rejects_modified_ciphertext() {
        let mut protected = Vec::new();

        encode_reader(b"secret data".as_slice(), &mut protected, "passphrase")
            .unwrap();

        // Header is 32 bytes; the first record header is 5 bytes.
        // The next byte begins the encrypted data.
        let ciphertext_position = HEADER_LEN + RECORD_HEADER_LEN;
        protected[ciphertext_position] ^= 1;

        let result = decode_reader(
            protected.as_slice(),
            Vec::new(),
            "passphrase",
        );

        assert!(result.is_err());
    }

    #[test]
    fn rejects_trailing_bytes_after_final_record() {
        let mut protected = Vec::new();

        encode_reader(b"data".as_slice(), &mut protected, "passphrase")
            .unwrap();

        protected.push(0);

        let result = decode_reader(
            protected.as_slice(),
            Vec::new(),
            "passphrase",
        );

        assert!(result.is_err());
    }

    #[test]
    fn rejects_empty_passphrase() {
        let result = encode_reader(b"data".as_slice(), Vec::new(), "");

        assert!(matches!(
            result,
            Err(FileKitError::EmptyPassphrase)
        ));
    }
	
	#[test]
    fn rejects_invalid_stream_magic() {
        let mut protected = Vec::new();

        encode_reader(b"test data".as_slice(), &mut protected, "passphrase")
            .unwrap();

        // Change the first byte of the FKT2 header.
        protected[0] ^= 1;

        let result = decode_reader(
            protected.as_slice(),
            Vec::new(),
            "passphrase",
        );

        assert!(matches!(
            result,
            Err(FileKitError::InvalidFormat)
        ));
    }

    #[test]
    fn rejects_unsupported_stream_work_factor() {
        let mut protected = Vec::new();

        encode_reader(b"test data".as_slice(), &mut protected, "passphrase")
            .unwrap();

        // The iteration count occupies header bytes 4 through 7.
        protected[4..8].copy_from_slice(&1u32.to_be_bytes());

        let result = decode_reader(
            protected.as_slice(),
            Vec::new(),
            "passphrase",
        );

        assert!(matches!(
            result,
            Err(FileKitError::InvalidWorkFactor)
        ));
    }

    #[test]
    fn rejects_unknown_record_type() {
        let mut protected = Vec::new();

        encode_reader(b"test data".as_slice(), &mut protected, "passphrase")
            .unwrap();

        // The first record header begins immediately after the FKT2 header.
        protected[HEADER_LEN] = 99;

        let result = decode_reader(
            protected.as_slice(),
            Vec::new(),
            "passphrase",
        );

        assert!(matches!(
            result,
            Err(FileKitError::InvalidFormat)
        ));
    }

    #[test]
    fn rejects_oversized_data_record() {
        let mut protected = Vec::new();

        encode_reader(b"test data".as_slice(), &mut protected, "passphrase")
            .unwrap();

        // Set the first data record's declared length to u32::MAX.
        let length_start = HEADER_LEN + 1;
        protected[length_start..length_start + 4]
            .copy_from_slice(&u32::MAX.to_be_bytes());

        let result = decode_reader(
            protected.as_slice(),
            Vec::new(),
            "passphrase",
        );

        assert!(matches!(
            result,
            Err(FileKitError::InvalidFormat)
        ));
    }
	
	    #[test]
    fn rejects_modified_nonce_prefix() {
        let mut protected = Vec::new();

        encode_reader(
            b"test data".as_slice(),
            &mut protected,
            "passphrase",
        )
        .unwrap();

        // The nonce prefix begins at byte 24 of the FKT2 header.
        // Change one byte without changing the encrypted records.
        protected[24] ^= 1;

        let result = decode_reader(
            protected.as_slice(),
            Vec::new(),
            "passphrase",
        );

        assert!(
            result.is_err(),
            "restoration must fail when the nonce prefix is modified"
        );
    }
}