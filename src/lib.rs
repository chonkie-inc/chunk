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
//! ```

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
pub struct Chunker<'a> {
    text: &'a [u8],
    target_size: usize,
    delimiters: &'a [u8],
    pos: usize,
    table: Option<[bool; 256]>,
    initialized: bool,
    prefix_mode: bool,
}

impl<'a> Chunker<'a> {
    fn new(text: &'a [u8]) -> Self {
        Self {
            text,
            target_size: DEFAULT_TARGET_SIZE,
            delimiters: DEFAULT_DELIMITERS,
            pos: 0,
            table: None,
            initialized: false,
            prefix_mode: false,
        }
    }

    /// Set the target chunk size in bytes.
    pub fn size(mut self, size: usize) -> Self {
        self.target_size = size;
        self
    }

    /// Set the delimiters to split on.
    pub fn delimiters(mut self, delimiters: &'a [u8]) -> Self {
        self.delimiters = delimiters;
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
        let window = &self.text[self.pos..end];

        // Find last delimiter in window
        let split_at = match find_last_delimiter(window, self.delimiters, self.table.as_ref()) {
            Some(pos) => {
                if self.prefix_mode {
                    self.pos + pos // Delimiter goes to next chunk
                } else {
                    self.pos + pos + 1 // Delimiter stays with current chunk
                }
            }
            None => end, // No delimiter, hard split at target
        };

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
    pos: usize,
    table: Option<[bool; 256]>,
    initialized: bool,
    prefix_mode: bool,
}

impl OwnedChunker {
    /// Create a new owned chunker with the given text.
    pub fn new(text: Vec<u8>) -> Self {
        Self {
            text,
            target_size: DEFAULT_TARGET_SIZE,
            delimiters: DEFAULT_DELIMITERS.to_vec(),
            pos: 0,
            table: None,
            initialized: false,
            prefix_mode: false,
        }
    }

    /// Set the target chunk size in bytes.
    pub fn size(mut self, size: usize) -> Self {
        self.target_size = size;
        self
    }

    /// Set the delimiters to split on.
    pub fn delimiters(mut self, delimiters: Vec<u8>) -> Self {
        self.delimiters = delimiters;
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
        let window = &self.text[self.pos..end];

        // Find last delimiter in window
        let split_at = match find_last_delimiter(window, &self.delimiters, self.table.as_ref()) {
            Some(pos) => {
                if self.prefix_mode {
                    self.pos + pos
                } else {
                    self.pos + pos + 1
                }
            }
            None => end,
        };

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
            let window = &self.text[pos..end];

            let split_at = match find_last_delimiter(window, &self.delimiters, self.table.as_ref())
            {
                Some(p) => {
                    if self.prefix_mode {
                        pos + p
                    } else {
                        pos + p + 1
                    }
                }
                None => end,
            };

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
