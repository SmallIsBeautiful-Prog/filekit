export function encodeFile(
    file: File | Blob,
    passphrase: string
): Promise<Blob>;

export function downloadBlob(
    blob: Blob,
    filename: string
): void;