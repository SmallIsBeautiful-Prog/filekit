use thiserror::Error;

/// Errors that can occur while protecting or restoring a file.
///
/// Keeping errors in one place makes the library easier to use
/// from other Rust programs and from our future browser adapter.
#[derive(Debug, Error)]
pub enum FileKitError {
    #[error("The passphrase cannot be empty.")]
    EmptyPassphrase,

    #[error("The file is too short or incomplete.")]
    FileTooShort,

    #[error("The file format is not recognised.")]
    InvalidFormat,

    #[error("The file contains an unsupported work factor.")]
    InvalidWorkFactor,

    #[error("Secure random number generation failed.")]
    RandomGenerationFailed,

    #[error("Cryptographic processing failed.")]
    ProcessingFailed,
	
	#[error("A file input or output operation failed: {0}")]
    Io(#[from] std::io::Error),
}
