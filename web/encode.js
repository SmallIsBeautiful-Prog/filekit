
import init, { encode_file } from "./pkg/filekit.js";

let initialization;

// Initialize WebAssembly only once, even if multiple callers use it.
async function ensureReady() {
    if (!initialization) {
        initialization = init();
    }

    await initialization;
}

/**
 * Protect a browser File or Blob.
 *
 * @param {File|Blob} file - The file to process.
 * @param {string} passphrase - The shared secret.
 * @returns {Promise<Blob>} The protected binary data.
 */
export async function encodeFile(file, passphrase) {
    if (!file) {
        throw new Error("Choose a file first.");
    }

    if (!passphrase) {
        throw new Error("Enter a passphrase first.");
    }

    await ensureReady();

    // Read the file as bytes; do not convert it to text.
    const input = new Uint8Array(await file.arrayBuffer());

    // Call the Rust function compiled to WebAssembly.
    const output = encode_file(input, passphrase);

    return new Blob([output], {
        type: "application/octet-stream"
    });
}

/**
 * Offer a Blob for download and release the temporary URL.
 *
 * The browser controls where the user saves the file.
 * This function does not write directly to arbitrary paths.
 */
export function downloadBlob(blob, filename) {
    if (!(blob instanceof Blob)) {
        throw new TypeError("Expected a Blob.");
    }

    if (
        typeof filename !== "string" ||
        !filename.trim() ||
        /[\\/:*?"<>|\u0000-\u001F]/.test(filename)
    ) {
        throw new Error("Invalid download filename.");
    }

    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");

    link.href = url;
    link.download = filename;
    link.hidden = true;

    document.body.appendChild(link);

    try {
        link.click();
    } finally {
        link.remove();

        // Release the temporary URL after allowing the
        // browser time to begin handling the download.
        setTimeout(() => URL.revokeObjectURL(url), 1000);
    }
}

