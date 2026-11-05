# FileKit — Web Developer Guide

*A practical guide to integrating Rust-powered file protection into browser applications.*

## 1. Introduction

FileKit is a file-protection library written in Rust and made available to JavaScript applications through WebAssembly.

It allows browser applications to encrypt and decrypt files locally, without sending their contents to an external encryption service.

This guide walks through integrating FileKit into a web application, explains the JavaScript API, and explores how the underlying Rust and WebAssembly components work together.

You do not need to know Rust or WebAssembly to use the JavaScript helpers.

Some familiarity with JavaScript, ES modules and browser file handling will be useful.

### What you'll learn

By the end of this guide, you'll understand how to:

- Install and import FileKit.
- Initialise and use its WebAssembly module.
- Select files through a browser interface.
- Encrypt and decrypt files using passphrases.
- Download the resulting files.
- Access the lower-level WebAssembly API.
- Understand the relationship between JavaScript, WebAssembly and Rust.
- Integrate FileKit with or without a JavaScript bundler.

A complete working example is included in the package's `demo/` directory.

---

## 2. Understanding the architecture

Before integrating FileKit, it helps to understand how its components fit together.

FileKit separates the cryptographic implementation from the browser interface.

The Rust core is compiled into the WebAssembly binary. The generated JavaScript module provides the interface that allows a browser application to initialise and call that compiled code.

```text
       Browser application
               |
               v
        JavaScript helpers
         encode.js / decode.js
               |
               v
        Shared WASM initialiser
           wasm-init.js
               |
               v
        Generated JavaScript interface
             filekit.js
               |
               v
        WebAssembly binary
        filekit_bg.wasm
        (contains the compiled Rust core)
```

Each component has a specific responsibility.

**JavaScript application**

Handles the browser interface, file selection, passphrase input and downloads.

**JavaScript helpers**

Provide convenient functions that accept browser `File` or `Blob` objects.

**WASM initialiser**

Ensures the WebAssembly module is initialised and shares the initialisation promise between the encryption and decryption helpers.

**Generated JavaScript interface**

Connects JavaScript to the functions exported by the compiled Rust code.

**Compiled Rust core**

Performs the underlying file-format processing and cryptographic operations.

The generated `filekit.js` module loads the accompanying `filekit_bg.wasm` binary at runtime. These files must remain available together in the deployed environment.

The package also includes TypeScript declarations describing the JavaScript-visible API.

This separation allows the Rust implementation to be reused independently of the browser interface.

---

## 3. Installing and integrating FileKit

FileKit can be integrated into a browser project in two ways:

1. Install the npm package and import its modules.
2. Use the supplied JavaScript and WebAssembly files directly in a browser project.

### Option A — Install through npm

Install the package:

```bash
npm install @smallisbeautiful/filekit
```

FileKit is an ES module package.

The examples in this guide assume a modern JavaScript development environment with npm package resolution.

For example, an application might have this structure:

```text
my-app/
|
|-- index.html
|-- package.json
|
|-- src/
|   |-- main.js
|   |-- file-protection.js
|
|-- node_modules/
    |-- @smallisbeautiful/
        |-- filekit/
```

You can import the required FileKit modules directly into your application.

### Available package imports

The package provides four public import paths:

| Import | Purpose |
|---|---|
| `@smallisbeautiful/filekit` | Low-level WebAssembly API |
| `@smallisbeautiful/filekit/wasm` | Alias to the same low-level API |
| `@smallisbeautiful/filekit/encode` | High-level encryption helper |
| `@smallisbeautiful/filekit/decode` | High-level decryption helper |

For most browser applications, the `/encode` and `/decode` imports are the recommended starting point.

The root import and `/wasm` alias expose the lower-level WebAssembly functions for applications that need more control.

### Bundler considerations

FileKit can be used in projects that support npm package resolution and ES modules.

JavaScript bundlers differ in how they handle WebAssembly assets and dynamically referenced files.

The generated `filekit.js` loader locates `filekit_bg.wasm` relative to its own module location. Your chosen bundler must preserve this relationship and ensure that the WASM binary is included and accessible in the deployed application.

Check the output produced by your bundler and test the deployed application, rather than assuming that every bundler handles the generated WASM loader in the same way.

### Option B — Manual browser integration

FileKit can also be integrated without installing it through npm or using a bundler.

This approach is useful for smaller projects or static websites that use browser ES modules directly.

The JavaScript helper files and generated WebAssembly files must be kept together in the same directory.

A simple project structure could look like this:

```text
my-project/
|
|-- index.html
|
|-- js/
|   |-- app.js
|
|-- lib/
    |-- filekit/
        |-- encode.js
        |-- decode.js
        |-- wasm-init.js
        |-- filekit.js
        |-- filekit_bg.wasm
```

The files required for this integration are:

- `encode.js`
- `decode.js`
- `wasm-init.js`
- `filekit.js`
- `filekit_bg.wasm`

The package also includes TypeScript declaration files for developers who want type information in their projects.

Once FileKit has been published, the package files can be obtained from the npm package page:

https://www.npmjs.com/package/@smallisbeautiful/filekit

Keep the supplied JavaScript files and WebAssembly binary together in the same directory. The generated loader uses a relative reference to locate `filekit_bg.wasm`.

Your application can then import the helpers using relative paths:

```javascript
import {
    encodeFile,
    downloadBlob
} from "../lib/filekit/encode.js";

import {
    decodeFile
} from "../lib/filekit/decode.js";
```

The exact relative paths will depend on your project structure.

### Serving the application

WebAssembly should be tested through a local HTTP server rather than by opening the HTML file directly through a `file://` URL.

Section 12 explains how to start a local server and run the included demo.

---

## 4. Using the high-level JavaScript API

For most browser applications, the high-level JavaScript helpers are the recommended starting point.

They handle the conversion between browser file objects and the byte arrays required by WebAssembly.

Although the helpers accept both `File` and `Blob` objects, the examples below use browser `File` objects obtained from a file input. This is because `File` objects provide a filename through their `name` property, whereas a plain `Blob` does not.

There are two separate entry points.

### Encryption

```javascript
import {
    encodeFile,
    downloadBlob
} from "@smallisbeautiful/filekit/encode";
```

The `encodeFile()` function accepts a browser `File` or `Blob` and returns a `Promise` containing the encrypted data as a Blob.

Example:

```javascript
async function encryptFile(file, passphrase) {
    const protectedBlob = await encodeFile(
        file,
        passphrase
    );

    downloadBlob(
        protectedBlob,
        file.name + ".fkt"
    );
}
```

The function can be called from a browser event handler:

```javascript
const fileInput = document.querySelector("#fileInput");
const passphraseInput = document.querySelector("#passphrase");

async function handleEncryption() {
    const file = fileInput.files[0];
    const passphrase = passphraseInput.value;

    if (!file || !passphrase) {
        return;
    }

    try {
        await encryptFile(file, passphrase);
    } catch (error) {
        console.error("Encryption failed:", error);
    }
}
```

### Decryption

```javascript
import {
    decodeFile,
    downloadBlob
} from "@smallisbeautiful/filekit/decode";
```

The `decodeFile()` function accepts a protected browser file and its original passphrase.

Example:

```javascript
async function decryptFile(file, passphrase) {
    const restoredBlob = await decodeFile(
        file,
        passphrase
    );

    const originalName = file.name.replace(/\.fkt$/, "");

    downloadBlob(
        restoredBlob,
        originalName || "restored-file"
    );
}
```

If the passphrase is incorrect, or the protected data has been modified, decryption fails.

Always handle errors in the application interface so users receive useful feedback.

### How the helpers work

The encryption helper performs several operations:

1. Reads the browser file into an `ArrayBuffer`.
2. Converts the file data into a `Uint8Array`.
3. Ensures the WebAssembly module is initialised.
4. Calls the Rust-backed `encode_file()` function.
5. Converts the returned encrypted bytes into a Blob.

The decryption helper follows the same general process, using `decode_file()` instead.

The application receives a Blob, which can then be downloaded or processed further.

### Shared WebAssembly initialisation

The package includes `wasm-init.js`.

It maintains a shared initialisation promise so that the encryption and decryption helpers can use the same WebAssembly instance.

You do not need to initialise WebAssembly manually when using `encodeFile()` or `decodeFile()`.

The helpers call the shared initialiser internally.

This avoids requiring every application to manage WebAssembly initialisation itself.

---

## 5. Building a simple browser interface

A browser application needs a way for users to select files, enter a passphrase and initiate operations.

A minimal HTML interface could look like this:

```html
<h2>Encrypt a file</h2>

<input type="file" id="fileInput">

<input
    type="password"
    id="encryptPassphrase"
    placeholder="Enter passphrase"
>

<button id="encryptButton">
    Encrypt
</button>

<hr>

<h2>Decrypt a file</h2>

<input type="file" id="protectedFile">

<input
    type="password"
    id="decryptPassphrase"
    placeholder="Enter passphrase"
>

<button id="decryptButton">
    Decrypt
</button>

<p id="status"></p>
```

Your JavaScript can connect these elements to the FileKit helpers.

For example:

```javascript
import {
    encodeFile,
    downloadBlob as downloadEncoded
} from "@smallisbeautiful/filekit/encode";

import {
    decodeFile,
    downloadBlob as downloadDecoded
} from "@smallisbeautiful/filekit/decode";

const fileInput = document.querySelector("#fileInput");
const protectedFile = document.querySelector("#protectedFile");

const encryptPassphrase = document.querySelector("#encryptPassphrase");
const decryptPassphrase = document.querySelector("#decryptPassphrase");

const status = document.querySelector("#status");

document.querySelector("#encryptButton").addEventListener(
    "click",
    async () => {
        const file = fileInput.files[0];
        const passphrase = encryptPassphrase.value;

        if (!file || !passphrase) {
            status.textContent = "Select a file and enter a passphrase.";
            return;
        }

        try {
            status.textContent = "Encrypting...";

            const result = await encodeFile(file, passphrase);

            downloadEncoded(result, file.name + ".fkt");

            status.textContent = "Encryption complete.";
        } catch (error) {
            console.error(error);
            status.textContent = "Encryption failed.";
        }
    }
);

document.querySelector("#decryptButton").addEventListener(
    "click",
    async () => {
        const file = protectedFile.files[0];
        const passphrase = decryptPassphrase.value;

        if (!file || !passphrase) {
            status.textContent = "Select a protected file and enter a passphrase.";
            return;
        }

        try {
            status.textContent = "Decrypting...";

            const result = await decodeFile(file, passphrase);

            const filename = file.name.replace(/\.fkt$/, "");

            downloadDecoded(result, filename || "restored-file");

            status.textContent = "Decryption complete.";
        } catch (error) {
            console.error(error);
            status.textContent = "Decryption failed. Check the passphrase and file.";
        }
    }
);
```

This is a deliberately simple example. A production application should also consider disabling buttons while operations are running, clearing passphrase fields when appropriate, and providing accessible status messages.

---

## 6. Understanding the download helper

Browsers do not generally allow web applications to write directly to arbitrary filesystem locations.

FileKit's `downloadBlob()` helper uses the browser's download mechanism instead.

For example:

```javascript
downloadBlob(
    protectedBlob,
    "document.txt.fkt"
);
```

The helper:

1. Creates a temporary object URL for the Blob.
2. Creates a temporary download link.
3. Triggers the browser download.
4. Cleans up the temporary resources.

The browser controls where the downloaded file is saved, according to its download settings.

This is intentionally different from native applications, which can write files directly to filesystem paths.

---

## 7. Accessing the lower-level WebAssembly API

The high-level helpers are convenient, but FileKit also exposes its lower-level WebAssembly functions.

These are useful when an application already manages its own byte arrays or needs more control over file processing.

Import the generated functions:

```javascript
import init, {
    encode_file,
    decode_file
} from "@smallisbeautiful/filekit";
```

The `/wasm` import path provides the same low-level API.

### Initialising WebAssembly

Before calling the cryptographic functions, initialise the module:

```javascript
await init();
```

The generated loader locates `filekit_bg.wasm` relative to its module location.

Your development server or bundler must make this binary available to the browser.

### Encrypting byte data

```javascript
const encoder = new TextEncoder();

const originalData = encoder.encode(
    "Example FileKit data"
);

const protectedData = encode_file(
    originalData,
    "example-passphrase"
);
```

The result is a `Uint8Array` containing the protected data.

### Decrypting byte data

```javascript
const restoredData = decode_file(
    protectedData,
    "example-passphrase"
);
```

The restored result is also a `Uint8Array`.

Convert it back to text when working with textual data:

```javascript
const decoder = new TextDecoder();

const restoredText = decoder.decode(restoredData);

console.log(restoredText);
```

### Understanding the difference

| High-level helpers | Low-level WASM API |
|---|---|
| Accepts `File` or `Blob` | Accepts `Uint8Array` |
| Returns a `Promise<Blob>` | Returns `Uint8Array` |
| Handles file conversion | Application manages conversion |
| Initialises WASM internally | Application initialises WASM |
| Convenient for browser file interfaces | Useful for custom data workflows |

Both interfaces use the same underlying Rust cryptographic implementation.

---

## 8. Understanding Rust and WebAssembly

One of the interesting aspects of FileKit is that the cryptographic implementation is not written in JavaScript.

It is written in Rust and compiled to WebAssembly.

### What is Rust doing?

Rust contains the core file-protection implementation.

It handles:

- File format processing.
- Passphrase-based key derivation.
- AES-256-GCM encryption.
- Authenticated decryption.
- Validation and error handling.

This means the same underlying Rust implementation can be used by different interfaces.

For example:

- Native Rust applications.
- Command-line tools.
- WebAssembly browser applications.

The browser and native implementations can therefore share the same underlying file-protection logic.

### What is WebAssembly doing?

WebAssembly (WASM) is a portable binary instruction format that modern browsers can execute.

It allows code written in languages such as Rust to run inside browser environments.

In FileKit, WebAssembly acts as the bridge between JavaScript and the compiled Rust implementation.

JavaScript calls the exported functions, and WebAssembly executes the compiled Rust code.

### What is JavaScript doing?

JavaScript handles the browser-specific tasks.

It provides:

- File selection.
- Passphrase input.
- Browser file reading.
- Conversion between file objects and byte arrays.
- Download handling.
- User-interface interaction.

The result is a separation of responsibilities: Rust performs the core protection operations, WebAssembly makes them available to the browser, and JavaScript connects everything to the user interface.

---

## 9. File formats and compatibility

FileKit currently has two file formats in its broader Rust implementation.

### FKT1 — Whole-file format

FKT1 is the format exposed by the current browser WebAssembly API.

The browser helpers read the complete file into memory before passing it to the Rust implementation.

The native Rust implementation can also read and write FKT1 files.

This provides compatibility between the browser and native implementations.

### FKT2 — Streaming format

FKT2 is the streaming format used by the native Rust implementation.

It is designed to support processing data in smaller portions rather than requiring the entire file to be held in memory.

The current browser WebAssembly interface does not expose FKT2 streaming functions.

### Compatibility overview

| Capability | Native Rust | Browser WASM |
|---|---:|---:|
| FKT1 encryption | Yes | Yes |
| FKT1 decryption | Yes | Yes |
| FKT2 encryption | Yes | Not currently exposed |
| FKT2 decryption | Yes | Not currently exposed |
| Streaming processing | Yes | Not currently exposed |

For cross-platform file exchange between the native Rust and browser WASM implementations, use the FKT1 format.

Both implementations support FKT1 encryption and decryption, allowing files protected by one implementation to be opened by the other, provided they use compatible FileKit versions.

---

## 10. Browser memory considerations

The current browser helpers process files as whole-file operations.

They use `file.arrayBuffer()` to read the complete file before passing it to the Rust implementation.

This has some practical consequences:

- The entire input file must fit into available memory.
- Additional memory is required for processing and output data.
- Large files can create significant memory pressure.
- Browser memory limits vary between devices and browsers.

For this reason, the current browser implementation is best suited to files that can comfortably fit into available browser memory.

For larger files, the native Rust implementation provides the FKT2 streaming approach.

---

## 11. Security considerations

FileKit uses established cryptographic primitives.

### AES-256-GCM

AES-256-GCM provides authenticated encryption.

It protects the confidentiality of the file contents and provides integrity protection, allowing modified or incorrectly authenticated data to be rejected during decryption.

### PBKDF2-HMAC-SHA-256

FileKit uses PBKDF2 with HMAC-SHA-256 to derive an encryption key from the user's passphrase.

This makes the encryption key dependent on the supplied passphrase and the parameters stored in the protected file format.

The current default work factor is 600,000 iterations. The iteration count is stored in the protected file format so that the decoder can use the parameters associated with the file.

### Passphrase handling

The passphrase is supplied by the application and passed to the WebAssembly implementation.

Applications should take care not to:

- Store passphrases in local storage.
- Include passphrases in URLs.
- Log passphrases to the browser console.
- Send passphrases to external services unnecessarily.
- Retain passphrases longer than required.

A lost passphrase cannot be recovered through FileKit.

### Important limitations

FileKit protects file contents, but it does not protect the entire application environment.

For example, it cannot prevent:

- Malware from accessing files before encryption.
- A compromised browser from observing user input.
- Malicious application code from accessing passphrases.
- Users from losing their passphrases.

FileKit has undergone project-level functional testing, including encryption/decryption round trips, wrong-passphrase rejection and tampering detection.

It has not undergone an independent professional cryptographic security audit.

---

## 12. Running the included demo

The package includes a working browser example in the `demo/` directory.

The demo uses the package-level JavaScript helpers rather than duplicating the generated WebAssembly files.

### Starting a local server

The demo must be served through HTTP or HTTPS so that the browser can load the WebAssembly module correctly.

Open a terminal in the package directory.

**Option 1 — Python**

If Python is installed, run:

```powershell
python -m http.server 8000
```

Alternatively, on Windows systems where the Python launcher is available:

```powershell
py -m http.server 8000
```

**Option 2 — Node.js using npx**

If Node.js is installed, you can use the `http-server` package through npx without installing it globally:

```powershell
npx http-server . -p 8000
```

In Windows PowerShell, if the `npx` command is blocked by the execution policy, use:

```powershell
npx.cmd http-server . -p 8000
```

If your project already has a development server configured, such as Vite's development server, you can use that instead.

The important requirement is that the server makes the JavaScript modules and WebAssembly binary available to the browser at their expected paths.

### Opening the demo

Once the server is running, open:

```text
http://localhost:8000/demo/
```

Leave the terminal running while you use the demo.

To stop the server, return to the terminal and press `Ctrl+C`.

### Testing the demo

1. Select a file in the encryption section.
2. Enter a passphrase.
3. Encrypt and download the protected file.
4. Select the protected file in the decryption section.
5. Enter the same passphrase.
6. Decrypt and download the restored file.
7. Compare the restored file with the original.

You can also test an incorrect passphrase to confirm that decryption fails as expected.

The demo provides a practical reference for connecting FileKit to a browser interface.

---

## 13. Troubleshooting

### WebAssembly fails to initialise

Check that:

- The application is served over HTTP or HTTPS.
- The generated `filekit_bg.wasm` binary is included in the package.
- Your bundler or development server can resolve the generated loader's WASM resource.
- The browser console does not report a failed resource request.
- The JavaScript loader and WASM binary remain together at the expected relative paths.

Opening an HTML file directly through a `file://` URL is not a reliable way to test WebAssembly loading.

### Encryption or decryption fails

Check that:

- A file has been selected.
- A non-empty passphrase has been supplied.
- The protected file is compatible with the browser FKT1 implementation.
- The protected data has not been modified.
- The original passphrase is being used.

### Large files cause problems

The current browser interface reads complete files into memory.

Try a smaller file or consider the native Rust implementation for streaming operations.

### Imports cannot be resolved

If you are using npm, make sure FileKit has been installed:

```bash
npm install @smallisbeautiful/filekit
```

Check that your application supports ES modules and npm package resolution.

If you are using manual browser integration, check that:

- The required JavaScript files are present.
- The import paths match your project structure.
- The generated WASM binary is in the same directory as the generated loader.
- The application is being served through a local HTTP server.

### The WASM file cannot be found

The generated `filekit.js` loader resolves `filekit_bg.wasm` relative to its own module location.

If the loader or WASM binary has been moved independently, the browser may be unable to find the binary.

Keep the files together and check the browser's network panel for failed resource requests.

---

## 14. Further development

FileKit is designed as a reusable Rust library with multiple interfaces.

The current npm package provides a browser interface through WebAssembly, while the native Rust implementation provides additional functionality, including streaming file protection.

Future development may extend the browser interface with additional capabilities, including streaming support.

The package is intended to make the underlying Rust implementation accessible to JavaScript developers without requiring them to understand the internal cryptographic implementation.

---

## 15. Project resources

**Source code**

https://github.com/SmallIsBeautiful-Prog/filekit

**npm package**

https://www.npmjs.com/package/@smallisbeautiful/filekit

**Licence**

FileKit is dual-licensed under MIT or Apache-2.0.

See the included licence files for details.

---

*FileKit — Built in Rust. Made accessible through WebAssembly.*