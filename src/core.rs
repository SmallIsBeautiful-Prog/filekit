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
// File format
// ------------------------------------------------------------

// The first four bytes identify this file format.
// The marker also gives us room to introduce future versions.
const MAGIC: &[u8; 4] = b"FKT1";

// PBKDF2 makes guessing a passphrase more computationally costly.
// This is a starting value; performance should be measured on
// the devices where the program will be used.
const DEFAULT_ITERATIONS: u32 = 600_000;

// The decoder accepts only work factors within these limits.
// This prevents a file header from requesting unlimited work.
const MIN_ITERATIONS: u32 = 100_000;
const MAX_ITERATIONS: u32 = 2_000_000;

// AES-256 uses a 32-byte key.
const KEY_LEN: usize = 32;

// PBKDF2 uses a random 16-byte salt.
const SALT_LEN: usize = 16;

// AES-GCM uses a 12-byte nonce with this algorithm.
const NONCE_LEN: usize = 12;

// AES-GCM appends a 16-byte authentication tag.
const TAG_LEN: usize = 16;

// Header layout:
// 4 bytes marker
// 4 bytes iteration count
// 16 bytes salt
// 12 bytes nonce
//
// Total header size: 36 bytes.
const HEADER_LEN: usize = 4 + 4 + SALT_LEN + NONCE_LEN;

// ------------------------------------------------------------
// Derive an AES key from a passphrase
// ------------------------------------------------------------

fn derive_key(passphrase: &str, salt: &[u8], iterations: u32) -> Zeroizing<[u8; KEY_LEN]> {
    // Zeroizing attempts to erase the key buffer when it is
    // dropped. It cannot guarantee erasure of every copy.
    let mut key = Zeroizing::new([0u8; KEY_LEN]);

    // The sender and receiver obtain the same key from the same
    // passphrase, salt and iteration count.
    pbkdf2_hmac::<Sha256>(passphrase.as_bytes(), salt, iterations, &mut key[..]);

    key
}

// ------------------------------------------------------------
// Protect arbitrary file bytes
// ------------------------------------------------------------

/// Protects any sequence of bytes with a passphrase.
///
/// Returns a binary file containing a header, encrypted data
/// and an authentication tag.
///
/// This function does not read or write files on disk.
pub fn encode_file(file_data: &[u8], passphrase: &str) -> Result<Vec<u8>, FileKitError> {
    if passphrase.is_empty() {
        return Err(FileKitError::EmptyPassphrase);
    }

    // Generate a new unpredictable salt for this file.
    let mut salt = [0u8; SALT_LEN];
    getrandom(&mut salt).map_err(|_| FileKitError::RandomGenerationFailed)?;

    // Generate a new nonce for this encryption operation.
    let mut nonce_bytes = [0u8; NONCE_LEN];
    getrandom(&mut nonce_bytes).map_err(|_| FileKitError::RandomGenerationFailed)?;

    // Derive the AES-256 key from the passphrase.
    let key = derive_key(passphrase, &salt, DEFAULT_ITERATIONS);

    let cipher = Aes256Gcm::new_from_slice(&key[..]).map_err(|_| FileKitError::ProcessingFailed)?;

    // Construct the header in a defined byte order.
    let mut header = Vec::with_capacity(HEADER_LEN);
    header.extend_from_slice(MAGIC);
    header.extend_from_slice(&DEFAULT_ITERATIONS.to_be_bytes());
    header.extend_from_slice(&salt);
    header.extend_from_slice(&nonce_bytes);

    let nonce = Nonce::from_slice(&nonce_bytes);

    // The header is authenticated as additional data.
    // It is not encrypted, but changing it invalidates the tag.
    let ciphertext = cipher
        .encrypt(
            nonce,
            Payload {
                msg: file_data,
                aad: &header,
            },
        )
        .map_err(|_| FileKitError::ProcessingFailed)?;

    // Store the header followed by ciphertext and its tag.
    let mut output = header;
    output.extend_from_slice(&ciphertext);

    Ok(output)
}

// ------------------------------------------------------------
// Restore the original file bytes
// ------------------------------------------------------------

/// Restores bytes from a file created by `encode_file`.
///
/// Returns an error if the format is invalid, the passphrase
/// is incorrect, or the encrypted contents have been changed.
pub fn decode_file(protected_data: &[u8], passphrase: &str) -> Result<Vec<u8>, FileKitError> {
    if passphrase.is_empty() {
        return Err(FileKitError::EmptyPassphrase);
    }

    // A valid file must contain a complete header and tag,
    // even when the original file was empty.
    if protected_data.len() < HEADER_LEN + TAG_LEN {
        return Err(FileKitError::FileTooShort);
    }

    let header = &protected_data[..HEADER_LEN];

    if &header[..4] != MAGIC {
        return Err(FileKitError::InvalidFormat);
    }

    // Convert the four stored bytes back to an iteration count.
    let iterations = u32::from_be_bytes([header[4], header[5], header[6], header[7]]);

    // Do not trust work-factor values supplied by a file.
    if !(MIN_ITERATIONS..=MAX_ITERATIONS).contains(&iterations) {
        return Err(FileKitError::InvalidWorkFactor);
    }

    let salt_start = 8;
    let salt_end = salt_start + SALT_LEN;
    let salt = &header[salt_start..salt_end];

    let nonce_end = salt_end + NONCE_LEN;
    let nonce_bytes = &header[salt_end..nonce_end];

    let key = derive_key(passphrase, salt, iterations);

    let cipher = Aes256Gcm::new_from_slice(&key[..]).map_err(|_| FileKitError::ProcessingFailed)?;

    let nonce = Nonce::from_slice(nonce_bytes);
    let ciphertext = &protected_data[HEADER_LEN..];

    // AES-GCM checks authenticity before returning plaintext.
    // We deliberately use the same error for a wrong key and
    // corrupted ciphertext, avoiding unnecessary disclosure.
    let mut plaintext = Zeroizing::new(
    cipher
        .decrypt(
            nonce,
            Payload {
                msg: ciphertext,
                aad: header,
            },
        )
        .map_err(|_| FileKitError::ProcessingFailed)?,
    );

    // Return the plaintext to the caller.
    //
    // The caller owns the returned bytes, so this function cannot
    // zeroize them after returning. We move the Vec out of the
    // wrapper so the caller receives the same public return type.
    Ok(std::mem::take(&mut *plaintext))
}
