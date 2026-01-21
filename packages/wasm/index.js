/**
 * @chonkiejs/chunk - The fastest semantic text chunking library
 *
 * @example
 * ```javascript
 * import { init, chunk } from '@chonkiejs/chunk';
 *
 * await init();
 *
 * // Simple string API - strings in, strings out
 * for (const slice of chunk("Hello. World. Test.", { size: 10 })) {
 *     console.log(slice);
 * }
 *
 * // Or use bytes for zero-copy performance
 * const bytes = new TextEncoder().encode("Hello. World.");
 * for (const slice of chunk(bytes, { size: 10 })) {
 *     console.log(slice); // Uint8Array
 * }
 * ```
 */

import initWasm, {
    Chunker as WasmChunker,
    default_target_size,
    default_delimiters,
    chunk_offsets as wasmChunkOffsets,
    chunk_offsets_pattern as wasmChunkOffsetsPattern,
    split_offsets as wasmSplitOffsets,
    merge_splits as wasmMergeSplits,
} from './pkg/chonkiejs_chunk.js';

export { default_target_size, default_delimiters };

const encoder = new TextEncoder();
const decoder = new TextDecoder();

/**
 * Convert input to bytes if it's a string.
 * @param {string | Uint8Array} input
 * @returns {Uint8Array}
 */
function toBytes(input) {
    return typeof input === 'string' ? encoder.encode(input) : input;
}

/**
 * Split text into chunks at delimiter boundaries.
 * Accepts strings or Uint8Array. Returns the same type as input.
 *
 * @param {string | Uint8Array} text - The text to chunk
 * @param {Object} [options] - Options
 * @param {number} [options.size=4096] - Target chunk size in bytes
 * @param {string} [options.delimiters="\n.?"] - Delimiter characters
 * @param {string | Uint8Array} [options.pattern] - Multi-byte pattern to split on
 * @param {boolean} [options.prefix=false] - Put delimiter/pattern at start of next chunk
 * @param {boolean} [options.consecutive=false] - Split at START of consecutive runs
 * @param {boolean} [options.forwardFallback=false] - Search forward if no pattern in backward window
 * @yields {string | Uint8Array} Chunks (same type as input)
 *
 * @example
 * // String input returns strings
 * for (const slice of chunk("Hello. World.", { size: 10 })) {
 *     console.log(slice);
 * }
 *
 * @example
 * // With pattern (e.g., metaspace for SentencePiece)
 * for (const slice of chunk("Hello▁World▁Test", { pattern: "▁", prefix: true })) {
 *     console.log(slice);
 * }
 */
export function* chunk(text, options = {}) {
    const isString = typeof text === 'string';
    const bytes = toBytes(text);
    const { size, delimiters, pattern, prefix, consecutive, forwardFallback } = options;

    let flat;
    if (pattern) {
        const patternBytes = toBytes(pattern);
        flat = wasmChunkOffsetsPattern(bytes, size ?? 4096, patternBytes, prefix, consecutive, forwardFallback);
    } else {
        flat = wasmChunkOffsets(bytes, size, delimiters, prefix);
    }

    for (let i = 0; i < flat.length; i += 2) {
        const slice = bytes.subarray(flat[i], flat[i + 1]);
        yield isString ? decoder.decode(slice) : slice;
    }
}

/**
 * Get chunk offsets without creating views.
 * Returns an array of [start, end] offset pairs.
 *
 * @param {string | Uint8Array} text - The text to chunk
 * @param {Object} [options] - Options
 * @param {number} [options.size=4096] - Target chunk size in bytes
 * @param {string} [options.delimiters="\n.?"] - Delimiter characters
 * @param {string | Uint8Array} [options.pattern] - Multi-byte pattern to split on
 * @param {boolean} [options.prefix=false] - Put delimiter/pattern at start of next chunk
 * @param {boolean} [options.consecutive=false] - Split at START of consecutive runs
 * @param {boolean} [options.forwardFallback=false] - Search forward if no pattern in backward window
 * @returns {Array<[number, number]>} Array of [start, end] byte offset pairs
 */
export function chunk_offsets(text, options = {}) {
    const bytes = toBytes(text);
    const { size, delimiters, pattern, prefix, consecutive, forwardFallback } = options;

    let flat;
    if (pattern) {
        const patternBytes = toBytes(pattern);
        flat = wasmChunkOffsetsPattern(bytes, size ?? 4096, patternBytes, prefix, consecutive, forwardFallback);
    } else {
        flat = wasmChunkOffsets(bytes, size, delimiters, prefix);
    }

    const pairs = [];
    for (let i = 0; i < flat.length; i += 2) {
        pairs.push([flat[i], flat[i + 1]]);
    }
    return pairs;
}

/**
 * Split text at every delimiter occurrence.
 * Unlike chunk() which creates size-based chunks, this splits at
 * **every** delimiter occurrence.
 *
 * @param {string | Uint8Array} text - The text to split
 * @param {Object} [options] - Options
 * @param {string} [options.delimiters="\n.?"] - Delimiter characters
 * @param {string} [options.includeDelim="prev"] - Where to attach delimiter: "prev", "next", or "none"
 * @param {number} [options.minChars=0] - Minimum characters per segment. Shorter segments are merged.
 * @yields {string | Uint8Array} Segments (same type as input)
 *
 * @example
 * // String input returns strings
 * for (const segment of split("Hello. World. Test.", { delimiters: "." })) {
 *     console.log(segment); // "Hello.", " World.", " Test."
 * }
 */
export function* split(text, options = {}) {
    const isString = typeof text === 'string';
    const bytes = toBytes(text);
    const { delimiters, includeDelim, minChars } = options;

    const flat = wasmSplitOffsets(bytes, delimiters, includeDelim, minChars);

    for (let i = 0; i < flat.length; i += 2) {
        const slice = bytes.subarray(flat[i], flat[i + 1]);
        yield isString ? decoder.decode(slice) : slice;
    }
}

/**
 * Get split offsets without creating views.
 * Unlike chunk_offsets() which creates size-based chunks, this splits at
 * **every** delimiter occurrence.
 *
 * @param {string | Uint8Array} text - The text to split
 * @param {Object} [options] - Options
 * @param {string} [options.delimiters="\n.?"] - Delimiter characters
 * @param {string} [options.includeDelim="prev"] - Where to attach delimiter: "prev", "next", or "none"
 * @param {number} [options.minChars=0] - Minimum characters per segment. Shorter segments are merged.
 * @returns {Array<[number, number]>} Array of [start, end] byte offset pairs
 *
 * @example
 * const offsets = split_offsets("Hello. World.", { delimiters: "." });
 * // [[0, 6], [6, 13]]
 */
export function split_offsets(text, options = {}) {
    const bytes = toBytes(text);
    const { delimiters, includeDelim, minChars } = options;

    const flat = wasmSplitOffsets(bytes, delimiters, includeDelim, minChars);

    const pairs = [];
    for (let i = 0; i < flat.length; i += 2) {
        pairs.push([flat[i], flat[i + 1]]);
    }
    return pairs;
}

/**
 * Merge segments based on token counts, respecting chunk size limits.
 *
 * This is the equivalent of Chonkie's Cython `_merge_splits` function.
 * Used by RecursiveChunker to merge small segments into larger chunks
 * that fit within a token budget.
 *
 * @param {number[] | Uint32Array} tokenCounts - Array of token counts for each segment
 * @param {number} chunkSize - Maximum tokens per merged chunk
 * @param {boolean} [combineWhitespace=false] - If true, adds +1 token per join for whitespace
 * @returns {{indices: number[], tokenCounts: number[]}} Object with indices and token counts
 *
 * @example
 * const result = merge_splits([1, 1, 1, 1, 1, 1, 1], 3);
 * // result.indices = [3, 6, 7]
 * // result.tokenCounts = [3, 3, 1]
 *
 * @example
 * // With whitespace tokens
 * const result = merge_splits([1, 1, 1, 1, 1, 1, 1], 5, true);
 * // result.indices = [3, 6, 7]
 * // result.tokenCounts = [5, 5, 1] (3 tokens + 2 whitespace joins per chunk)
 */
export function merge_splits(tokenCounts, chunkSize, combineWhitespace = false) {
    const flat = wasmMergeSplits(tokenCounts, chunkSize, combineWhitespace);
    const indices = [];
    const counts = [];
    for (let i = 0; i < flat.length; i += 2) {
        indices.push(flat[i]);
        counts.push(flat[i + 1]);
    }
    return { indices, tokenCounts: counts };
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
 *
 * @example
 * // String input
 * const chunker = new Chunker("Hello. World. Test.", { size: 10 });
 * for (const slice of chunker) {
 *     console.log(slice); // strings
 * }
 *
 * @example
 * // With pattern
 * const chunker = new Chunker("Hello▁World", { pattern: "▁", prefix: true });
 * for (const slice of chunker) {
 *     console.log(slice);
 * }
 */
export class Chunker {
    /**
     * Create a new Chunker.
     * @param {string | Uint8Array} text - The text to chunk
     * @param {Object} [options] - Options
     * @param {number} [options.size=4096] - Target chunk size in bytes
     * @param {string} [options.delimiters="\n.?"] - Delimiter characters
     * @param {string | Uint8Array} [options.pattern] - Multi-byte pattern to split on
     * @param {boolean} [options.prefix=false] - Put delimiter/pattern at start of next chunk
     * @param {boolean} [options.consecutive=false] - Split at START of consecutive runs
     * @param {boolean} [options.forwardFallback=false] - Search forward if no pattern in backward window
     */
    constructor(text, options = {}) {
        this._isString = typeof text === 'string';
        const bytes = toBytes(text);
        const { size, delimiters, pattern, prefix, consecutive, forwardFallback } = options;

        if (pattern) {
            const patternBytes = toBytes(pattern);
            this._chunker = WasmChunker.with_pattern(bytes, size ?? 4096, patternBytes, prefix, consecutive, forwardFallback);
        } else {
            this._chunker = new WasmChunker(bytes, size, delimiters, prefix);
        }
    }

    /**
     * Get the next chunk, or undefined if exhausted.
     * @returns {string | Uint8Array | undefined}
     */
    next() {
        const chunk = this._chunker.next();
        if (chunk === undefined) return undefined;
        return this._isString ? decoder.decode(chunk) : chunk;
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
            yield this._isString ? decoder.decode(chunk) : chunk;
        }
    }
}
