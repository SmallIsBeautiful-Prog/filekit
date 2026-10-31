mod core;
mod error;
mod safe_output;
mod stream;

// Make these functions available to other Rust projects.
pub use core::{decode_file, encode_file};

// Let callers inspect and handle errors.
pub use error::FileKitError;

// Streaming support for large files.
pub use stream::{decode_reader, encode_reader, STREAM_CHUNK_SIZE};

// Safe output helper.
pub use safe_output::{write_new_file, write_new_file_with_writer};

// Compile the JavaScript bindings only for WebAssembly.
#[cfg(target_arch = "wasm32")]
mod wasm;