//! The fastest semantic text chunking library — up to 1TB/s chunking throughput.
//!
//! # Example
//!
//! ```
//! use memchunk::chunk;
//!
//! let text = b"Hello world. How are you? I'm fine.\nThanks for asking.";
//!
//! // With defaults (4KB chunks, split at \n . ?)
//! let chunks: Vec<&[u8]> = chunk(text).collect();
//!
//! // With custom size and delimiters
//! let chunks: Vec<&[u8]> = chunk(text).size(1024).delimiters(b"\n.?!").collect();
//!
//! // With multi-byte pattern (e.g., metaspace for SentencePiece tokenizers)
//! let metaspace = "▁".as_bytes(); // [0xE2, 0x96, 0x81]
//! let chunks: Vec<&[u8]> = chunk(b"Hello\xE2\x96\x81World").pattern(metaspace).collect();
//! ```

use memchr::memmem;

/// Default chunk target size (4KB).
pub const DEFAULT_TARGET_SIZE: usize = 4096;

/// Default delimiters: newline, period, question mark.
pub const DEFAULT_DELIMITERS: &[u8] = b"\n.?";

/// Find last delimiter in window using SIMD-accelerated memchr (1-3 delimiters)
/// or lookup table (4+ delimiters).
#[inline]
fn find_last_delimiter(
    window: &[u8],
    delimiters: &[u8],
    table: Option<&[bool; 256]>,
) -> Option<usize> {
    if let Some(t) = table {
        window.iter().rposition(|&b| t[b as usize])
    } else {
        match delimiters.len() {
            1 => memchr::memrchr(delimiters[0], window),
            2 => memchr::memrchr2(delimiters[0], delimiters[1], window),
            3 => memchr::memrchr3(delimiters[0], delimiters[1], delimiters[2], window),
            0 => None,
            _ => unreachable!(),
        }
    }
}

/// Find first delimiter in window using SIMD-accelerated memchr (1-3 delimiters)
/// or lookup table (4+ delimiters).
#[inline]
fn find_first_delimiter(
    window: &[u8],
    delimiters: &[u8],
    table: Option<&[bool; 256]>,
) -> Option<usize> {
    if let Some(t) = table {
        window.iter().position(|&b| t[b as usize])
    } else {
        match delimiters.len() {
            1 => memchr::memchr(delimiters[0], window),
            2 => memchr::memchr2(delimiters[0], delimiters[1], window),
            3 => memchr::memchr3(delimiters[0], delimiters[1], delimiters[2], window),
            0 => None,
            _ => unreachable!(),
        }
    }
}

/// Find delimiter boundary that is the START of a consecutive run.
///
/// Searches backward from `target_end`, then forward if `forward_fallback` is true.
/// When `consecutive` is true, returns position of a delimiter that is NOT preceded
/// by the same delimiter byte.
///
/// Returns positions > start (never returns start itself, as that wouldn't make progress).
fn find_delimiter_boundary(
    text: &[u8],
    delimiters: &[u8],
    table: Option<&[bool; 256]>,
    start: usize,
    target_end: usize,
    consecutive: bool,
    forward_fallback: bool,
) -> Option<usize> {
    if delimiters.is_empty() || start >= text.len() {
        return None;
    }

    let target_end = target_end.min(text.len());
    let window = &text[start..target_end];

    // Backward search
    if consecutive {
        // Find last delimiter that is START of consecutive run (same delimiter)
        let mut search_end = window.len();
        while search_end > 0 {
            let rel_pos = find_last_delimiter(&window[..search_end], delimiters, table);

            if let Some(rel_pos) = rel_pos {
                let abs_pos = start + rel_pos;
                let delim_byte = text[abs_pos];
                // Check if this is START of consecutive run (not preceded by same delimiter)
                if abs_pos == 0 || text[abs_pos - 1] != delim_byte {
                    // Found valid boundary, but skip if it equals start (no progress)
                    if abs_pos > start {
                        return Some(abs_pos);
                    }
                    // We've traced back to start - chunk is full of consecutive delimiters
                    // Fall through to forward fallback
                    break;
                }
                // In middle of run, search earlier
                search_end = rel_pos;
            } else {
                break;
            }
        }
    } else {
        // Simple case: just find last occurrence (but not at start position)
        let rel_pos = find_last_delimiter(window, delimiters, table);
        if let Some(rel_pos) = rel_pos {
            let abs_pos = start + rel_pos;
            if abs_pos > start {
                return Some(abs_pos);
            }
        }
    }

    // Forward fallback search - find next boundary after target_end
    if forward_fallback {
        // First, determine where to start searching forward.
        // If we're at the start of a consecutive run, skip past the entire run.
        let mut forward_from = target_end;

        if consecutive && start < text.len() {
            // Check if start is at a delimiter
            let is_delim_at_start = if let Some(t) = table {
                t[text[start] as usize]
            } else {
                delimiters.contains(&text[start])
            };

            if is_delim_at_start {
                // We're at a delimiter at `start`. Skip all consecutive same delimiters.
                let delim_byte = text[start];
                let mut pos = start;
                while pos < text.len() && text[pos] == delim_byte {
                    pos += 1;
                }
                // Start searching from end of consecutive run, but not before target_end
                forward_from = forward_from.max(pos);
            }
        }

        if forward_from < text.len() {
            let forward_window = &text[forward_from..];

            if consecutive {
                // Find first delimiter that is START of consecutive run
                let mut search_start = 0;
                while search_start < forward_window.len() {
                    let rel_pos =
                        find_first_delimiter(&forward_window[search_start..], delimiters, table);

                    if let Some(rel_pos) = rel_pos {
                        let abs_pos = forward_from + search_start + rel_pos;
                        let delim_byte = text[abs_pos];
                        if abs_pos == 0 || text[abs_pos - 1] != delim_byte {
                            return Some(abs_pos);
                        }
                        // In middle of run, search later
                        search_start += rel_pos + 1;
                    } else {
                        break;
                    }
                }
            } else {
                let rel_pos = find_first_delimiter(forward_window, delimiters, table);
                if let Some(rel_pos) = rel_pos {
                    return Some(forward_from + rel_pos);
                }
            }
        }

        // No delimiter found forward. Return text.len() to include all remaining
        // text in one chunk. This avoids O(n²) behavior from repeatedly searching
        // forward through the entire remaining text on each iteration.
        return Some(text.len());
    }

    None
}

/// Build lookup table for 4+ delimiters.
#[inline]
fn build_table(delimiters: &[u8]) -> Option<[bool; 256]> {
    if delimiters.len() > 3 {
        let mut t = [false; 256];
        for &b in delimiters {
            t[b as usize] = true;
        }
        Some(t)
    } else {
        None
    }
}

/// Compute the split position given the current state.
///
/// Returns the position to split at, handling pattern mode vs delimiter mode,
/// prefix vs suffix mode, and the special `text.len()` signal.
#[inline]
#[allow(clippy::too_many_arguments)]
fn compute_split_at(
    text: &[u8],
    pos: usize,
    end: usize,
    pattern: Option<&[u8]>,
    delimiters: &[u8],
    table: Option<&[bool; 256]>,
    prefix_mode: bool,
    consecutive: bool,
    forward_fallback: bool,
) -> usize {
    if let Some(pattern) = pattern {
        // Multi-byte pattern mode
        match find_pattern_boundary(text, pattern, pos, end, consecutive, forward_fallback) {
            Some(found_pos) => {
                if found_pos == text.len() {
                    // Special case: text.len() means "take all remaining"
                    found_pos
                } else if prefix_mode {
                    // Split BEFORE pattern (pattern goes to next chunk)
                    if found_pos == pos { end } else { found_pos }
                } else {
                    // Split AFTER pattern (pattern stays with current chunk)
                    found_pos + pattern.len()
                }
            }
            None => end, // No pattern found, hard split at target
        }
    } else {
        // Single-byte delimiters mode
        match find_delimiter_boundary(
            text,
            delimiters,
            table,
            pos,
            end,
            consecutive,
            forward_fallback,
        ) {
            Some(found_pos) => {
                if found_pos == text.len() {
                    // Special case: text.len() means "take all remaining"
                    found_pos
                } else if prefix_mode {
                    // Split BEFORE delimiter (delimiter goes to next chunk)
                    if found_pos == pos { end } else { found_pos }
                } else {
                    // Split AFTER delimiter (delimiter stays with current chunk)
                    found_pos + 1
                }
            }
            None => end, // No delimiter found, hard split at target
        }
    }
}

/// Find pattern boundary that is the START of a consecutive run.
///
/// Searches backward from `target_end`, then forward if `forward_fallback` is true.
/// When `consecutive` is true, returns position of a pattern that is NOT preceded
/// by another instance of the same pattern.
///
/// Returns positions > start (never returns start itself, as that wouldn't make progress).
fn find_pattern_boundary(
    text: &[u8],
    pattern: &[u8],
    start: usize,
    target_end: usize,
    consecutive: bool,
    forward_fallback: bool,
) -> Option<usize> {
    let plen = pattern.len();
    if plen == 0 || start >= text.len() {
        return None;
    }

    let target_end = target_end.min(text.len());
    let window = &text[start..target_end];

    // Backward search
    if consecutive {
        // Find last pattern that is START of consecutive run
        let mut search_end = window.len();
        while search_end > 0 {
            let rel_pos = if plen == 1 {
                memchr::memrchr(pattern[0], &window[..search_end])
            } else {
                memmem::rfind(&window[..search_end], pattern)
            };

            if let Some(rel_pos) = rel_pos {
                let abs_pos = start + rel_pos;
                // Check if this is START of consecutive run (not preceded by same pattern)
                if abs_pos < plen || &text[abs_pos - plen..abs_pos] != pattern {
                    // Found valid boundary, but skip if it equals start (no progress)
                    if abs_pos > start {
                        return Some(abs_pos);
                    }
                    // We've traced back to start - chunk is full of consecutive patterns
                    // Fall through to forward fallback
                    break;
                }
                // In middle of run, search earlier
                search_end = rel_pos;
            } else {
                break;
            }
        }
    } else {
        // Simple case: just find last occurrence (but not at start position)
        let rel_pos = if plen == 1 {
            memchr::memrchr(pattern[0], window)
        } else {
            memmem::rfind(window, pattern)
        };
        if let Some(rel_pos) = rel_pos {
            let abs_pos = start + rel_pos;
            if abs_pos > start {
                return Some(abs_pos);
            }
        }
    }

    // Forward fallback search - find next boundary after target_end
    if forward_fallback {
        // First, determine where to start searching forward.
        // If we're at the start of a consecutive run, skip past the entire run.
        let mut forward_from = target_end;

        if consecutive && start + plen <= text.len() && &text[start..start + plen] == pattern {
            // We're at a pattern at `start`. Skip all consecutive patterns.
            let mut pos = start;
            while pos + plen <= text.len() && &text[pos..pos + plen] == pattern {
                pos += plen;
            }
            // Start searching from end of consecutive run, but not before target_end
            forward_from = forward_from.max(pos);
        }

        if forward_from < text.len() {
            let forward_window = &text[forward_from..];

            if consecutive {
                // Find first pattern that is START of consecutive run
                let mut search_start = 0;
                while search_start < forward_window.len() {
                    let rel_pos = if plen == 1 {
                        memchr::memchr(pattern[0], &forward_window[search_start..])
                    } else {
                        memmem::find(&forward_window[search_start..], pattern)
                    };

                    if let Some(rel_pos) = rel_pos {
                        let abs_pos = forward_from + search_start + rel_pos;
                        if abs_pos < plen || &text[abs_pos - plen..abs_pos] != pattern {
                            return Some(abs_pos);
                        }
                        // In middle of run, search later
                        search_start += rel_pos + 1;
                    } else {
                        break;
                    }
                }
            } else {
                let rel_pos = if plen == 1 {
                    memchr::memchr(pattern[0], forward_window)
                } else {
                    memmem::find(forward_window, pattern)
                };
                if let Some(rel_pos) = rel_pos {
                    return Some(forward_from + rel_pos);
                }
            }
        }

        // No pattern found forward. Return text.len() to include all remaining
        // text in one chunk. This avoids O(n²) behavior from repeatedly searching
        // forward through the entire remaining text on each iteration.
        return Some(text.len());
    }

    None
}

/// Chunk text at delimiter boundaries.
///
/// Returns a builder that can be configured with `.size()` and `.delimiters()`,
/// or used directly as an iterator with defaults (4KB chunks, `\n.?` delimiters).
///
/// - For 1-3 delimiters: uses SIMD-accelerated memchr
/// - For 4+ delimiters: uses lookup table
///
/// # Example
///
/// ```
/// use memchunk::chunk;
///
/// let text = b"First sentence. Second sentence. Third sentence.";
///
/// // With defaults
/// let chunks: Vec<_> = chunk(text).collect();
///
/// // With custom size
/// let chunks: Vec<_> = chunk(text).size(1024).collect();
///
/// // With custom delimiters
/// let chunks: Vec<_> = chunk(text).delimiters(b"\n.?!").collect();
///
/// // With both
/// let chunks: Vec<_> = chunk(text).size(8192).delimiters(b"\n").collect();
/// ```
pub fn chunk(text: &[u8]) -> Chunker<'_> {
    Chunker::new(text)
}

/// Chunker splits text at delimiter boundaries.
///
/// Created via [`chunk()`], can be configured with `.size()` and `.delimiters()`.
/// For multi-byte delimiters, use `.pattern()` instead.
pub struct Chunker<'a> {
    text: &'a [u8],
    target_size: usize,
    delimiters: &'a [u8],
    pattern: Option<&'a [u8]>,
    pos: usize,
    table: Option<[bool; 256]>,
    initialized: bool,
    prefix_mode: bool,
    /// When true, find the START of consecutive pattern runs (not middle)
    consecutive: bool,
    /// When true, search forward if no pattern found in backward window
    forward_fallback: bool,
}

impl<'a> Chunker<'a> {
    fn new(text: &'a [u8]) -> Self {
        Self {
            text,
            target_size: DEFAULT_TARGET_SIZE,
            delimiters: DEFAULT_DELIMITERS,
            pattern: None,
            pos: 0,
            table: None,
            initialized: false,
            prefix_mode: false,
            consecutive: false,
            forward_fallback: false,
        }
    }

    /// Set the target chunk size in bytes.
    pub fn size(mut self, size: usize) -> Self {
        self.target_size = size;
        self
    }

    /// Set single-byte delimiters to split on.
    ///
    /// Mutually exclusive with `pattern()` - last one set wins.
    pub fn delimiters(mut self, delimiters: &'a [u8]) -> Self {
        self.delimiters = delimiters;
        self.pattern = None; // Clear pattern mode
        self
    }

    /// Set a multi-byte pattern to split on.
    ///
    /// Use this for multi-byte delimiters like UTF-8 characters (e.g., metaspace `▁`).
    /// Mutually exclusive with `delimiters()` - last one set wins.
    ///
    /// ```
    /// use memchunk::chunk;
    /// let metaspace = "▁".as_bytes(); // [0xE2, 0x96, 0x81]
    /// let chunks: Vec<_> = chunk(b"Hello\xE2\x96\x81World\xE2\x96\x81Test")
    ///     .size(15)
    ///     .pattern(metaspace)
    ///     .prefix()
    ///     .collect();
    /// assert_eq!(chunks[0], b"Hello");
    /// assert_eq!(chunks[1], b"\xE2\x96\x81World\xE2\x96\x81Test");
    /// ```
    pub fn pattern(mut self, pattern: &'a [u8]) -> Self {
        self.pattern = Some(pattern);
        self.delimiters = &[]; // Clear single-byte delimiters
        self
    }

    /// Put delimiter at the start of the next chunk (prefix mode).
    ///
    /// ```
    /// use memchunk::chunk;
    /// let chunks: Vec<_> = chunk(b"Hello World").size(8).delimiters(b" ").prefix().collect();
    /// assert_eq!(chunks, vec![b"Hello".as_slice(), b" World".as_slice()]);
    /// ```
    pub fn prefix(mut self) -> Self {
        self.prefix_mode = true;
        self
    }

    /// Put delimiter at the end of the current chunk (suffix mode, default).
    ///
    /// ```
    /// use memchunk::chunk;
    /// let chunks: Vec<_> = chunk(b"Hello World").size(8).delimiters(b" ").suffix().collect();
    /// assert_eq!(chunks, vec![b"Hello ".as_slice(), b"World".as_slice()]);
    /// ```
    pub fn suffix(mut self) -> Self {
        self.prefix_mode = false;
        self
    }

    /// Enable consecutive delimiter/pattern handling.
    ///
    /// When splitting, ensures we split at the START of a consecutive run
    /// of the same delimiter/pattern, not in the middle. For example:
    /// - With pattern: "word▁▁▁next" splits as ["word"]["▁▁▁next"]
    /// - With delimiter: "word\n\n\nnext" splits as ["word"]["\\n\\n\\nnext"]
    ///
    /// This is useful for patterns that can merge (like BPE tokenization)
    /// or when consecutive delimiters have semantic meaning (like `\n\n`
    /// for paragraph breaks).
    ///
    /// Works with both `.pattern()` and `.delimiters()`.
    ///
    /// ```
    /// use memchunk::chunk;
    ///
    /// // With pattern
    /// let text = b"word\xE2\x96\x81\xE2\x96\x81\xE2\x96\x81next"; // word▁▁▁next
    /// let metaspace = b"\xE2\x96\x81";
    /// let chunks: Vec<_> = chunk(text)
    ///     .pattern(metaspace)
    ///     .size(10)
    ///     .prefix()
    ///     .consecutive()
    ///     .collect();
    /// assert_eq!(chunks[0], b"word");
    ///
    /// // With delimiters
    /// let text = b"Hello\n\n\nWorld";
    /// let chunks: Vec<_> = chunk(text)
    ///     .delimiters(b"\n")
    ///     .size(8)
    ///     .prefix()
    ///     .consecutive()
    ///     .collect();
    /// assert_eq!(chunks[0], b"Hello");
    /// assert_eq!(chunks[1], b"\n\n\nWorld");
    /// ```
    pub fn consecutive(mut self) -> Self {
        self.consecutive = true;
        self
    }

    /// Enable forward fallback search.
    ///
    /// When no delimiter/pattern is found in the backward search window,
    /// search forward from target_end instead of doing a hard split.
    ///
    /// This ensures splits always occur at semantic boundaries when possible,
    /// even if the nearest boundary is past the target size.
    ///
    /// Works with both `.pattern()` and `.delimiters()`.
    ///
    /// ```
    /// use memchunk::chunk;
    ///
    /// // With pattern
    /// let text = b"verylongword\xE2\x96\x81short"; // verylongword▁short
    /// let metaspace = b"\xE2\x96\x81";
    /// let chunks: Vec<_> = chunk(text)
    ///     .pattern(metaspace)
    ///     .size(6)
    ///     .prefix()
    ///     .forward_fallback()
    ///     .collect();
    /// // Without forward_fallback: hard split at position 6
    /// // With forward_fallback: finds ▁ at position 12
    /// assert_eq!(chunks[0], b"verylongword");
    ///
    /// // With delimiters
    /// let text = b"verylongword next";
    /// let chunks: Vec<_> = chunk(text)
    ///     .delimiters(b" ")
    ///     .size(6)
    ///     .prefix()
    ///     .forward_fallback()
    ///     .collect();
    /// assert_eq!(chunks[0], b"verylongword");
    /// assert_eq!(chunks[1], b" next");
    /// ```
    pub fn forward_fallback(mut self) -> Self {
        self.forward_fallback = true;
        self
    }

    /// Initialize lookup table if needed (called on first iteration).
    fn init(&mut self) {
        if !self.initialized {
            self.table = build_table(self.delimiters);
            self.initialized = true;
        }
    }
}

impl<'a> Iterator for Chunker<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        self.init();

        if self.pos >= self.text.len() {
            return None;
        }

        let remaining = self.text.len() - self.pos;

        // Last chunk - return remainder
        if remaining <= self.target_size {
            let chunk = &self.text[self.pos..];
            self.pos = self.text.len();
            return Some(chunk);
        }

        let end = self.pos + self.target_size;

        let split_at = compute_split_at(
            self.text,
            self.pos,
            end,
            self.pattern,
            self.delimiters,
            self.table.as_ref(),
            self.prefix_mode,
            self.consecutive,
            self.forward_fallback,
        );

        let chunk = &self.text[self.pos..split_at];
        self.pos = split_at;
        Some(chunk)
    }
}

/// Owned chunker for FFI bindings (Python, WASM).
///
/// Unlike [`Chunker`], this owns its data and returns owned chunks.
/// Use this when you need to cross FFI boundaries where lifetimes can't be tracked.
///
/// # Example
///
/// ```
/// use memchunk::OwnedChunker;
///
/// let text = b"Hello world. How are you?".to_vec();
/// let mut chunker = OwnedChunker::new(text)
///     .size(15)
///     .delimiters(b"\n.?".to_vec());
///
/// while let Some(chunk) = chunker.next_chunk() {
///     println!("{:?}", chunk);
/// }
/// ```
pub struct OwnedChunker {
    text: Vec<u8>,
    target_size: usize,
    delimiters: Vec<u8>,
    pattern: Option<Vec<u8>>,
    pos: usize,
    table: Option<[bool; 256]>,
    initialized: bool,
    prefix_mode: bool,
    consecutive: bool,
    forward_fallback: bool,
}

impl OwnedChunker {
    /// Create a new owned chunker with the given text.
    pub fn new(text: Vec<u8>) -> Self {
        Self {
            text,
            target_size: DEFAULT_TARGET_SIZE,
            delimiters: DEFAULT_DELIMITERS.to_vec(),
            pattern: None,
            pos: 0,
            table: None,
            initialized: false,
            prefix_mode: false,
            consecutive: false,
            forward_fallback: false,
        }
    }

    /// Set the target chunk size in bytes.
    pub fn size(mut self, size: usize) -> Self {
        self.target_size = size;
        self
    }

    /// Set single-byte delimiters to split on.
    ///
    /// Mutually exclusive with `pattern()` - last one set wins.
    pub fn delimiters(mut self, delimiters: Vec<u8>) -> Self {
        self.delimiters = delimiters;
        self.pattern = None; // Clear pattern mode
        self
    }

    /// Set a multi-byte pattern to split on.
    ///
    /// Use this for multi-byte delimiters like UTF-8 characters (e.g., metaspace `▁`).
    /// Mutually exclusive with `delimiters()` - last one set wins.
    pub fn pattern(mut self, pattern: Vec<u8>) -> Self {
        self.pattern = Some(pattern);
        self.delimiters = vec![]; // Clear single-byte delimiters
        self
    }

    /// Put delimiter at the start of the next chunk (prefix mode).
    pub fn prefix(mut self) -> Self {
        self.prefix_mode = true;
        self
    }

    /// Put delimiter at the end of the current chunk (suffix mode, default).
    pub fn suffix(mut self) -> Self {
        self.prefix_mode = false;
        self
    }

    /// Enable consecutive delimiter/pattern handling.
    ///
    /// When splitting, ensures we split at the START of a consecutive run
    /// of the same delimiter/pattern, not in the middle.
    /// Works with both `.pattern()` and `.delimiters()`.
    pub fn consecutive(mut self) -> Self {
        self.consecutive = true;
        self
    }

    /// Enable forward fallback search.
    ///
    /// When no delimiter/pattern is found in the backward search window,
    /// search forward from target_end instead of doing a hard split.
    /// Works with both `.pattern()` and `.delimiters()`.
    pub fn forward_fallback(mut self) -> Self {
        self.forward_fallback = true;
        self
    }

    /// Initialize lookup table if needed.
    fn init(&mut self) {
        if !self.initialized {
            self.table = build_table(&self.delimiters);
            self.initialized = true;
        }
    }

    /// Get the next chunk, or None if exhausted.
    pub fn next_chunk(&mut self) -> Option<Vec<u8>> {
        self.init();

        if self.pos >= self.text.len() {
            return None;
        }

        let remaining = self.text.len() - self.pos;

        // Last chunk - return remainder
        if remaining <= self.target_size {
            let chunk = self.text[self.pos..].to_vec();
            self.pos = self.text.len();
            return Some(chunk);
        }

        let end = self.pos + self.target_size;

        let split_at = compute_split_at(
            &self.text,
            self.pos,
            end,
            self.pattern.as_deref(),
            &self.delimiters,
            self.table.as_ref(),
            self.prefix_mode,
            self.consecutive,
            self.forward_fallback,
        );

        let chunk = self.text[self.pos..split_at].to_vec();
        self.pos = split_at;
        Some(chunk)
    }

    /// Reset the chunker to start from the beginning.
    pub fn reset(&mut self) {
        self.pos = 0;
    }

    /// Get a reference to the underlying text.
    pub fn text(&self) -> &[u8] {
        &self.text
    }

    /// Collect all chunk offsets as (start, end) pairs.
    /// This is more efficient for FFI as it returns all offsets in one call.
    pub fn collect_offsets(&mut self) -> Vec<(usize, usize)> {
        self.init();

        let mut offsets = Vec::new();
        let mut pos = 0;

        while pos < self.text.len() {
            let remaining = self.text.len() - pos;

            if remaining <= self.target_size {
                offsets.push((pos, self.text.len()));
                break;
            }

            let end = pos + self.target_size;

            let split_at = compute_split_at(
                &self.text,
                pos,
                end,
                self.pattern.as_deref(),
                &self.delimiters,
                self.table.as_ref(),
                self.prefix_mode,
                self.consecutive,
                self.forward_fallback,
            );

            offsets.push((pos, split_at));
            pos = split_at;
        }

        offsets
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_chunking() {
        let text = b"Hello. World. Test.";
        let chunks: Vec<_> = chunk(text).size(10).delimiters(b".").collect();
        // "Hello." (6) + " World." (7) + " Test." (6) = 19
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0], b"Hello.");
        assert_eq!(chunks[1], b" World.");
        assert_eq!(chunks[2], b" Test.");
    }

    #[test]
    fn test_newline_delimiter() {
        let text = b"Line one\nLine two\nLine three";
        let chunks: Vec<_> = chunk(text).size(15).delimiters(b"\n").collect();
        assert_eq!(chunks[0], b"Line one\n");
        assert_eq!(chunks[1], b"Line two\n");
        assert_eq!(chunks[2], b"Line three");
    }

    #[test]
    fn test_multiple_delimiters() {
        let text = b"Hello? World. Yes!";
        let chunks: Vec<_> = chunk(text).size(10).delimiters(b".?!").collect();
        assert_eq!(chunks[0], b"Hello?");
    }

    #[test]
    fn test_four_delimiters_uses_table() {
        let text = b"A. B? C! D; E";
        let chunks: Vec<_> = chunk(text).size(5).delimiters(b".?!;").collect();
        assert!(chunks.len() >= 2);
    }

    #[test]
    fn test_no_delimiter_hard_split() {
        let text = b"abcdefghij";
        let chunks: Vec<_> = chunk(text).size(5).delimiters(b".").collect();
        assert_eq!(chunks[0], b"abcde");
        assert_eq!(chunks[1], b"fghij");
    }

    #[test]
    fn test_empty_text() {
        let text = b"";
        let chunks: Vec<_> = chunk(text).size(10).delimiters(b".").collect();
        assert_eq!(chunks.len(), 0);
    }

    #[test]
    fn test_text_smaller_than_target() {
        let text = b"Small";
        let chunks: Vec<_> = chunk(text).size(100).delimiters(b".").collect();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], b"Small");
    }

    #[test]
    fn test_total_bytes_preserved() {
        let text = b"The quick brown fox jumps over the lazy dog. How vexingly quick!";
        let chunks: Vec<_> = chunk(text).size(20).delimiters(b"\n.?!").collect();
        let total: usize = chunks.iter().map(|c| c.len()).sum();
        assert_eq!(total, text.len());
    }

    #[test]
    fn test_defaults() {
        let text = b"Hello world. This is a test.";
        // Should work with just defaults
        let chunks: Vec<_> = chunk(text).collect();
        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_suffix_mode_default() {
        let text = b"Hello World Test";
        let chunks: Vec<_> = chunk(text).size(8).delimiters(b" ").collect();
        assert_eq!(chunks[0], b"Hello ");
        assert_eq!(chunks[1], b"World ");
        assert_eq!(chunks[2], b"Test");
    }

    #[test]
    fn test_prefix_mode() {
        let text = b"Hello World Test";
        let chunks: Vec<_> = chunk(text).size(8).delimiters(b" ").prefix().collect();
        assert_eq!(chunks[0], b"Hello");
        assert_eq!(chunks[1], b" World");
        assert_eq!(chunks[2], b" Test");
    }

    #[test]
    fn test_suffix_mode_explicit() {
        let text = b"Hello World Test";
        let chunks: Vec<_> = chunk(text).size(8).delimiters(b" ").suffix().collect();
        assert_eq!(chunks[0], b"Hello ");
        assert_eq!(chunks[1], b"World ");
        assert_eq!(chunks[2], b"Test");
    }

    #[test]
    fn test_prefix_preserves_total_bytes() {
        let text = b"Hello World Test More Words Here";
        let chunks: Vec<_> = chunk(text).size(10).delimiters(b" ").prefix().collect();
        let total: usize = chunks.iter().map(|c| c.len()).sum();
        assert_eq!(total, text.len());
    }
}

#[test]
fn test_consecutive_delimiters() {
    // Test with consecutive newlines
    let text = b"Hello\n\nWorld";

    // Suffix mode (default)
    let chunks: Vec<_> = chunk(text).size(8).delimiters(b"\n").collect();
    let total: usize = chunks.iter().map(|c| c.len()).sum();
    assert_eq!(total, text.len());

    // Prefix mode
    let chunks: Vec<_> = chunk(text).size(8).delimiters(b"\n").prefix().collect();
    let total: usize = chunks.iter().map(|c| c.len()).sum();
    assert_eq!(total, text.len());

    // Smaller target to force more splits
    let chunks: Vec<_> = chunk(text).size(4).delimiters(b"\n").prefix().collect();
    let total: usize = chunks.iter().map(|c| c.len()).sum();
    assert_eq!(total, text.len());
}

#[test]
fn test_prefix_mode_delimiter_at_window_start() {
    // This was causing an infinite loop before the fix.
    // When window starts with delimiter in prefix mode, we now hard split at target size.
    let text = b"Hello world";

    // With size=5: "Hello" (no delim) → "Hello", then " worl" (delim at 0) → hard split
    let chunks: Vec<_> = chunk(text).size(5).delimiters(b" ").prefix().collect();

    // Should not hang and should preserve all bytes
    let total: usize = chunks.iter().map(|c| c.len()).sum();
    assert_eq!(total, text.len());

    // Hard split behavior: ["Hello", " worl", "d"]
    assert_eq!(chunks[0], b"Hello");
    assert_eq!(chunks[1], b" worl");
    assert_eq!(chunks[2], b"d");
}

#[test]
fn test_prefix_mode_small_chunks() {
    // More edge cases with small chunks
    let text = b"a b c d e";

    let chunks: Vec<_> = chunk(text).size(2).delimiters(b" ").prefix().collect();
    let total: usize = chunks.iter().map(|c| c.len()).sum();
    assert_eq!(total, text.len());

    // Each chunk should be non-empty
    for chunk in &chunks {
        assert!(!chunk.is_empty(), "Found empty chunk!");
    }
}

// ============ Multi-byte pattern tests ============

#[test]
fn test_pattern_metaspace_suffix() {
    // Metaspace: ▁ = [0xE2, 0x96, 0x81]
    let metaspace = "▁".as_bytes();
    let text = "Hello▁World▁Test".as_bytes();

    // Suffix mode (default): metaspace at end of chunk
    let chunks: Vec<_> = chunk(text).size(15).pattern(metaspace).collect();

    // First chunk: "Hello▁" (8 bytes)
    assert_eq!(chunks[0], "Hello▁".as_bytes());
    // Remaining: "World▁Test"
    assert_eq!(chunks[1], "World▁Test".as_bytes());

    // Total bytes preserved
    let total: usize = chunks.iter().map(|c| c.len()).sum();
    assert_eq!(total, text.len());
}

#[test]
fn test_pattern_metaspace_prefix() {
    let metaspace = "▁".as_bytes();
    let text = "Hello▁World▁Test".as_bytes();

    // Prefix mode: metaspace at start of next chunk
    let chunks: Vec<_> = chunk(text).size(15).pattern(metaspace).prefix().collect();

    // First chunk: "Hello" (5 bytes)
    assert_eq!(chunks[0], "Hello".as_bytes());
    // Second chunk: "▁World▁Test" (remaining)
    assert_eq!(chunks[1], "▁World▁Test".as_bytes());

    let total: usize = chunks.iter().map(|c| c.len()).sum();
    assert_eq!(total, text.len());
}

#[test]
fn test_pattern_preserves_bytes() {
    let metaspace = "▁".as_bytes();
    let text = "The▁quick▁brown▁fox▁jumps▁over▁the▁lazy▁dog".as_bytes();

    // Suffix mode
    let chunks: Vec<_> = chunk(text).size(20).pattern(metaspace).collect();
    let total: usize = chunks.iter().map(|c| c.len()).sum();
    assert_eq!(total, text.len());

    // Prefix mode
    let chunks: Vec<_> = chunk(text).size(20).pattern(metaspace).prefix().collect();
    let total: usize = chunks.iter().map(|c| c.len()).sum();
    assert_eq!(total, text.len());
}

#[test]
fn test_pattern_no_match_hard_split() {
    let pattern = b"XYZ";
    let text = b"abcdefghijklmnop";

    let chunks: Vec<_> = chunk(text).size(5).pattern(pattern).collect();
    assert_eq!(chunks[0], b"abcde");
    assert_eq!(chunks[1], b"fghij");
    assert_eq!(chunks[2], b"klmno");
    assert_eq!(chunks[3], b"p");
}

#[test]
fn test_pattern_single_byte_optimization() {
    // Single-byte pattern should work (uses memrchr optimization)
    let text = b"Hello World Test";
    let chunks: Vec<_> = chunk(text).size(8).pattern(b" ").prefix().collect();

    assert_eq!(chunks[0], b"Hello");
    assert_eq!(chunks[1], b" World");
    assert_eq!(chunks[2], b" Test");
}

#[test]
fn test_pattern_at_window_start_prefix() {
    // Edge case: pattern at position 0 in prefix mode should hard split
    let metaspace = "▁".as_bytes();
    let text = "Hello▁▁World".as_bytes(); // Two consecutive metaspaces

    let chunks: Vec<_> = chunk(text).size(10).pattern(metaspace).prefix().collect();

    // Should not hang and should preserve all bytes
    let total: usize = chunks.iter().map(|c| c.len()).sum();
    assert_eq!(total, text.len());

    // Each chunk should be non-empty
    for c in &chunks {
        assert!(!c.is_empty(), "Found empty chunk!");
    }
}

#[test]
fn test_pattern_empty() {
    // Empty pattern should result in hard splits only
    let text = b"Hello World Test";
    let chunks: Vec<_> = chunk(text).size(5).pattern(b"").collect();

    assert_eq!(chunks[0], b"Hello");
    assert_eq!(chunks[1], b" Worl");
    assert_eq!(chunks[2], b"d Tes");
    assert_eq!(chunks[3], b"t");
}

#[test]
fn test_owned_chunker_pattern() {
    let metaspace = "▁".as_bytes();
    let text = "Hello▁World▁Test".as_bytes().to_vec();

    let mut chunker = OwnedChunker::new(text.clone())
        .size(15)
        .pattern(metaspace.to_vec())
        .prefix();

    let mut chunks = Vec::new();
    while let Some(c) = chunker.next_chunk() {
        chunks.push(c);
    }

    assert_eq!(chunks[0], "Hello".as_bytes());
    assert_eq!(chunks[1], "▁World▁Test".as_bytes());

    let total: usize = chunks.iter().map(|c| c.len()).sum();
    assert_eq!(total, text.len());
}

#[test]
fn test_owned_chunker_pattern_collect_offsets() {
    let metaspace = "▁".as_bytes();
    let text = "Hello▁World▁Test".as_bytes().to_vec();

    let mut chunker = OwnedChunker::new(text.clone())
        .size(15)
        .pattern(metaspace.to_vec())
        .prefix();

    let offsets = chunker.collect_offsets();

    // Verify offsets
    assert_eq!(offsets[0], (0, 5)); // "Hello"
    assert_eq!(offsets[1], (5, text.len())); // "▁World▁Test"

    // Verify slicing produces correct chunks
    assert_eq!(&text[offsets[0].0..offsets[0].1], "Hello".as_bytes());
    assert_eq!(&text[offsets[1].0..offsets[1].1], "▁World▁Test".as_bytes());
}

// ============ Consecutive and Forward Fallback Tests ============

#[test]
fn test_consecutive_pattern_basic() {
    // word▁▁▁next - should split at START of ▁▁▁, not in the middle
    let metaspace = b"\xE2\x96\x81";
    let text = b"word\xE2\x96\x81\xE2\x96\x81\xE2\x96\x81next"; // word▁▁▁next

    // Without consecutive: may split in middle of ▁▁▁
    let chunks_default: Vec<_> = chunk(text).pattern(metaspace).size(10).prefix().collect();

    // With consecutive: splits at START of ▁▁▁
    let chunks_consecutive: Vec<_> = chunk(text)
        .pattern(metaspace)
        .size(10)
        .prefix()
        .consecutive()
        .collect();

    // Both should preserve all bytes
    let total_default: usize = chunks_default.iter().map(|c| c.len()).sum();
    let total_consecutive: usize = chunks_consecutive.iter().map(|c| c.len()).sum();
    assert_eq!(total_default, text.len());
    assert_eq!(total_consecutive, text.len());

    // Consecutive mode should split at "word" | "▁▁▁next"
    assert_eq!(chunks_consecutive[0], b"word");
    assert!(chunks_consecutive[1].starts_with(metaspace));
}

#[test]
fn test_consecutive_preserves_runs() {
    let metaspace = b"\xE2\x96\x81";
    // Three consecutive metaspaces should stay together
    // abc = 3, ▁▁▁ = 9, xyz = 3 => total = 15 bytes
    let text = b"abc\xE2\x96\x81\xE2\x96\x81\xE2\x96\x81xyz";

    // Need forward_fallback to find boundary past the run when run extends past target
    let chunks: Vec<_> = chunk(text)
        .pattern(metaspace)
        .size(8)
        .prefix()
        .consecutive()
        .forward_fallback()
        .collect();

    // First chunk should be "abc", second should have all three metaspaces + xyz
    assert_eq!(chunks[0], b"abc");
    // Verify the three consecutive metaspaces are together
    assert!(chunks[1].starts_with(b"\xE2\x96\x81\xE2\x96\x81\xE2\x96\x81"));
    assert_eq!(chunks[1], b"\xE2\x96\x81\xE2\x96\x81\xE2\x96\x81xyz");
}

#[test]
fn test_forward_fallback_basic() {
    let metaspace = b"\xE2\x96\x81";
    // verylongword▁short - pattern is past target_size
    let text = b"verylongword\xE2\x96\x81short";

    // Without forward_fallback: hard split at target
    let chunks_default: Vec<_> = chunk(text).pattern(metaspace).size(6).prefix().collect();

    // With forward_fallback: finds ▁ past target
    let chunks_forward: Vec<_> = chunk(text)
        .pattern(metaspace)
        .size(6)
        .prefix()
        .forward_fallback()
        .collect();

    // Default should hard-split at position 6
    assert_eq!(chunks_default[0], b"verylo");

    // Forward fallback should find metaspace at position 12
    assert_eq!(chunks_forward[0], b"verylongword");
    assert!(chunks_forward[1].starts_with(metaspace));
}

#[test]
fn test_consecutive_and_forward_fallback_combined() {
    let metaspace = b"\xE2\x96\x81";
    // longword▁▁▁short - pattern is past target and consecutive
    let text = b"longword\xE2\x96\x81\xE2\x96\x81\xE2\x96\x81short";

    let chunks: Vec<_> = chunk(text)
        .pattern(metaspace)
        .size(6)
        .prefix()
        .consecutive()
        .forward_fallback()
        .collect();

    // Should find START of ▁▁▁ via forward search
    assert_eq!(chunks[0], b"longword");
    assert!(chunks[1].starts_with(b"\xE2\x96\x81\xE2\x96\x81\xE2\x96\x81"));
}

#[test]
fn test_forward_fallback_no_pattern_anywhere() {
    // No pattern in text at all - forward search finds nothing, returns all remaining
    // This avoids O(n²) behavior from repeatedly searching forward
    let text = b"abcdefghijklmnop";

    let chunks: Vec<_> = chunk(text)
        .pattern(b"XYZ")
        .size(5)
        .forward_fallback()
        .collect();

    // First chunk hard splits (backward found nothing), forward finds nothing → take all
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0], text.as_slice());
}

#[test]
fn test_consecutive_single_byte_pattern() {
    // Test consecutive with single-byte pattern (uses memrchr optimization)
    let text = b"word   next"; // Three spaces

    let chunks: Vec<_> = chunk(text)
        .pattern(b" ")
        .size(6)
        .prefix()
        .consecutive()
        .collect();

    // Should split at start of space run
    assert_eq!(chunks[0], b"word");
    assert!(chunks[1].starts_with(b"   "));
}

#[test]
fn test_owned_chunker_consecutive() {
    let metaspace = b"\xE2\x96\x81";
    let text = b"word\xE2\x96\x81\xE2\x96\x81\xE2\x96\x81next".to_vec();

    let mut chunker = OwnedChunker::new(text.clone())
        .size(10)
        .pattern(metaspace.to_vec())
        .prefix()
        .consecutive();

    let mut chunks = Vec::new();
    while let Some(c) = chunker.next_chunk() {
        chunks.push(c);
    }

    assert_eq!(chunks[0], b"word");
    assert!(chunks[1].starts_with(metaspace));

    let total: usize = chunks.iter().map(|c| c.len()).sum();
    assert_eq!(total, text.len());
}

#[test]
fn test_owned_chunker_forward_fallback() {
    let metaspace = b"\xE2\x96\x81";
    let text = b"verylongword\xE2\x96\x81short".to_vec();

    let mut chunker = OwnedChunker::new(text.clone())
        .size(6)
        .pattern(metaspace.to_vec())
        .prefix()
        .forward_fallback();

    let mut chunks = Vec::new();
    while let Some(c) = chunker.next_chunk() {
        chunks.push(c);
    }

    assert_eq!(chunks[0], b"verylongword");
    assert!(chunks[1].starts_with(metaspace));
}

// ============ Delimiter Consecutive and Forward Fallback Tests ============

#[test]
fn test_delimiter_consecutive_basic() {
    // Hello\n\n\nWorld - should split at START of \n\n\n, not in the middle
    let text = b"Hello\n\n\nWorld";

    // With consecutive: splits at START of \n\n\n
    let chunks: Vec<_> = chunk(text)
        .delimiters(b"\n")
        .size(8)
        .prefix()
        .consecutive()
        .collect();

    // Should preserve all bytes
    let total: usize = chunks.iter().map(|c| c.len()).sum();
    assert_eq!(total, text.len());

    // Consecutive mode should split at "Hello" | "\n\n\nWorld"
    assert_eq!(chunks[0], b"Hello");
    assert!(chunks[1].starts_with(b"\n"));
    assert_eq!(chunks[1], b"\n\n\nWorld");
}

#[test]
fn test_delimiter_consecutive_suffix_mode() {
    // Test consecutive in suffix mode
    let text = b"Hello\n\n\nWorld";

    let chunks: Vec<_> = chunk(text)
        .delimiters(b"\n")
        .size(8)
        .suffix()
        .consecutive()
        .collect();

    // Should preserve all bytes
    let total: usize = chunks.iter().map(|c| c.len()).sum();
    assert_eq!(total, text.len());

    // In suffix mode, first newline goes with "Hello"
    assert_eq!(chunks[0], b"Hello\n");
    assert_eq!(chunks[1], b"\n\nWorld");
}

#[test]
fn test_delimiter_forward_fallback_basic() {
    // verylongword next - delimiter is past target_size
    let text = b"verylongword next";

    // Without forward_fallback: hard split at target
    let chunks_default: Vec<_> = chunk(text).delimiters(b" ").size(6).prefix().collect();

    // With forward_fallback: finds space past target
    let chunks_forward: Vec<_> = chunk(text)
        .delimiters(b" ")
        .size(6)
        .prefix()
        .forward_fallback()
        .collect();

    // Default should hard-split at position 6
    assert_eq!(chunks_default[0], b"verylo");

    // Forward fallback should find space at position 12
    assert_eq!(chunks_forward[0], b"verylongword");
    assert_eq!(chunks_forward[1], b" next");
}

#[test]
fn test_delimiter_consecutive_and_forward_fallback_combined() {
    // longword\n\n\nshort - delimiter is past target and consecutive
    let text = b"longword\n\n\nshort";

    let chunks: Vec<_> = chunk(text)
        .delimiters(b"\n")
        .size(6)
        .prefix()
        .consecutive()
        .forward_fallback()
        .collect();

    // Should find START of \n\n\n via forward search
    assert_eq!(chunks[0], b"longword");
    assert!(chunks[1].starts_with(b"\n\n\n"));
    assert_eq!(chunks[1], b"\n\n\nshort");
}

#[test]
fn test_delimiter_forward_fallback_no_delimiter_anywhere() {
    // No delimiter in text at all - forward search finds nothing, returns all remaining
    let text = b"abcdefghijklmnop";

    let chunks: Vec<_> = chunk(text)
        .delimiters(b".")
        .size(5)
        .forward_fallback()
        .collect();

    // First chunk hard splits (backward found nothing), forward finds nothing → take all
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0], text.as_slice());
}

#[test]
fn test_delimiter_consecutive_multiple_delimiters() {
    // Test with multiple delimiter types - consecutive only applies to same delimiter
    let text = b"Hello.\n\nWorld";

    let chunks: Vec<_> = chunk(text)
        .delimiters(b".\n")
        .size(10)
        .prefix()
        .consecutive()
        .collect();

    // Should preserve all bytes
    let total: usize = chunks.iter().map(|c| c.len()).sum();
    assert_eq!(total, text.len());

    // Backward search finds: \n at 7 (preceded by \n, skip), \n at 6 (preceded by ., valid!)
    // So we split at position 6, keeping the period with "Hello"
    // The \n\n run stays together in the second chunk
    assert_eq!(chunks[0], b"Hello.");
    assert_eq!(chunks[1], b"\n\nWorld");
}

#[test]
fn test_owned_chunker_delimiter_consecutive() {
    let text = b"Hello\n\n\nWorld".to_vec();

    let mut chunker = OwnedChunker::new(text.clone())
        .size(8)
        .delimiters(b"\n".to_vec())
        .prefix()
        .consecutive();

    let mut chunks = Vec::new();
    while let Some(c) = chunker.next_chunk() {
        chunks.push(c);
    }

    assert_eq!(chunks[0], b"Hello");
    assert_eq!(chunks[1], b"\n\n\nWorld");

    let total: usize = chunks.iter().map(|c| c.len()).sum();
    assert_eq!(total, text.len());
}

#[test]
fn test_owned_chunker_delimiter_forward_fallback() {
    let text = b"verylongword next".to_vec();

    let mut chunker = OwnedChunker::new(text.clone())
        .size(6)
        .delimiters(b" ".to_vec())
        .prefix()
        .forward_fallback();

    let mut chunks = Vec::new();
    while let Some(c) = chunker.next_chunk() {
        chunks.push(c);
    }

    assert_eq!(chunks[0], b"verylongword");
    assert_eq!(chunks[1], b" next");
}
