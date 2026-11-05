# FileKit

**File protection for browser applications, powered by Rust and WebAssembly.**

FileKit is a file-protection library built in Rust, with a WebAssembly interface that allows JavaScript applications to encrypt and decrypt files directly in the browser.

It uses authenticated encryption to protect file contents and is designed to provide a reusable core that can be accessed through Rust, WebAssembly and JavaScript.

## Features

- **AES-256-GCM** authenticated encryption.
- **PBKDF2-HMAC-SHA-256** passphrase-based key derivation.
- Browser-friendly JavaScript helper functions.
- Lower-level WebAssembly functions for direct access.
- TypeScript declaration files included.
- Compatible file format shared with the native Rust implementation (FKT1).
- No server-side encryption service required.

## Installation

Install FileKit using npm:

```bash
npm install @smallisbeautiful/filekit
```

### Available package imports

| Import | Purpose |
|---|---|
| `@smallisbeautiful/filekit` | Low-level WebAssembly API |
| `@smallisbeautiful/filekit/wasm` | Alias for the low-level API |
| `@smallisbeautiful/filekit/encode` | High-level encryption helper |
| `@smallisbeautiful/filekit/decode` | High-level decryption helper |

For most browser applications, the `/encode` and `/decode` imports are the recommended starting point.

## Quick start

FileKit provides separate JavaScript helpers for encryption and decryption.

### Encrypt a file

```javascript
import {
    encodeFile,
    downloadBlob
} from "@smallisbeautiful/filekit/encode";

const fileInput = document.querySelector("#fileInput");

async function encryptSelectedFile(passphrase) {
    const file = fileInput.files[0];

    if (!file) {
        throw new Error("Please select a file.");
    }

    const protectedBlob = await encodeFile(file, passphrase);

    downloadBlob(
        protectedBlob,
        file.name + ".fkt"
    );
}
```

### Decrypt a file

```javascript
import {
    decodeFile,
    downloadBlob
} from "@smallisbeautiful/filekit/decode";

const fileInput = document.querySelector("#protectedFile");

async function decryptSelectedFile(passphrase) {
    const file = fileInput.files[0];

    if (!file) {
        throw new Error("Please select a protected file.");
    }

    const restoredBlob = await decodeFile(file, passphrase);

    const originalName = file.name.replace(/\.fkt$/, "");

    downloadBlob(
        restoredBlob,
        originalName || "restored-file"
    );
}
```

These examples assume a JavaScript environment that supports ES modules and resolves npm package imports.

Bundlers may differ in how they handle WebAssembly assets, so check that your chosen bundler makes the accompanying WASM binary available at runtime.

## JavaScript API

### `encodeFile(file, passphrase)`

Encrypts a browser `File` or `Blob`.

**Parameters**

| Parameter | Type | Description |
|---|---|---|
| `file` | `File` or `Blob` | Source data to encrypt |
| `passphrase` | `string` | Passphrase used for encryption |

**Returns**

`Promise<Blob>` containing the protected file data.

### `decodeFile(file, passphrase)`

Decrypts a FileKit-protected browser `File` or `Blob`.

**Parameters**

| Parameter | Type | Description |
|---|---|---|
| `file` | `File` or `Blob` | FileKit-protected data |
| `passphrase` | `string` | Original encryption passphrase |

**Returns**

`Promise<Blob>` containing the restored file data.

An incorrect passphrase or modified protected data causes decryption to fail.

### `downloadBlob(blob, filename)`

Triggers a browser download for a supplied Blob.

This helper uses a temporary object URL and a download link. It does not write directly to arbitrary filesystem locations.

## Lower-level WebAssembly API

FileKit also exposes its generated WebAssembly functions for applications that need more direct control.

Import them from the package root:

```javascript
import init, {
    encode_file,
    decode_file
} from "@smallisbeautiful/filekit";
```

Initialise WebAssembly before calling either function:

```javascript
await init();
```

The generated loader locates `filekit_bg.wasm` relative to its own module location. Your development server or bundler must make this binary available to the browser.

### `encode_file(file_data, passphrase)`

```javascript
const protectedData = encode_file(
    fileData,
    passphrase
);
```

Accepts a `Uint8Array` and returns a `Uint8Array` containing the protected data.

### `decode_file(protected_data, passphrase)`

```javascript
const restoredData = decode_file(
    protectedData,
    passphrase
);
```

Accepts protected data as a `Uint8Array` and returns the restored contents as a `Uint8Array`.

The lower-level functions are synchronous once WebAssembly has been initialised.

## File format compatibility

FileKit currently supports two formats in its broader Rust implementation:

- **FKT1:** Whole-file protection format, currently exposed through the browser WebAssembly interface.
- **FKT2:** Streaming format used by the native Rust implementation.

The browser API currently uses FKT1. The native Rust implementation supports FKT1 and FKT2, allowing compatible files to be exchanged between the browser and native implementations when using FKT1.

FKT2 streaming is not currently exposed through this npm package's browser API.

## How it works

FileKit is built around a reusable Rust core.

```text
JavaScript application
        |
        v
JavaScript helper functions
        |
        v
Generated JavaScript interface
        |
        v
WebAssembly binary
(contains the compiled Rust core)
        |
        v
Cryptographic operations
(AES-256-GCM)
```

Rust handles the underlying cryptographic operations. WebAssembly makes those functions accessible to browser applications, while JavaScript provides a convenient interface for working with browser files and downloads.

## Browser considerations

The browser helpers read the complete input file into memory before processing it.

This means:

- Available memory limits the practical file size.
- Very large files may consume substantial browser memory.
- The current browser interface does not provide streaming encryption or decryption.

For applications that need streaming file processing, consider using the native Rust implementation instead.

## Security considerations

FileKit uses:

- AES-256-GCM for authenticated encryption.
- PBKDF2-HMAC-SHA-256 for passphrase-based key derivation.

AES-GCM provides both confidentiality and integrity protection. If the protected data has been modified or the wrong passphrase is supplied, decryption fails.

A few important considerations:

- Use a strong, unique passphrase.
- Do not store the passphrase alongside the protected file.
- Losing the passphrase means the original file cannot be recovered through normal decryption.
- Encryption does not protect against malware, compromised devices, or insecure application environments.

**Security status:** FileKit has undergone project-level functional testing, including round-trip encryption/decryption, incorrect-passphrase rejection and tampering detection. It has not undergone an independent professional cryptographic security audit.

## Documentation and examples

For a practical walkthrough of integrating FileKit into a browser application, see:

**[Web Developer Guide](docs/WEB_DEVELOPER_GUIDE.md)**

A working browser example is included in the package:

**[Browser Demo](demo/index.html)**

The demo can be served locally using a simple HTTP development server.

## Project

FileKit is developed as an open-source Rust project.

- **GitHub:** https://github.com/SmallIsBeautiful-Prog/filekit
- **npm:** https://www.npmjs.com/package/@smallisbeautiful/filekit

## Licence

FileKit is dual-licensed under either:

- Apache License, Version 2.0
- MIT License

You may choose either licence.

See `LICENSE-APACHE` and `LICENSE-MIT` for the full licence texts.

---

*FileKit — Built in Rust. Made accessible through WebAssembly.*