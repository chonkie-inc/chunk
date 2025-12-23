/**
 * memchunk - The fastest semantic text chunking library
 *
 * @example
 * ```javascript
 * import { init, chunk } from 'memchunk';
 *
 * await init();
 *
 * const text = new TextEncoder().encode("Hello. World. Test.");
 * for (const slice of chunk(text, { size: 10, delimiters: "." })) {
 *     console.log(new TextDecoder().decode(slice));
 * }
 * ```
 */

import initWasm, {
    Chunker as WasmChunker,
    default_target_size,
    default_delimiters,
    chunk_offsets as wasmChunkOffsets,
} from './pkg/memchunk_wasm.js';

export { default_target_size, default_delimiters };

/**
 * Split text into chunks at delimiter boundaries.
 * Returns an iterator of zero-copy Uint8Array subarray views.
 *
 * @param {Uint8Array} text - The text to chunk as bytes
 * @param {Object} [options] - Options
 * @param {number} [options.size=4096] - Target chunk size in bytes
 * @param {string} [options.delimiters="\n.?"] - Delimiter characters
 * @yields {Uint8Array} Zero-copy subarray views of the original text
 *
 * @example
 * const text = new TextEncoder().encode("Hello. World. Test.");
 * for (const slice of chunk(text, { size: 10, delimiters: "." })) {
 *     console.log(new TextDecoder().decode(slice));
 * }
 */
export function* chunk(text, options = {}) {
    const { size, delimiters } = options;
    const flat = wasmChunkOffsets(text, size, delimiters);
    for (let i = 0; i < flat.length; i += 2) {
        yield text.subarray(flat[i], flat[i + 1]);
    }
}

/**
 * Get chunk offsets without creating views.
 * Returns an array of [start, end] offset pairs.
 *
 * @param {Uint8Array} text - The text to chunk as bytes
 * @param {Object} [options] - Options
 * @param {number} [options.size=4096] - Target chunk size in bytes
 * @param {string} [options.delimiters="\n.?"] - Delimiter characters
 * @returns {Array<[number, number]>} Array of [start, end] offset pairs
 */
export function chunk_offsets(text, options = {}) {
    const { size, delimiters } = options;
    const flat = wasmChunkOffsets(text, size, delimiters);
    const pairs = [];
    for (let i = 0; i < flat.length; i += 2) {
        pairs.push([flat[i], flat[i + 1]]);
    }
    return pairs;
}

let initialized = false;

/**
 * Initialize the WASM module. Must be called before using chunk functions.
 */
export async function init() {
    if (!initialized) {
        await initWasm();
        initialized = true;
    }
}

/**
 * Chunker splits text at delimiter boundaries.
 * Implements Symbol.iterator for use in for...of loops.
 */
export class Chunker {
    /**
     * Create a new Chunker.
     * @param {Uint8Array} text - The text to chunk as bytes
     * @param {Object} [options] - Options
     * @param {number} [options.size=4096] - Target chunk size in bytes
     * @param {string} [options.delimiters="\n.?"] - Delimiter characters
     */
    constructor(text, options = {}) {
        const { size, delimiters } = options;
        this._chunker = new WasmChunker(text, size, delimiters);
    }

    /**
     * Get the next chunk, or undefined if exhausted.
     * @returns {Uint8Array | undefined}
     */
    next() {
        return this._chunker.next();
    }

    /**
     * Reset the chunker to iterate from the beginning.
     */
    reset() {
        this._chunker.reset();
    }

    /**
     * Collect all chunk offsets as an array of [start, end] pairs.
     * This is faster than iterating as it makes a single WASM call.
     * @returns {Array<[number, number]>}
     */
    collectOffsets() {
        const flat = this._chunker.collect_offsets();
        const pairs = [];
        for (let i = 0; i < flat.length; i += 2) {
            pairs.push([flat[i], flat[i + 1]]);
        }
        return pairs;
    }

    /**
     * Free the underlying WASM memory.
     */
    free() {
        this._chunker.free();
    }

    /**
     * Iterator protocol - allows use in for...of loops.
     */
    *[Symbol.iterator]() {
        let chunk;
        while ((chunk = this._chunker.next()) !== undefined) {
            yield chunk;
        }
    }
}
