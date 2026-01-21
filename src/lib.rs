//! The fastest semantic text chunking library — up to 1TB/s chunking throughput.
//!
//! This crate provides three main functionalities:
//!
//! 1. **Size-based chunking** ([`chunk`] module): Split text into chunks of a target size,
//!    preferring to break at delimiter boundaries.
//!
//! 2. **Delimiter splitting** ([`split`] module): Split text at every delimiter occurrence,
//!    equivalent to Cython's `split_text` function.
//!
//! 3. **Token-aware merging** ([`merge`] module): Merge segments based on token counts,
//!    equivalent to Cython's `_merge_splits` function.
//!
//! # Examples
//!
//! ## Size-based chunking
//!
//! ```
//! use chunk::chunk;
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
//!
//! ## Delimiter splitting
//!
//! ```
//! use chunk::{split, split_at_delimiters, IncludeDelim};
//!
//! let text = b"Hello. World. Test.";
//!
//! // Using the builder API
//! let slices = split(text).delimiters(b".").include_prev().collect_slices();
//! assert_eq!(slices, vec![b"Hello.".as_slice(), b" World.".as_slice(), b" Test.".as_slice()]);
//!
//! // Using the function directly
//! let offsets = split_at_delimiters(text, b".", IncludeDelim::Prev, 0);
//! assert_eq!(&text[offsets[0].0..offsets[0].1], b"Hello.");
//! ```
//!
//! ## Token-aware merging
//!
//! ```
//! use chunk::merge_splits;
//!
//! // Merge text segments based on token counts
//! let splits = vec!["a", "b", "c", "d", "e", "f", "g"];
//! let token_counts = vec![1, 1, 1, 1, 1, 1, 1];
//! let result = merge_splits(&splits, &token_counts, 3);
//! assert_eq!(result.merged, vec!["abc", "def", "g"]);
//! assert_eq!(result.token_counts, vec![3, 3, 1]);
//! ```

mod chunk;
mod delim;
mod merge;
mod split;

// Re-export from chunk module
pub use crate::chunk::{Chunker, OwnedChunker, chunk};

// Re-export from split module
pub use crate::split::{IncludeDelim, PatternSplitter, Splitter, split, split_at_delimiters, split_at_patterns};

// Re-export from merge module
pub use crate::merge::{MergeResult, find_merge_indices, merge_splits};

// Re-export constants from delim module
pub use crate::delim::{DEFAULT_DELIMITERS, DEFAULT_TARGET_SIZE};

// Additional tests that span modules
#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_chunk_and_split_consistency() {
        // Both should preserve all bytes
        let text = b"Hello. World. Test.";

        let chunk_total: usize = chunk(text).size(10).delimiters(b".").map(|c| c.len()).sum();
        let split_total: usize = split_at_delimiters(text, b".", IncludeDelim::Prev, 0)
            .iter()
            .map(|(s, e)| e - s)
            .sum();

        assert_eq!(chunk_total, text.len());
        assert_eq!(split_total, text.len());
    }

    #[test]
    fn test_consecutive_delimiters_chunk() {
        let text = b"Hello\n\nWorld";
        let chunks: Vec<_> = chunk(text).size(8).delimiters(b"\n").collect();
        let total: usize = chunks.iter().map(|c| c.len()).sum();
        assert_eq!(total, text.len());
    }

    #[test]
    fn test_prefix_mode_chunk() {
        let text = b"Hello World Test";
        let chunks: Vec<_> = chunk(text).size(8).delimiters(b" ").prefix().collect();
        assert_eq!(chunks[0], b"Hello");
        assert_eq!(chunks[1], b" World");
        assert_eq!(chunks[2], b" Test");
    }

    #[test]
    fn test_prefix_preserves_total_bytes() {
        let text = b"Hello World Test More Words Here";
        let chunks: Vec<_> = chunk(text).size(10).delimiters(b" ").prefix().collect();
        let total: usize = chunks.iter().map(|c| c.len()).sum();
        assert_eq!(total, text.len());
    }

    #[test]
    fn test_prefix_mode_delimiter_at_window_start() {
        let text = b"Hello world";
        let chunks: Vec<_> = chunk(text).size(5).delimiters(b" ").prefix().collect();
        let total: usize = chunks.iter().map(|c| c.len()).sum();
        assert_eq!(total, text.len());
        assert_eq!(chunks[0], b"Hello");
    }

    #[test]
    fn test_prefix_mode_small_chunks() {
        let text = b"a b c d e";
        let chunks: Vec<_> = chunk(text).size(2).delimiters(b" ").prefix().collect();
        let total: usize = chunks.iter().map(|c| c.len()).sum();
        assert_eq!(total, text.len());
        for c in &chunks {
            assert!(!c.is_empty(), "Found empty chunk!");
        }
    }

    // ============ Multi-byte pattern tests ============

    #[test]
    fn test_pattern_metaspace_suffix() {
        let metaspace = "▁".as_bytes();
        let text = "Hello▁World▁Test".as_bytes();
        let chunks: Vec<_> = chunk(text).size(15).pattern(metaspace).collect();
        assert_eq!(chunks[0], "Hello▁".as_bytes());
        assert_eq!(chunks[1], "World▁Test".as_bytes());
        let total: usize = chunks.iter().map(|c| c.len()).sum();
        assert_eq!(total, text.len());
    }

    #[test]
    fn test_pattern_metaspace_prefix() {
        let metaspace = "▁".as_bytes();
        let text = "Hello▁World▁Test".as_bytes();
        let chunks: Vec<_> = chunk(text).size(15).pattern(metaspace).prefix().collect();
        assert_eq!(chunks[0], "Hello".as_bytes());
        assert_eq!(chunks[1], "▁World▁Test".as_bytes());
        let total: usize = chunks.iter().map(|c| c.len()).sum();
        assert_eq!(total, text.len());
    }

    #[test]
    fn test_pattern_preserves_bytes() {
        let metaspace = "▁".as_bytes();
        let text = "The▁quick▁brown▁fox▁jumps▁over▁the▁lazy▁dog".as_bytes();
        let chunks: Vec<_> = chunk(text).size(20).pattern(metaspace).collect();
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
    }

    #[test]
    fn test_pattern_single_byte_optimization() {
        let text = b"Hello World Test";
        let chunks: Vec<_> = chunk(text).size(8).pattern(b" ").prefix().collect();
        assert_eq!(chunks[0], b"Hello");
        assert_eq!(chunks[1], b" World");
    }

    // ============ Consecutive and Forward Fallback Tests ============

    #[test]
    fn test_consecutive_pattern_basic() {
        let metaspace = b"\xE2\x96\x81";
        let text = b"word\xE2\x96\x81\xE2\x96\x81\xE2\x96\x81next";
        let chunks: Vec<_> = chunk(text)
            .pattern(metaspace)
            .size(10)
            .prefix()
            .consecutive()
            .collect();
        let total: usize = chunks.iter().map(|c| c.len()).sum();
        assert_eq!(total, text.len());
        assert_eq!(chunks[0], b"word");
        assert!(chunks[1].starts_with(metaspace));
    }

    #[test]
    fn test_forward_fallback_basic() {
        let metaspace = b"\xE2\x96\x81";
        let text = b"verylongword\xE2\x96\x81short";
        let chunks: Vec<_> = chunk(text)
            .pattern(metaspace)
            .size(6)
            .prefix()
            .forward_fallback()
            .collect();
        assert_eq!(chunks[0], b"verylongword");
        assert!(chunks[1].starts_with(metaspace));
    }

    #[test]
    fn test_delimiter_consecutive_basic() {
        let text = b"Hello\n\n\nWorld";
        let chunks: Vec<_> = chunk(text)
            .delimiters(b"\n")
            .size(8)
            .prefix()
            .consecutive()
            .collect();
        let total: usize = chunks.iter().map(|c| c.len()).sum();
        assert_eq!(total, text.len());
        assert_eq!(chunks[0], b"Hello");
        assert_eq!(chunks[1], b"\n\n\nWorld");
    }

    #[test]
    fn test_delimiter_forward_fallback_basic() {
        let text = b"verylongword next";
        let chunks: Vec<_> = chunk(text)
            .delimiters(b" ")
            .size(6)
            .prefix()
            .forward_fallback()
            .collect();
        assert_eq!(chunks[0], b"verylongword");
        assert_eq!(chunks[1], b" next");
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
        let total: usize = chunks.iter().map(|c| c.len()).sum();
        assert_eq!(total, text.len());
    }

    #[test]
    fn test_owned_chunker_collect_offsets() {
        let metaspace = "▁".as_bytes();
        let text = "Hello▁World▁Test".as_bytes().to_vec();
        let mut chunker = OwnedChunker::new(text.clone())
            .size(15)
            .pattern(metaspace.to_vec())
            .prefix();
        let offsets = chunker.collect_offsets();
        assert_eq!(offsets[0], (0, 5));
        assert_eq!(&text[offsets[0].0..offsets[0].1], "Hello".as_bytes());
    }
}
