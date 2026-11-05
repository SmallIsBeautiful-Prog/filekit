import init from "./filekit.js";

let initialization;

/**
 * Initialise WebAssembly once and share the
 * same initialisation promise across the helpers.
 */
export function ensureWasmReady() {
    if (!initialization) {
        initialization = init();
    }

    return initialization;
}