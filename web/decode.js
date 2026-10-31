
import init, { decode_file } from "./pkg/filekit.js";

let initialization;

async function ensureReady() {
    if (!initialization) {
        initialization = init();
    }

    await initialization;
}

/**
 * Restore a protected browser File or Blob.
 *
 * @param {File|Blob} file - The protected file.
 * @param {string} passphrase - The original shared secret.
 * @returns {Promise<Blob>} The restored binary data.
 */
export async function decodeFile(file, passphrase) {
    if (!file) {
        throw new Error("Choose a file first.");
    }

    if (!passphrase) {
        throw new Error("Enter a passphrase first.");
    }

    await ensureReady();

    const input = new Uint8Array(await file.arrayBuffer());

    // Rust verifies the authentication tag before returning data.
    const output = decode_file(input, passphrase);

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

