use chunk::{
    DEFAULT_DELIMITERS, DEFAULT_TARGET_SIZE, IncludeDelim, OwnedChunker,
    find_merge_indices as rust_find_merge_indices, split_at_delimiters,
};
use wasm_bindgen::prelude::*;

/// Chunker splits text at delimiter boundaries.
///
/// @example Single-byte delimiters
/// ```javascript
/// const chunker = new Chunker(textBytes, 4096, ".\n?");
/// let chunk;
/// while ((chunk = chunker.next()) !== undefined) {
///     console.log(chunk);
/// }
/// ```
///
/// @example Multi-byte pattern (e.g., metaspace for SentencePiece)
/// ```javascript
/// const encoder = new TextEncoder();
/// const metaspace = encoder.encode("▁");
/// const chunker = Chunker.with_pattern(textBytes, 4096, metaspace, true);
/// ```
#[wasm_bindgen]
pub struct Chunker {
    inner: OwnedChunker,
}

#[wasm_bindgen]
impl Chunker {
    /// Create a new Chunker with single-byte delimiters.
    ///
    /// @param text - The text to chunk (as Uint8Array)
    /// @param size - Target chunk size in bytes (default: 4096)
    /// @param delimiters - Delimiter characters as string (default: "\n.?")
    /// @param prefix - Put delimiter at start of next chunk (default: false)
    /// @param consecutive - Split at START of consecutive runs (default: false)
    /// @param forward_fallback - Search forward if no delimiter in backward window (default: false)
    #[wasm_bindgen(constructor)]
    pub fn new(
        text: &[u8],
        size: Option<usize>,
        delimiters: Option<String>,
        prefix: Option<bool>,
        consecutive: Option<bool>,
        forward_fallback: Option<bool>,
    ) -> Chunker {
        let target_size = size.unwrap_or(DEFAULT_TARGET_SIZE);
        let delims = delimiters
            .map(|s| s.into_bytes())
            .unwrap_or_else(|| DEFAULT_DELIMITERS.to_vec());
        let mut inner = OwnedChunker::new(text.to_vec())
            .size(target_size)
            .delimiters(delims);
        if prefix.unwrap_or(false) {
            inner = inner.prefix();
        }
        if consecutive.unwrap_or(false) {
            inner = inner.consecutive();
        }
        if forward_fallback.unwrap_or(false) {
            inner = inner.forward_fallback();
        }
        Chunker { inner }
    }

    /// Create a new Chunker with a multi-byte pattern.
    ///
    /// @param text - The text to chunk (as Uint8Array)
    /// @param size - Target chunk size in bytes
    /// @param pattern - Multi-byte pattern to split on (as Uint8Array)
    /// @param prefix - Put pattern at start of next chunk (default: false)
    /// @param consecutive - Split at START of consecutive runs (default: false)
    /// @param forward_fallback - Search forward if no pattern in backward window (default: false)
    #[wasm_bindgen]
    pub fn with_pattern(
        text: &[u8],
        size: usize,
        pattern: &[u8],
        prefix: Option<bool>,
        consecutive: Option<bool>,
        forward_fallback: Option<bool>,
    ) -> Chunker {
        let mut inner = OwnedChunker::new(text.to_vec())
            .size(size)
            .pattern(pattern.to_vec());
        if prefix.unwrap_or(false) {
            inner = inner.prefix();
        }
        if consecutive.unwrap_or(false) {
            inner = inner.consecutive();
        }
        if forward_fallback.unwrap_or(false) {
            inner = inner.forward_fallback();
        }
        Chunker { inner }
    }

    /// Get the next chunk, or undefined if exhausted.
    #[wasm_bindgen]
    #[allow(clippy::should_implement_trait)]
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
/// @example Single-byte delimiters
/// ```javascript
/// const offsets = chunk_offsets(textBytes, 4096, ".\n?");
/// const chunks = [];
/// for (let i = 0; i < offsets.length; i += 2) {
///     chunks.push(textBytes.subarray(offsets[i], offsets[i + 1]));
/// }
/// ```
#[wasm_bindgen]
pub fn chunk_offsets(
    text: &[u8],
    size: Option<usize>,
    delimiters: Option<String>,
    prefix: Option<bool>,
    consecutive: Option<bool>,
    forward_fallback: Option<bool>,
) -> Vec<usize> {
    let target_size = size.unwrap_or(DEFAULT_TARGET_SIZE);
    let delims = delimiters
        .map(|s| s.into_bytes())
        .unwrap_or_else(|| DEFAULT_DELIMITERS.to_vec());
    let mut chunker = OwnedChunker::new(text.to_vec())
        .size(target_size)
        .delimiters(delims);
    if prefix.unwrap_or(false) {
        chunker = chunker.prefix();
    }
    if consecutive.unwrap_or(false) {
        chunker = chunker.consecutive();
    }
    if forward_fallback.unwrap_or(false) {
        chunker = chunker.forward_fallback();
    }
    chunker
        .collect_offsets()
        .into_iter()
        .flat_map(|(start, end)| [start, end])
        .collect()
}

/// Fast chunking function with multi-byte pattern support.
/// Returns a flat array [start1, end1, start2, end2, ...].
///
/// @example Multi-byte pattern (metaspace)
/// ```javascript
/// const encoder = new TextEncoder();
/// const metaspace = encoder.encode("▁");
/// const offsets = chunk_offsets_pattern(textBytes, 4096, metaspace, true, true, true);
/// ```
#[wasm_bindgen]
pub fn chunk_offsets_pattern(
    text: &[u8],
    size: usize,
    pattern: &[u8],
    prefix: Option<bool>,
    consecutive: Option<bool>,
    forward_fallback: Option<bool>,
) -> Vec<usize> {
    let mut chunker = OwnedChunker::new(text.to_vec())
        .size(size)
        .pattern(pattern.to_vec());
    if prefix.unwrap_or(false) {
        chunker = chunker.prefix();
    }
    if consecutive.unwrap_or(false) {
        chunker = chunker.consecutive();
    }
    if forward_fallback.unwrap_or(false) {
        chunker = chunker.forward_fallback();
    }
    chunker
        .collect_offsets()
        .into_iter()
        .flat_map(|(start, end)| [start, end])
        .collect()
}

/// Split text at every delimiter occurrence, returning offsets.
/// Unlike chunk_offsets which creates size-based chunks, this splits at
/// **every** delimiter occurrence.
///
/// Returns a flat array [start1, end1, start2, end2, ...].
///
/// @param text - The text to split (as Uint8Array)
/// @param delimiters - Delimiter characters as string (default: "\n.?")
/// @param include_delim - Where to attach delimiter: "prev" (default), "next", or "none"
/// @param min_chars - Minimum characters per segment (default: 0). Shorter segments are merged.
///
/// @example
/// ```javascript
/// const offsets = split_offsets(textBytes, ".", "prev", 0);
/// const segments = [];
/// for (let i = 0; i < offsets.length; i += 2) {
///     segments.push(textBytes.subarray(offsets[i], offsets[i + 1]));
/// }
/// // ["Hello.", " World.", " Test."]
/// ```
#[wasm_bindgen]
pub fn split_offsets(
    text: &[u8],
    delimiters: Option<String>,
    include_delim: Option<String>,
    min_chars: Option<usize>,
) -> Vec<usize> {
    let delims = delimiters
        .map(|s| s.into_bytes())
        .unwrap_or_else(|| DEFAULT_DELIMITERS.to_vec());

    let include = match include_delim.as_deref() {
        Some("next") => IncludeDelim::Next,
        Some("none") => IncludeDelim::None,
        _ => IncludeDelim::Prev, // default
    };

    let min = min_chars.unwrap_or(0);

    split_at_delimiters(text, &delims, include, min)
        .into_iter()
        .flat_map(|(start, end)| [start, end])
        .collect()
}

/// Find merge indices for combining segments within token limits.
///
/// Returns indices marking where to split segments into chunks that
/// respect the token budget. Use this to determine merge boundaries,
/// then join strings in JavaScript.
///
/// @param token_counts - Array of token counts for each segment
/// @param chunk_size - Maximum tokens per merged chunk
/// @returns Array of end indices (exclusive) for each chunk
///
/// @example
/// ```javascript
/// const tokenCounts = new Uint32Array([1, 1, 1, 1, 1, 1, 1]);
/// const indices = find_merge_indices(tokenCounts, 3);
/// // indices = [3, 6, 7]
/// // Use to slice: segments.slice(0, 3), segments.slice(3, 6), segments.slice(6, 7)
/// ```
#[wasm_bindgen]
pub fn find_merge_indices(token_counts: &[usize], chunk_size: usize) -> Vec<usize> {
    rust_find_merge_indices(token_counts, chunk_size)
}

/// Merge segments based on token counts, respecting chunk size limits.
/// Returns a flat array [endIndex1, tokenCount1, endIndex2, tokenCount2, ...].
///
/// This is the equivalent of Chonkie's `_merge_splits` function.
/// Used by RecursiveChunker to merge small segments into larger chunks
/// that fit within a token budget.
///
/// @param token_counts - Array of token counts for each segment
/// @param chunk_size - Maximum tokens per merged chunk
/// @param combine_whitespace - If true, adds +1 token per join for whitespace
/// @returns Flat array of [endIndex, tokenCount] pairs
///
/// @example
/// ```javascript
/// const result = merge_splits([1, 1, 1, 1, 1, 1, 1], 3, false);
/// // result = [3, 3, 6, 3, 7, 1]
/// // Meaning: chunk 0-3 has 3 tokens, chunk 3-6 has 3 tokens, chunk 6-7 has 1 token
/// ```
#[wasm_bindgen]
pub fn merge_splits(
    token_counts: &[usize],
    chunk_size: usize,
    combine_whitespace: Option<bool>,
) -> Vec<usize> {
    if token_counts.is_empty() {
        return vec![];
    }

    let combine_ws = combine_whitespace.unwrap_or(false);

    // Build cumulative token counts
    let mut cumulative: Vec<usize> = vec![0];
    let mut sum = 0usize;
    for &count in token_counts {
        sum += count + if combine_ws { 1 } else { 0 };
        cumulative.push(sum);
    }

    // Find merge boundaries
    let mut result = Vec::new();
    let mut current_index = 0;

    while current_index < token_counts.len() {
        let current_token_count = cumulative[current_index];
        let required_token_count = current_token_count + chunk_size;

        // Binary search for the insertion point
        let mut lo = current_index;
        let mut hi = cumulative.len();
        while lo < hi {
            let mid = (lo + hi) / 2;
            if cumulative[mid] < required_token_count {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }

        let mut index = lo.saturating_sub(1);
        index = index.min(token_counts.len());

        if index == current_index {
            index += 1;
        }

        // Calculate token count for this chunk
        let chunk_tokens = cumulative[index.min(token_counts.len())] - current_token_count;

        result.push(index);
        result.push(chunk_tokens);

        current_index = index;
    }

    result
}
