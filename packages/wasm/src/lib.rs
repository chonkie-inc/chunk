use memchunk::{OwnedChunker, DEFAULT_DELIMITERS, DEFAULT_TARGET_SIZE};
use wasm_bindgen::prelude::*;

/// Chunker splits text at delimiter boundaries.
///
/// @example
/// ```javascript
/// const chunker = new Chunker(textBytes, 4096, ".\n?");
/// let chunk;
/// while ((chunk = chunker.next()) !== undefined) {
///     console.log(chunk);
/// }
/// ```
#[wasm_bindgen]
pub struct Chunker {
    inner: OwnedChunker,
}

#[wasm_bindgen]
impl Chunker {
    /// Create a new Chunker.
    ///
    /// @param text - The text to chunk (as Uint8Array or string)
    /// @param size - Target chunk size in bytes (default: 4096)
    /// @param delimiters - Delimiter characters (default: "\n.?")
    #[wasm_bindgen(constructor)]
    pub fn new(text: &[u8], size: Option<usize>, delimiters: Option<String>) -> Chunker {
        let target_size = size.unwrap_or(DEFAULT_TARGET_SIZE);
        let delims = delimiters
            .map(|s| s.into_bytes())
            .unwrap_or_else(|| DEFAULT_DELIMITERS.to_vec());
        let inner = OwnedChunker::new(text.to_vec())
            .size(target_size)
            .delimiters(delims);
        Chunker { inner }
    }

    /// Get the next chunk, or undefined if exhausted.
    #[wasm_bindgen]
    pub fn next(&mut self) -> Option<Vec<u8>> {
        self.inner.next_chunk()
    }

    /// Reset the chunker to iterate from the beginning.
    #[wasm_bindgen]
    pub fn reset(&mut self) {
        self.inner.reset();
    }

    /// Collect all chunk offsets as a flat array [start1, end1, start2, end2, ...].
    /// This is faster than iterating as it makes a single WASM call.
    #[wasm_bindgen]
    pub fn collect_offsets(&mut self) -> Vec<usize> {
        self.inner
            .collect_offsets()
            .into_iter()
            .flat_map(|(start, end)| [start, end])
            .collect()
    }
}

/// Get the default target size (4096 bytes).
#[wasm_bindgen]
pub fn default_target_size() -> usize {
    DEFAULT_TARGET_SIZE
}

/// Get the default delimiters ("\n.?").
#[wasm_bindgen]
pub fn default_delimiters() -> Vec<u8> {
    DEFAULT_DELIMITERS.to_vec()
}

/// Fast chunking function that returns offsets in a single call.
/// Returns a flat array [start1, end1, start2, end2, ...].
/// Use this with subarray for maximum performance.
///
/// @example
/// ```javascript
/// const offsets = chunk_offsets(textBytes, 4096, ".\n?");
/// const chunks = [];
/// for (let i = 0; i < offsets.length; i += 2) {
///     chunks.push(textBytes.subarray(offsets[i], offsets[i + 1]));
/// }
/// ```
#[wasm_bindgen]
pub fn chunk_offsets(text: &[u8], size: Option<usize>, delimiters: Option<String>) -> Vec<usize> {
    let target_size = size.unwrap_or(DEFAULT_TARGET_SIZE);
    let delims = delimiters
        .map(|s| s.into_bytes())
        .unwrap_or_else(|| DEFAULT_DELIMITERS.to_vec());
    let mut chunker = OwnedChunker::new(text.to_vec())
        .size(target_size)
        .delimiters(delims);
    chunker
        .collect_offsets()
        .into_iter()
        .flat_map(|(start, end)| [start, end])
        .collect()
}
