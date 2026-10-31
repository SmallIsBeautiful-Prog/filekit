# FileKit

**A reusable Rust library for protecting and restoring files.**

FileKit is a Rust library that provides authenticated file encryption, streaming support for large files, safe output helpers, command-line programs, and a WebAssembly interface for browser applications.

The library is designed to be reusable: other Rust applications can call its functions directly, while JavaScript applications can use the WebAssembly interface.

## Contents

- [Features](#features)
- [How FileKit works](#how-filekit-works)
- [Getting started](#getting-started)
- [Using FileKit from Rust](#using-filekit-from-rust)
- [Rust API reference](#rust-api-reference)
- [Command-line programs](#command-line-programs)
- [Using FileKit in a browser](#using-filekit-in-a-browser)
- [Running the browser interface locally](#running-the-browser-interface-locally)
- [WebAssembly API reference](#webassembly-api-reference)
- [JavaScript helper API reference](#javascript-helper-api-reference)
- [Error handling](#error-handling)
- [Security considerations](#security-considerations)
- [Testing](#testing)
- [Project structure](#project-structure)
- [Dependencies](#dependencies)
- [Contributing](#contributing)
- [Licence](#licence)

---

## Features

- **Reusable Rust library:** Call FileKit from your own Rust programs.
- **Authenticated encryption:** Uses AES-256-GCM to protect data and detect unauthorised modifications.
- **Passphrase-based encryption:** Derives encryption keys using PBKDF2-HMAC-SHA-256.
- **Two file formats:** FKT1 for whole-file processing and FKT2 for streaming.
- **Streaming support:** Processes large inputs in chunks instead of loading the entire file into memory.
- **Safe output helpers:** Publish completed files without overwriting an existing destination.
- **Native command-line programs:** Protect and restore files from a terminal.
- **WebAssembly interface:** Call the shared cryptographic implementation from JavaScript.
- **Browser helpers:** Process browser `File` and `Blob` objects and initiate downloads.
- **Binary-safe:** Designed to handle arbitrary bytes, not just text.

## How FileKit works

FileKit separates the cryptographic implementation from the interfaces used to access it.

```text
                         FileKit
                            |
                 Shared Rust library
                            |
               +------------+------------+
               |                         |
        Whole-file API              Streaming API
            FKT1                        FKT2
               |                         |
       +-------+-------+                 |
       |               |                 |
    Rust apps      WebAssembly       Native Rust apps
                       |
                   JavaScript
                       |
                Browser applications
```

The native command-line programs use the FKT2 streaming API when protecting files. They can restore both FKT1 and FKT2 files.

The current WebAssembly interface uses the FKT1 whole-file API. It does not currently expose the FKT2 streaming functions.

### Which API should I use?

| Situation | Recommended API |
|---|---|
| Protecting bytes already held in memory | `encode_file` |
| Restoring an FKT1 file held in memory | `decode_file` |
| Protecting a large input stream | `encode_reader` |
| Restoring an FKT2 input stream | `decode_reader` |
| Saving a completed byte buffer to a new file | `write_new_file` |
| Saving the result of a streaming operation | `write_new_file_with_writer` |
| Protecting or restoring a browser file | `encodeFile` or `decodeFile` |
| Downloading a result from a browser | `downloadBlob` |

**Important:** The two formats have different processing requirements. FKT1 requires the entire input in memory, whereas FKT2 processes data in chunks.

---

## Getting started

### Prerequisites

For native Rust development, install Rust and Cargo using the official Rust installation instructions.

For WebAssembly development, you will also need a WebAssembly-compatible Rust target and `wasm-pack`.

- Rust installation: https://www.rust-lang.org/tools/install
- Cargo documentation: https://doc.rust-lang.org/cargo/
- wasm-pack: https://rustwasm.github.io/wasm-pack/

### 1. Get the project

Clone the FileKit repository from GitHub:

```bash
git clone https://github.com/SmallIsBeautiful-Prog/filekit.git
cd filekit
```

Alternatively, download the repository and open a terminal in the project directory.

### 2. Build the library

Run:

```bash
cargo build
```

For an optimised release build:

```bash
cargo build --release
```

### 3. Run the tests

```bash
cargo test
```

The tests cover encryption and restoration, invalid input, incorrect passphrases, tampering, streaming behaviour, and safe output handling.

### 4. Choose an interface

You can now:

- Use FileKit directly from a Rust program.
- Run the native command-line programs.
- Build the WebAssembly package for a browser application.

The following sections explain each option.

---

# Using FileKit from Rust

## Add FileKit to your project

If FileKit is published on crates.io, add it to your project's `Cargo.toml`:

```toml
[dependencies]
filekit = "0.1"
```

If you are developing against a local copy of FileKit, use a path dependency instead:

```toml
[dependencies]
filekit = { path = "../filekit" }
```

Change the path to match your directory structure.

## Example: Protect and restore data

This example protects a byte buffer, restores it, and checks that the original data is recovered.

Create a Rust binary or add the following code to an existing application:

```rust
use filekit::{decode_file, encode_file};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let original = b"Hello from FileKit!";
    let passphrase = "replace-with-a-strong-passphrase";

    // Protect the original bytes.
    let protected = encode_file(original, passphrase)?;

    // Restore the original bytes.
    let restored = decode_file(&protected, passphrase)?;

    assert_eq!(restored.as_slice(), original);

    println!("The data was restored successfully.");

    Ok(())
}
```

Run the application using:

```bash
cargo run
```

### What happens?

1. `encode_file` receives the original bytes and passphrase.
2. FileKit generates a random salt and nonce and derives an encryption key.
3. The function returns the protected data as a `Vec<u8>`.
4. `decode_file` authenticates and restores the protected bytes.
5. The example compares the restored bytes with the original.

The returned protected data is binary. Do not convert it to a UTF-8 string or save it as ordinary text.

**Passphrase note:** The hard-coded passphrase is for demonstrating the API only. Real applications should obtain passphrases through an appropriate secure input mechanism and avoid embedding them in source code.

## Example: Protect and save a file

The following example reads a file into memory, protects it, and writes the result to a new destination.

```rust
use filekit::{encode_file, write_new_file};
use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input_path = Path::new("example.txt");
    let output_path = Path::new("example.txt.fkt");
    let passphrase = "replace-with-a-strong-passphrase";

    let original = fs::read(input_path)?;
    let protected = encode_file(&original, passphrase)?;

    // Fails rather than replacing an existing destination.
    write_new_file(output_path, &protected)?;

    println!("Protected file saved.");

    Ok(())
}
```

This example uses the whole-file API, so the original data and protected result are held in memory.

For large files, see the streaming example below.

---

# Rust API reference

The following functions are publicly exported by the Rust library.

## 1. `encode_file`

**Purpose:** Protect a byte buffer using the FKT1 format.

### Signature

```rust
pub fn encode_file(
    file_data: &[u8],
    passphrase: &str,
) -> Result<Vec<u8>, FileKitError>
```

### Parameters

| Parameter | Type | Description |
|---|---|---|
| `file_data` | `&[u8]` | The original bytes to protect |
| `passphrase` | `&str` | The passphrase used to derive the encryption key |

### Return value

On success, returns a `Vec<u8>` containing the complete FKT1 protected file.

The result contains the header followed by the encrypted data and authentication tag.

### Errors

The function can return:

- `FileKitError::EmptyPassphrase` if the passphrase is empty.
- `FileKitError::RandomGenerationFailed` if secure random generation fails.
- `FileKitError::ProcessingFailed` if cryptographic processing fails.

### Example

```rust
use filekit::encode_file;

let protected = encode_file(b"Example data", "a strong passphrase")?;
```

### When should I use it?

Use `encode_file` when the complete input is already available in memory and you want a straightforward API.

It does not read or write files on disk.

**Memory consideration:** The input and returned protected data can both occupy memory. Use `encode_reader` for large inputs.

---

## 2. `decode_file`

**Purpose:** Authenticate and restore an FKT1 protected file.

### Signature

```rust
pub fn decode_file(
    protected_data: &[u8],
    passphrase: &str,
) -> Result<Vec<u8>, FileKitError>
```

### Parameters

| Parameter | Type | Description |
|---|---|---|
| `protected_data` | `&[u8]` | The complete FKT1 protected file |
| `passphrase` | `&str` | The passphrase used during protection |

### Return value

On success, returns the restored bytes as a `Vec<u8>`.

### Errors

The function can return:

- `EmptyPassphrase` for an empty passphrase.
- `FileTooShort` if the input is too short to contain the required header and authentication tag.
- `InvalidFormat` if the format marker is not recognised.
- `InvalidWorkFactor` if the stored PBKDF2 iteration count is outside the accepted range.
- `ProcessingFailed` if authentication or cryptographic processing fails.

The implementation deliberately uses the same cryptographic processing error for an incorrect passphrase and modified ciphertext.

### Example

```rust
use filekit::decode_file;

let restored = decode_file(&protected, "a strong passphrase")?;
```

### Important behaviour

FileKit authenticates the encrypted data before returning the restored bytes.

However, the returned plaintext belongs to the caller. The caller is responsible for managing its lifetime and any sensitive copies.

### When should I use it?

Use `decode_file` when restoring an FKT1 buffer held in memory.

It is also the current whole-file decoder used by the WebAssembly interface.

---

## 3. `encode_reader`

**Purpose:** Protect an input stream using the FKT2 streaming format.

### Signature

```rust
pub fn encode_reader<R: Read, W: Write>(
    reader: R,
    writer: W,
    passphrase: &str,
) -> Result<(), FileKitError>
```

The actual implementation takes mutable reader and writer parameters internally, but callers pass values implementing the relevant traits.

### Parameters

| Parameter | Requirement | Description |
|---|---|---|
| `reader` | Implements `std::io::Read` | Supplies the original bytes |
| `writer` | Implements `std::io::Write` | Receives the protected bytes |
| `passphrase` | `&str` | Passphrase used to derive the encryption key |

### Return value

Returns `Ok(())` when the stream has been protected successfully and the output writer has been flushed.

### How streaming works

FileKit processes the input in chunks of up to 4 MiB.

The FKT2 stream contains:

- An authenticated-format header.
- Authenticated data records.
- An authenticated final record.

The final record allows the decoder to detect a stream that has been truncated before completion.

### Example: Protect a file stream

```rust
use filekit::encode_reader;
use std::fs::File;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = File::open("large-file.bin")?;
    let mut output = File::create("large-file.bin.fkt")?;

    encode_reader(&mut input, &mut output, "a strong passphrase")?;

    Ok(())
}
```

This demonstrates the streaming API, but it writes directly to the destination. If encryption fails, the destination may contain incomplete output.

For disk files, prefer the safe-output example below.

### Important limitation

`encode_reader` does not create or publish files itself. The caller chooses the reader and writer.

If the operation fails, the writer may already contain partial output. Use `write_new_file_with_writer` when writing a new disk file that should only become visible after successful completion.

---

## 4. `decode_reader`

**Purpose:** Restore and authenticate an FKT2 input stream.

### Signature

```rust
pub fn decode_reader<R: Read, W: Write>(
    reader: R,
    writer: W,
    passphrase: &str,
) -> Result<(), FileKitError>
```

### Parameters

| Parameter | Requirement | Description |
|---|---|---|
| `reader` | Implements `std::io::Read` | Supplies the complete FKT2 stream |
| `writer` | Implements `std::io::Write` | Receives the restored bytes |
| `passphrase` | `&str` | The original passphrase |

### Return value

Returns `Ok(())` only after the data records and final record have been authenticated, the end of the stream has been verified, and the output writer has been flushed.

### Errors

Errors include invalid format, an unsupported work factor, truncated input, authentication failures, and input/output failures.

### Example: Restore a file stream

```rust
use filekit::decode_reader;
use std::fs::File;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = File::open("large-file.bin.fkt")?;
    let mut output = File::create("large-file-restored.bin")?;

    decode_reader(&mut input, &mut output, "a strong passphrase")?;

    Ok(())
}
```

As with the encryption example, this writes directly to the destination and is not the recommended pattern for publishing restored files safely.

### Important security consideration

Each data record is authenticated before its plaintext is written. However, a later record may fail authentication after earlier records have already been written.

**Never assume the output is complete merely because some bytes have been written.** Only treat the restored file as successful after `decode_reader` returns `Ok(())`.

For disk output, use `write_new_file_with_writer` so an unsuccessful restoration does not publish a final destination file.

---

## 5. `write_new_file`

**Purpose:** Write a byte buffer to a new file without overwriting an existing destination.

### Signature

```rust
pub fn write_new_file(
    destination: &Path,
    data: &[u8],
) -> std::io::Result<()>
```

### Parameters

| Parameter | Type | Description |
|---|---|---|
| `destination` | `&Path` | Path where the completed file should be published |
| `data` | `&[u8]` | Bytes to write |

### Behaviour

FileKit:

1. Creates a temporary file in the destination directory.
2. Writes the supplied bytes.
3. Synchronises the temporary file.
4. Publishes the completed file using a hard link.
5. Removes the temporary filename when the temporary-file object is dropped.

If the destination already exists, publication fails rather than replacing it.

If writing or publication fails, the helper returns an I/O error.

### Example

```rust
use filekit::write_new_file;
use std::path::Path;

fn main() -> std::io::Result<()> {
    write_new_file(
        Path::new("output.bin"),
        b"Completed data",
    )?;

    Ok(())
}
```

### Important considerations

- The destination's parent directory must already exist.
- The helper does not create missing parent directories.
- The filesystem must support the hard-link operation used for publication.
- The helper is intended for native filesystem use, not browser downloads.

---

## 6. `write_new_file_with_writer`

**Purpose:** Write data through a supplied operation and publish the resulting file only if that operation succeeds.

This is the recommended helper for safely publishing streaming encryption or restoration results to a new native file.

### Signature

```rust
pub fn write_new_file_with_writer<F>(
    destination: &Path,
    write_data: F,
) -> std::io::Result<()>
where
    F: FnOnce(&mut tempfile::NamedTempFile) -> std::io::Result<()>;
```

### Parameters

| Parameter | Description |
|---|---|
| `destination` | Destination path for the completed file |
| `write_data` | Closure that writes to the temporary file and returns an I/O result |

### Example: Safely protect a large file

```rust
use filekit::{encode_reader, write_new_file_with_writer};
use std::fs::File;
use std::io;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = File::open("large-file.bin")?;
    let destination = Path::new("large-file.bin.fkt");
    let passphrase = "a strong passphrase";

    write_new_file_with_writer(destination, |temporary| {
        encode_reader(&mut input, temporary, passphrase)
            .map_err(|error| io::Error::other(error.to_string()))
    })?;

    println!("Protection completed.");

    Ok(())
}
```

### Example: Safely restore a large file

```rust
use filekit::{decode_reader, write_new_file_with_writer};
use std::fs::File;
use std::io;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = File::open("large-file.bin.fkt")?;
    let destination = Path::new("large-file-restored.bin");
    let passphrase = "a strong passphrase";

    write_new_file_with_writer(destination, |temporary| {
        decode_reader(&mut input, temporary, passphrase)
            .map_err(|error| io::Error::other(error.to_string()))
    })?;

    println!("Restoration completed.");

    Ok(())
}
```

### Why this matters

The closure writes into a temporary file rather than directly into the final destination.

If the streaming operation fails, the helper does not publish the destination. If the operation succeeds, it synchronises and publishes the completed temporary file.

The helper does not overwrite an existing destination.

**Do not replace this pattern with direct writes to the final restored-file path if incomplete output must not be published.**

---

## 7. `FileKitError`

**Purpose:** Represent errors produced by FileKit's whole-file and streaming cryptographic APIs.

### Import

```rust
use filekit::FileKitError;
```

### Variants

| Variant | Meaning |
|---|---|
| `EmptyPassphrase` | An empty passphrase was supplied |
| `FileTooShort` | The input is too short or incomplete |
| `InvalidFormat` | The file format or stream structure is invalid |
| `InvalidWorkFactor` | The stored key-derivation work factor is outside the accepted range |
| `RandomGenerationFailed` | Secure random generation failed |
| `ProcessingFailed` | Cryptographic processing or authentication failed |
| `Io(std::io::Error)` | An input/output operation failed |

The `Io` variant supports conversion from `std::io::Error`.

Note that `write_new_file` and `write_new_file_with_writer` return `std::io::Result<()>` directly. They do not return `FileKitError`.

### Example: Handle an error

```rust
use filekit::{encode_file, FileKitError};

fn main() {
    match encode_file(b"example", "") {
        Ok(protected) => {
            println!("Protected {} bytes.", protected.len());
        }
        Err(FileKitError::EmptyPassphrase) => {
            eprintln!("Please supply a non-empty passphrase.");
        }
        Err(error) => {
            eprintln!("FileKit operation failed: {error}");
        }
    }
}
```

Applications should avoid exposing unnecessary cryptographic details to untrusted users.

---

## 8. `STREAM_CHUNK_SIZE`

**Purpose:** Expose the maximum plaintext size processed in one FKT2 data chunk.

### Definition

```rust
pub const STREAM_CHUNK_SIZE: usize = 4 * 1024 * 1024;
```

This equals **4 MiB**, or 4,194,304 bytes.

### Example

```rust
use filekit::STREAM_CHUNK_SIZE;

fn main() {
    println!(
        "Maximum plaintext chunk: {} bytes",
        STREAM_CHUNK_SIZE
    );
}
```

This constant is useful when inspecting or describing the streaming behaviour. Most callers do not need to use it to operate the streaming functions.

---

# Command-line programs

FileKit includes two native command-line programs:

- `encode`: protects a file using FKT2 streaming encryption.
- `decode`: restores an FKT1 or FKT2 file.

They are convenient examples of using the library from a native application.

## Protect a file

From the FileKit project directory:

```bash
cargo run --release --bin encode -- "example.txt"
```

The program:

1. Checks that the input path identifies a file.
2. Uses the input filename with `.fkt` appended as the output filename.
3. Refuses to proceed if the output already exists.
4. Prompts for a passphrase without echoing it to the terminal.
5. Encrypts the file using FKT2 streaming encryption.
6. Publishes the completed result only if encryption succeeds.

For example, protecting `example.txt` produces `example.txt.fkt`.

## Restore a file

```bash
cargo run --release --bin decode -- "example.txt.fkt"
```

The program prompts for the passphrase and checks the file's format marker.

For an FKT1 file, it uses the whole-file decoder. For an FKT2 file, it uses the streaming decoder.

When the input filename ends in `.fkt`, the default output filename is formed by removing that suffix. Otherwise, `.restored` is appended.

The program refuses to overwrite an existing output file.

### Important notes

- Keep your passphrase safe. Losing it may mean losing access to the protected data.
- Do not use the same destination for unrelated files unless you have checked the output path.
- FKT1 restoration loads the protected file into memory. FKT2 restoration uses streaming.
- The command-line programs are native applications. The browser interface is documented separately below.

---

# Using FileKit in a browser

FileKit provides a WebAssembly interface for JavaScript applications.

The WebAssembly functions use the same Rust cryptographic implementation as the native library's FKT1 functions.

The supplied JavaScript helpers make it easier to use the WebAssembly interface with browser files and downloads.

## Browser architecture

```text
Browser file input
       |
       v
JavaScript File or Blob
       |
       v
encodeFile() / decodeFile()
       |
       v
Uint8Array
       |
       v
WebAssembly function
       |
       v
Rust cryptographic implementation
       |
       v
Uint8Array
       |
       v
JavaScript Blob
       |
       v
downloadBlob()
```

The browser helpers do not write directly to arbitrary filesystem paths. They initiate a browser download, and the browser controls where the user saves the file.

## 1. Prepare the WebAssembly build environment

Install `wasm-pack` and the WebAssembly target if they are not already installed.

A typical setup is:

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
```

These commands prepare the tools; they do not modify the cryptographic implementation.

## 2. Build the WebAssembly package

From the FileKit project root, run:

```bash
wasm-pack build --target web --out-dir web/pkg
```

This generates the JavaScript bindings and WebAssembly binary used by the browser interface.

The JavaScript helpers in `web/encode.js` and `web/decode.js` import the generated module from:

```text
web/pkg/filekit.js
```

The generated package should also contain the associated WebAssembly binary and declaration files.

If the build generates files in a different location, adjust the output directory or the import paths so they agree.

If you are using the generated `web/pkg/` files included in the repository and do not need to rebuild the Rust/WebAssembly code, you can skip this build step.

## Running the browser interface locally

The supplied browser interface uses JavaScript modules and WebAssembly. Browsers generally require these files to be served over HTTP, so opening `web/index.html` directly from your file manager may not work.

Run one of the following commands from the **FileKit project root**. Each command serves the contents of the `web/` directory as the website root, so you can open the interface directly at `http://localhost:8000/`.

### Option 1: Python

If Python is installed, run:

```bash
python -m http.server 8000 --directory web
```

On Windows, if `python` is not recognised, try:

```powershell
py -m http.server 8000 --directory web
```

### Option 2: Node.js with `npx`

If Node.js is installed, you can run the lightweight `http-server` package through `npx` without installing it globally:

```powershell
npx.cmd http-server web -p 8000
```

On macOS or Linux, use `npx` instead of `npx.cmd`:

```bash
npx http-server web -p 8000
```

The first run may download the `http-server` package if it is not already available. On Windows PowerShell, `npx.cmd` can avoid execution-policy issues affecting the `npx.ps1` script.

### Open FileKit

Once the server is running, open this address in your browser:

```text
http://localhost:8000/
```

To stop the server, return to the terminal and press `Ctrl+C`.

These tools are only needed to run a local development server. FileKit itself does not require Python or Node.js as application dependencies.

## 3. Use the JavaScript helpers

The easiest starting point is to import the provided helpers rather than calling the raw WebAssembly functions yourself.

For example:

```javascript
import { encodeFile, downloadBlob } from "./encode.js";
```

To restore a file:

```javascript
import { decodeFile, downloadBlob } from "./decode.js";
```

The helper functions initialise WebAssembly on their first use and reuse the initialisation promise for subsequent calls within the same module.

You do not need to call the generated `init()` function yourself when using these helpers.

## 4. Minimal browser example: Protect a file

The following example assumes your HTML contains:

- A file input with `id="file-input"`.
- A passphrase input with `id="passphrase"`.
- A button with `id="protect-button"`.

```html
<input type="file" id="file-input">
<input type="password" id="passphrase">
<button id="protect-button">Protect file</button>
```

Create a JavaScript module, for example `app.js`:

```javascript
import { encodeFile, downloadBlob } from "./encode.js";

const fileInput = document.querySelector("#file-input");
const passphraseInput = document.querySelector("#passphrase");
const protectButton = document.querySelector("#protect-button");

protectButton.addEventListener("click", async () => {
    const file = fileInput.files[0];
    const passphrase = passphraseInput.value;

    try {
        const protectedBlob = await encodeFile(file, passphrase);

        downloadBlob(
            protectedBlob,
            `${file.name}.fkt`
        );
    } catch (error) {
        console.error("Protection failed:", error);
        alert(`Protection failed: ${error.message}`);
    }
});
```

Include the module in your HTML:

```html
<script type="module" src="./app.js"></script>
```

Serve the application through a local development server or web server configured to serve WebAssembly correctly. Avoid assuming that every browser will load WebAssembly modules from a `file://` URL.

## 5. Minimal browser example: Restore a file

Use a file input, passphrase input, and restore button:

```html
<input type="file" id="file-input">
<input type="password" id="passphrase">
<button id="restore-button">Restore file</button>
```

Create a JavaScript module:

```javascript
import { decodeFile, downloadBlob } from "./decode.js";

const fileInput = document.querySelector("#file-input");
const passphraseInput = document.querySelector("#passphrase");
const restoreButton = document.querySelector("#restore-button");

restoreButton.addEventListener("click", async () => {
    const file = fileInput.files[0];
    const passphrase = passphraseInput.value;

    try {
        const restoredBlob = await decodeFile(file, passphrase);

        const outputName = file.name.endsWith(".fkt")
            ? file.name.slice(0, -4)
            : `${file.name}.restored`;

        downloadBlob(restoredBlob, outputName);
    } catch (error) {
        console.error("Restoration failed:", error);
        alert(`Restoration failed: ${error.message}`);
    }
});
```

This example uses the same filename convention as the native decoder for the `.fkt` suffix.

For a production application, improve the user interface by disabling buttons while an operation is running, clearing passphrase fields when appropriate, and displaying accessible status messages.

## Browser memory considerations

The JavaScript helpers read the complete input into an `ArrayBuffer`, convert it to a `Uint8Array`, and call the whole-file WebAssembly function.

Consequently, the input, protected or restored output, and intermediate buffers can require substantial memory for large files.

The current WebAssembly interface does **not** provide the native FKT2 streaming API. For very large files, use the native streaming interface or extend the browser interface with a suitable streaming design.

---

# WebAssembly API reference

The raw WebAssembly bindings are generated from `src/wasm.rs`.

They expose two functions to JavaScript.

## 1. `encode_file`

### JavaScript signature

```typescript
export function encode_file(
    file_data: Uint8Array,
    passphrase: string
): Uint8Array;
```

### Purpose

Protects the supplied bytes using the shared Rust FKT1 implementation.

### Parameters

| Parameter | Type | Description |
|---|---|---|
| `file_data` | `Uint8Array` | Original bytes to protect |
| `passphrase` | `string` | Passphrase used to derive the encryption key |

### Return value

Returns the protected bytes as a `Uint8Array`.

### Example

```javascript
import init, { encode_file } from "./pkg/filekit.js";

await init();

const original = new TextEncoder().encode("Hello from FileKit!");
const protectedData = encode_file(original, "a strong passphrase");
```

The example encodes text into bytes for demonstration. FileKit itself accepts arbitrary binary data and does not require text encoding.

### Errors

The function reports Rust-side failures as JavaScript errors, including an empty passphrase, random-generation failure, and cryptographic processing failure.

---

## 2. `decode_file`

### JavaScript signature

```typescript
export function decode_file(
    protected_data: Uint8Array,
    passphrase: string
): Uint8Array;
```

### Purpose

Authenticates and restores an FKT1 protected buffer.

### Parameters

| Parameter | Type | Description |
|---|---|---|
| `protected_data` | `Uint8Array` | Complete FKT1 protected file |
| `passphrase` | `string` | Original passphrase |

### Return value

Returns the restored bytes as a `Uint8Array`.

### Example

```javascript
import init, { decode_file } from "./pkg/filekit.js";

await init();

const restoredData = decode_file(
    protectedData,
    "a strong passphrase"
);
```

The example assumes `protectedData` contains the complete protected file as a `Uint8Array`.

### Errors

An invalid file, an unsupported work factor, an incorrect passphrase, or modified encrypted data can cause the function to throw a JavaScript error.

### Direct WebAssembly usage

When calling the raw WebAssembly functions directly, initialise the module first.

The supplied JavaScript helpers perform that initialisation automatically, which is generally more convenient for application code.

The generated `.d.ts` file describes the JavaScript-visible bindings. Internal WebAssembly memory functions in that file are implementation details, not intended application APIs.

---

# JavaScript helper API reference

The JavaScript helper modules are designed to make the raw WebAssembly functions easier to use.

They are separate from the raw WebAssembly exports.

## 1. `encodeFile`

**Module:** `web/encode.js`

### Signature

```typescript
export async function encodeFile(
    file: File | Blob,
    passphrase: string
): Promise<Blob>;
```

### Purpose

Reads a browser `File` or `Blob`, passes its bytes to the WebAssembly encoder, and returns the protected result as a binary `Blob`.

### Parameters

| Parameter | Type | Description |
|---|---|---|
| `file` | `File` or `Blob` | Original file data |
| `passphrase` | `string` | Passphrase used for protection |

### Return value

A promise that resolves to a `Blob` containing the protected FKT1 data.

### Errors

The helper rejects if no file is supplied, the passphrase is empty, WebAssembly initialisation fails, or the underlying encoder reports an error.

### Example

```javascript
import { encodeFile } from "./encode.js";

const protectedBlob = await encodeFile(file, passphrase);
```

The result can be passed to `downloadBlob`.

---

## 2. `decodeFile`

**Module:** `web/decode.js`

### Signature

```typescript
export async function decodeFile(
    file: File | Blob,
    passphrase: string
): Promise<Blob>;
```

### Purpose

Reads a browser `File` or `Blob`, passes its bytes to the WebAssembly decoder, and returns the restored result as a binary `Blob`.

### Parameters

| Parameter | Type | Description |
|---|---|---|
| `file` | `File` or `Blob` | Complete protected file |
| `passphrase` | `string` | Original passphrase |

### Return value

A promise that resolves to a `Blob` containing the restored bytes.

### Errors

The helper rejects if the input is missing, the passphrase is empty, WebAssembly initialisation fails, or restoration fails.

### Example

```javascript
import { decodeFile } from "./decode.js";

const restoredBlob = await decodeFile(file, passphrase);
```

The function does not decide the output filename. The calling application should choose a suitable filename.

---

## 3. `downloadBlob`

**Module:** Both `web/encode.js` and `web/decode.js`.

### Signature

```typescript
export function downloadBlob(
    blob: Blob,
    filename: string
): void;
```

### Purpose

Initiates a browser download for a `Blob`.

The helper creates a temporary object URL, creates a hidden download link, clicks it, removes the link, and schedules the temporary URL for release.

### Parameters

| Parameter | Type | Description |
|---|---|---|
| `blob` | `Blob` | Data to offer for download |
| `filename` | `string` | Suggested download filename |

### Return value

The function returns no value.

### Validation

The helper checks that:

- The input is a `Blob`.
- The filename is a non-empty string.
- The filename does not contain the characters rejected by its validation expression, including path separators and common filename-reserved characters.

### Example

```javascript
import { downloadBlob } from "./encode.js";

downloadBlob(protectedBlob, "example.txt.fkt");
```

### Important browser behaviour

The helper does not save files directly to arbitrary filesystem paths. The browser controls the actual save location and may apply its own download settings.

The function is a download helper, not a general filesystem-writing API.

---

# Error handling

Applications should handle failures explicitly rather than assuming that an operation will always succeed.

## Rust

The whole-file and streaming functions return `Result` values.

Use `?` to propagate errors where appropriate, or match on the error when the application needs to respond differently to particular cases.

For example:

```rust
match filekit::decode_file(protected_data, passphrase) {
    Ok(restored) => {
        println!("Restoration succeeded.");
        // Use the restored bytes.
        drop(restored);
    }
    Err(error) => {
        eprintln!("Restoration failed: {error}");
    }
}
```

The example drops the restored bytes because it does not need to use them further.

## JavaScript

The raw WebAssembly exports can throw JavaScript errors. The asynchronous helper functions can also reject their promises.

Use `try`/`catch` when calling the helpers:

```javascript
try {
    const protectedBlob = await encodeFile(file, passphrase);
    downloadBlob(protectedBlob, "output.fkt");
} catch (error) {
    console.error("FileKit operation failed:", error);
}
```

Avoid displaying sensitive implementation details to users. A user-facing message can explain that the operation failed without exposing unnecessary internal information.

---

# Security considerations

FileKit uses established cryptographic building blocks, but applications still need to handle passphrases, files, and errors carefully.

## Encryption

The current implementation uses:

- **AES-256-GCM** for authenticated encryption.
- **PBKDF2-HMAC-SHA-256** for deriving an encryption key from a passphrase.
- Random salts and nonces generated through the configured random-number generator.

The default PBKDF2 work factor is 600,000 iterations. The decoders accept stored work factors in the range from 100,000 to 2,000,000 iterations.

The formats store the iteration count in their headers. FKT1 uses a 36-byte header, while FKT2 uses a 32-byte header.

## Authentication and tampering

The authenticated encryption detects modifications to protected data.

The FKT2 format also uses an authenticated final record so that a decoder can detect truncation and verify the actual end of the stream.

Applications should not treat restored data as a successfully completed file until the relevant decoding function returns successfully.

## Passphrase management

The passphrase is essential to restoring protected data.

- Choose a strong, unique passphrase.
- Do not hard-code real passphrases into application source code.
- Do not log passphrases.
- Avoid unnecessarily retaining passphrases in memory.
- Make sure users understand the consequences of losing a passphrase.

FileKit does not provide a passphrase recovery service.

## Memory handling

Some key and plaintext buffers are wrapped in `Zeroizing` so their contents are cleared when those particular buffers are dropped.

This is a useful precaution, but it is **not a guarantee that every copy of sensitive data is erased**.

For example, the whole-file decoder returns a normal `Vec<u8>`, and the caller owns that returned buffer. Applications should consider the lifetime of restored data and avoid unnecessary copies.

## Safe output

For native applications, prefer `write_new_file` or `write_new_file_with_writer` when writing a new destination that must not overwrite an existing file.

For streaming restoration, use `write_new_file_with_writer` so that a partially restored result is not published as the final destination if decoding fails.

The helper relies on temporary files and hard-link publication. Filesystem behaviour and permissions still matter, and applications should handle any resulting I/O errors.

## Browser limitations

The browser helpers use the whole-file API, so they may require substantial memory for large files.

The browser download helper also cannot guarantee a particular save location. The browser controls where the file is saved.

## Scope of security claims

FileKit is a file-protection library, not a complete security system.

The security of an application also depends on passphrase strength, the operating environment, how data is handled in memory, how outputs are managed, and how the application is deployed.

A successful test run or dependency audit is useful evidence, but neither proves that a cryptographic implementation or application is free of all vulnerabilities.

---

# Testing

Run the full test suite from the project directory:

```bash
cargo test
```

The current tests cover areas including:

- Whole-file round trips.
- Binary and empty inputs.
- Incorrect passphrases.
- Invalid formats and truncated files.
- Modified headers and ciphertext.
- Streaming round trips.
- Streaming truncation and trailing-byte rejection.
- Invalid streaming record types and lengths.
- Safe output behaviour and refusal to overwrite existing destinations.

Cryptographic tests can take longer than ordinary unit tests because the configured passphrase derivation intentionally performs substantial work.

If you change the cryptographic implementation, file format, or public API, add or update tests before relying on the change.

---

# Project structure

The main source files are organised as follows:

```text
filekit/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── LICENSE-APACHE
├── LICENSE-MIT
├── src/
│   ├── lib.rs
│   ├── core.rs
│   ├── error.rs
│   ├── safe_output.rs
│   ├── stream.rs
│   ├── wasm.rs
│   └── bin/
│       ├── encode.rs
│       └── decode.rs
├── tests/
│   └── round_trip.rs
└── web/
    ├── index.html
    ├── encode.js
    ├── decode.js
    └── pkg/
        └── ...
```

The `web/pkg/` directory contains generated WebAssembly package files. Its exact contents depend on the build output.

### Main components

| File | Responsibility |
|---|---|
| `src/lib.rs` | Public Rust API and module exports |
| `src/core.rs` | FKT1 whole-file encryption and restoration |
| `src/error.rs` | Shared cryptographic and processing error types |
| `src/safe_output.rs` | Safe publication of new native files |
| `src/stream.rs` | FKT2 streaming encryption and restoration |
| `src/wasm.rs` | JavaScript-visible WebAssembly exports |
| `src/bin/encode.rs` | Native encryption command-line program |
| `src/bin/decode.rs` | Native restoration command-line program |
| `tests/round_trip.rs` | Integration tests |
| `web/encode.js` | Browser encryption and download helpers |
| `web/decode.js` | Browser restoration and download helpers |

---

# Dependencies

FileKit uses the following main dependencies:

| Dependency | Purpose |
|---|---|
| `aes-gcm` | AES-GCM authenticated encryption |
| `pbkdf2` | Passphrase-based key derivation |
| `sha2` | SHA-256 used by PBKDF2 |
| `getrandom` | Cryptographically secure random generation |
| `zeroize` | Clearing selected sensitive buffers when dropped |
| `thiserror` | Defining descriptive Rust error types |
| `rpassword` | Hidden terminal passphrase input in the CLI |
| `tempfile` | Temporary files for safe native output |
| `wasm-bindgen` | Connecting Rust functions to JavaScript |

The `Cargo.toml` file defines the dependency versions and features used by the project.

Before releasing a new version, run the tests and check the dependency tree and security advisories. Keep dependencies updated with due care, and retest after changes.

---

# Guidance for AI coding assistants

FileKit is intended to be reusable in other software projects. When using an AI coding assistant to integrate FileKit, the following instructions help maintain compatibility and avoid incorrect assumptions.

## Integration rules

1. **Read the public API first.** Check `src/lib.rs`, this README, and the relevant source files before writing integration code.
2. **Reuse FileKit's implementation.** Do not independently reimplement encryption or key derivation when the existing API meets the requirement.
3. **Choose the appropriate format.** Use the whole-file API for in-memory processing and the streaming API for native streaming operations.
4. **Respect format compatibility.** FKT1 and FKT2 have different formats and processing requirements. Do not assume every interface supports both.
5. **Distinguish API layers.** Rust functions such as `encode_file` and JavaScript helpers such as `encodeFile` are different interfaces. Check their parameter and return types.
6. **Handle errors explicitly.** Do not assume that file operations, WebAssembly initialisation, or cryptographic processing will always succeed.
7. **Preserve binary data.** Treat protected and restored data as bytes. Do not convert arbitrary binary files to text.
8. **Protect output files.** Use the safe-output helper when native files should not be overwritten or published partially.
9. **Respect streaming limitations.** A streaming decoder can write earlier authenticated chunks before a later error. Publish the final file only after successful completion.
10. **Do not invent APIs.** If a function, feature, or capability is not documented, inspect the current source and verify it before using it.
11. **Do not overstate security guarantees.** Describe what the code actually does and distinguish verified behaviour from assumptions.
12. **Test the integration.** Run relevant tests and identify any behaviour that has not been verified.

## Suggested workflow for AI-assisted development

When asked to add a FileKit integration:

1. Identify whether the target application is Rust, JavaScript, or another environment.
2. Identify whether the input is in-memory data, a native file stream, or a browser `File`/`Blob`.
3. Select the correct API.
4. Provide a minimal working example using the actual public signatures.
5. Add error handling and safe output handling where needed.
6. Explain any memory or format limitations.
7. Run the relevant build and tests if the environment permits.
8. Report exactly what was tested and what remains unverified.

**The guiding principle is to integrate with FileKit's existing interfaces rather than inventing new ones.**

---

# Contributing

Contributions are welcome.

Before proposing changes:

1. Read the relevant implementation and tests.
2. Keep the public API and file-format behaviour clear.
3. Add tests for new behaviour and regression cases.
4. Run `cargo test`.
5. Explain any compatibility, dependency, or security implications.

Changes to cryptographic operations or file formats deserve particular care because they can affect the ability to restore existing protected files.

Please describe the problem being solved, the proposed change, and the tests performed.

---

# Licence

FileKit is intended to be distributed under the dual licence:

- MIT
- Apache License, Version 2.0

You may choose either licence, subject to its terms.

The repository should include the corresponding `LICENSE-MIT` and `LICENSE-APACHE` licence texts.

---

# Disclaimer

FileKit is provided as software for file protection and restoration. It is supplied without a guarantee that it will meet every security or operational requirement.

Review the implementation, test it in the intended environment, protect passphrases appropriately, and maintain suitable backups of important data.