use wasm_bindgen::prelude::*;

use crate::{decode_file, encode_file, FileKitError};

// Convert our ordinary Rust errors into browser-friendly errors.
fn to_js_error(error: FileKitError) -> JsError {
    JsError::new(&error.to_string())
}

/// JavaScript calls this function to protect a file's bytes.
///
/// The actual cryptographic work remains in the shared Rust core.
#[wasm_bindgen(js_name = encode_file)]
pub fn encode_file_wasm(file_data: &[u8], passphrase: &str) -> Result<Vec<u8>, JsError> {
    encode_file(file_data, passphrase).map_err(to_js_error)
}

/// JavaScript calls this function to restore a protected file.
///
/// Authentication failures are reported as JavaScript errors.
#[wasm_bindgen(js_name = decode_file)]
pub fn decode_file_wasm(protected_data: &[u8], passphrase: &str) -> Result<Vec<u8>, JsError> {
    decode_file(protected_data, passphrase).map_err(to_js_error)
}
