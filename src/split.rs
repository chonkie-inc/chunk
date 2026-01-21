//! Delimiter-based text splitting.
//!
//! This module provides functions to split text at every delimiter occurrence,
//! equivalent to Cython's `split_text` function. Unlike the [`chunk`](crate::chunk)
//! module which creates size-based chunks, this splits at **every** delimiter.

use crate::delim::{DEFAULT_DELIMITERS, build_table, find_first_delimiter};

/// Where to include the delimiter in splits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IncludeDelim {
    /// Attach delimiter to the previous segment (e.g., "Hello." | " World.")
    #[default]
    Prev,
    /// Attach delimiter to the next segment (e.g., "Hello" | ". World")
    Next,
    /// Don't include delimiter in either segment
    None,
}

/// Split text at every delimiter occurrence, returning offsets.
///
/// This is the Rust equivalent of Cython's `split_text` function.
/// Unlike [`chunk()`](crate::chunk) which creates size-based chunks, this splits at
/// **every** delimiter occurrence.
///
/// # Arguments
///
/// * `text` - The text to split
/// * `delimiters` - Single-byte delimiters to split on
/// * `include_delim` - Where to attach the delimiter (Prev, Next, or None)
/// * `min_chars` - Minimum characters per segment; shorter segments are merged
///
/// # Returns
///
/// Vector of (start, end) byte offsets for each segment.
///
/// # Example
///
/// ```
/// use chunk::{split_at_delimiters, IncludeDelim};
///
/// let text = b"Hello. World. Test.";
/// let offsets = split_at_delimiters(text, b".", IncludeDelim::Prev, 0);
///
/// // Creates: ["Hello.", " World.", " Test."]
/// assert_eq!(offsets.len(), 3);
/// assert_eq!(&text[offsets[0].0..offsets[0].1], b"Hello.");
/// assert_eq!(&text[offsets[1].0..offsets[1].1], b" World.");
/// assert_eq!(&text[offsets[2].0..offsets[2].1], b" Test.");
/// ```
pub fn split_at_delimiters(
    text: &[u8],
    delimiters: &[u8],
    include_delim: IncludeDelim,
    min_chars: usize,
) -> Vec<(usize, usize)> {
    if text.is_empty() {
        return vec![];
    }

    if delimiters.is_empty() {
        // No delimiters - return whole text as single segment
        return vec![(0, text.len())];
    }

    // Build lookup table for 4+ delimiters
    let table = build_table(delimiters);

    // Optimization #2: Pre-allocate with heuristic estimate
    // Assume average segment size of ~40 bytes, minimum 4 segments
    let estimated_segments = (text.len() / 40).max(4);
    let mut splits: Vec<(usize, usize)> = Vec::with_capacity(estimated_segments);

    let mut segment_start = 0;
    let mut pos = 0;

    // Optimization #1: Single-pass min_chars merging
    // Track accumulated segment that hasn't been emitted yet
    let mut accum_start: usize = 0;
    let mut accum_end: usize = 0;

    // Helper macro to emit a segment with optional min_chars merging
    macro_rules! emit_segment {
        ($seg_start:expr, $seg_end:expr) => {
            let seg_start = $seg_start;
            let seg_end = $seg_end;

            if min_chars == 0 {
                // No merging needed, emit directly
                splits.push((seg_start, seg_end));
            } else if accum_start == accum_end {
                // No accumulated segment yet, start one
                accum_start = seg_start;
                accum_end = seg_end;
            } else {
                let accum_len = accum_end - accum_start;
                let seg_len = seg_end - seg_start;

                if accum_len < min_chars || seg_len < min_chars {
                    // Either is too short, merge them
                    accum_end = seg_end;
                } else {
                    // Both long enough, emit accumulated and start new
                    splits.push((accum_start, accum_end));
                    accum_start = seg_start;
                    accum_end = seg_end;
                }
            }
        };
    }

    while pos < text.len() {
        // Find next delimiter
        let delim_pos = find_first_delimiter(&text[pos..], delimiters, table.as_ref());

        match delim_pos {
            Some(rel_pos) => {
                let abs_pos = pos + rel_pos;

                match include_delim {
                    IncludeDelim::Prev => {
                        // Delimiter goes with current segment: "Hello."
                        let seg_end = abs_pos + 1;
                        if segment_start < seg_end {
                            emit_segment!(segment_start, seg_end);
                        }
                        segment_start = seg_end;
                    }
                    IncludeDelim::Next => {
                        // Delimiter goes with next segment: "Hello" | ". World"
                        if segment_start < abs_pos {
                            emit_segment!(segment_start, abs_pos);
                        }
                        segment_start = abs_pos;
                    }
                    IncludeDelim::None => {
                        // Don't include delimiter
                        if segment_start < abs_pos {
                            emit_segment!(segment_start, abs_pos);
                        }
                        segment_start = abs_pos + 1;
                    }
                }
                pos = abs_pos + 1;
            }
            None => {
                // No more delimiters - add remaining text
                if segment_start < text.len() {
                    emit_segment!(segment_start, text.len());
                }
                break;
            }
        }
    }

    // Handle trailing content after last delimiter
    if segment_start < text.len() && (splits.is_empty() || splits.last().map_or(true, |&(_, e)| e < text.len())) {
        // Only emit if not already handled by the None branch
        if min_chars == 0 || accum_end < text.len() {
            emit_segment!(segment_start, text.len());
        }
    }

    // Emit final accumulated segment (for min_chars mode)
    if min_chars > 0 && accum_start < accum_end {
        splits.push((accum_start, accum_end));
    }

    splits
}

/// Builder for delimiter-based splitting with more options.
///
/// Created via [`split()`], can be configured with various options.
///
/// # Example
///
/// ```
/// use chunk::split;
///
/// let text = b"Hello. World? Test!";
/// let offsets: Vec<_> = split(text)
///     .delimiters(b".?!")
///     .include_prev()
///     .min_chars(5)
///     .collect();
/// ```
pub fn split(text: &[u8]) -> Splitter<'_> {
    Splitter::new(text)
}

/// Splitter splits text at every delimiter occurrence.
///
/// Created via [`split()`], can be configured with `.delimiters()`, `.include_prev()`, etc.
pub struct Splitter<'a> {
    text: &'a [u8],
    delimiters: &'a [u8],
    include_delim: IncludeDelim,
    min_chars: usize,
}

impl<'a> Splitter<'a> {
    fn new(text: &'a [u8]) -> Self {
        Self {
            text,
            delimiters: DEFAULT_DELIMITERS,
            include_delim: IncludeDelim::Prev,
            min_chars: 0,
        }
    }

    /// Set delimiters to split on.
    pub fn delimiters(mut self, delimiters: &'a [u8]) -> Self {
        self.delimiters = delimiters;
        self
    }

    /// Include delimiter with previous segment (default).
    pub fn include_prev(mut self) -> Self {
        self.include_delim = IncludeDelim::Prev;
        self
    }

    /// Include delimiter with next segment.
    pub fn include_next(mut self) -> Self {
        self.include_delim = IncludeDelim::Next;
        self
    }

    /// Don't include delimiter in either segment.
    pub fn include_none(mut self) -> Self {
        self.include_delim = IncludeDelim::None;
        self
    }

    /// Set minimum characters per segment (merges shorter segments).
    pub fn min_chars(mut self, min: usize) -> Self {
        self.min_chars = min;
        self
    }

    /// Collect all split offsets.
    pub fn collect(self) -> Vec<(usize, usize)> {
        split_at_delimiters(
            self.text,
            self.delimiters,
            self.include_delim,
            self.min_chars,
        )
    }

    /// Collect splits as byte slices.
    pub fn collect_slices(self) -> Vec<&'a [u8]> {
        let text = self.text;
        let offsets =
            split_at_delimiters(text, self.delimiters, self.include_delim, self.min_chars);
        offsets
            .into_iter()
            .map(|(start, end)| &text[start..end])
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_basic() {
        let text = b"Hello. World. Test.";
        let offsets = split_at_delimiters(text, b".", IncludeDelim::Prev, 0);
        assert_eq!(offsets.len(), 3);
        assert_eq!(&text[offsets[0].0..offsets[0].1], b"Hello.");
        assert_eq!(&text[offsets[1].0..offsets[1].1], b" World.");
        assert_eq!(&text[offsets[2].0..offsets[2].1], b" Test.");
    }

    #[test]
    fn test_split_include_next() {
        let text = b"Hello. World. Test.";
        let offsets = split_at_delimiters(text, b".", IncludeDelim::Next, 0);
        // Splits at EVERY delimiter, so trailing "." becomes its own segment
        assert_eq!(offsets.len(), 4);
        assert_eq!(&text[offsets[0].0..offsets[0].1], b"Hello");
        assert_eq!(&text[offsets[1].0..offsets[1].1], b". World");
        assert_eq!(&text[offsets[2].0..offsets[2].1], b". Test");
        assert_eq!(&text[offsets[3].0..offsets[3].1], b".");
    }

    #[test]
    fn test_split_include_next_no_trailing() {
        // Text without trailing delimiter
        let text = b"Hello. World. Test";
        let offsets = split_at_delimiters(text, b".", IncludeDelim::Next, 0);
        assert_eq!(offsets.len(), 3);
        assert_eq!(&text[offsets[0].0..offsets[0].1], b"Hello");
        assert_eq!(&text[offsets[1].0..offsets[1].1], b". World");
        assert_eq!(&text[offsets[2].0..offsets[2].1], b". Test");
    }

    #[test]
    fn test_split_include_none() {
        let text = b"Hello. World. Test.";
        let offsets = split_at_delimiters(text, b".", IncludeDelim::None, 0);
        assert_eq!(offsets.len(), 3);
        assert_eq!(&text[offsets[0].0..offsets[0].1], b"Hello");
        assert_eq!(&text[offsets[1].0..offsets[1].1], b" World");
        assert_eq!(&text[offsets[2].0..offsets[2].1], b" Test");
    }

    #[test]
    fn test_split_multiple_delimiters() {
        let text = b"Hello. World? Test!";
        let offsets = split_at_delimiters(text, b".?!", IncludeDelim::Prev, 0);
        assert_eq!(offsets.len(), 3);
        assert_eq!(&text[offsets[0].0..offsets[0].1], b"Hello.");
        assert_eq!(&text[offsets[1].0..offsets[1].1], b" World?");
        assert_eq!(&text[offsets[2].0..offsets[2].1], b" Test!");
    }

    #[test]
    fn test_split_min_chars() {
        let text = b"A. B. C. D. E.";
        // Without min_chars, we get 5 short segments
        let offsets = split_at_delimiters(text, b".", IncludeDelim::Prev, 0);
        assert_eq!(offsets.len(), 5);

        // With min_chars=4, short segments get merged
        let offsets = split_at_delimiters(text, b".", IncludeDelim::Prev, 4);
        assert!(offsets.len() < 5);
    }

    #[test]
    fn test_split_empty_text() {
        let text = b"";
        let offsets = split_at_delimiters(text, b".", IncludeDelim::Prev, 0);
        assert_eq!(offsets.len(), 0);
    }

    #[test]
    fn test_split_no_delimiters() {
        let text = b"Hello World";
        let offsets = split_at_delimiters(text, b".", IncludeDelim::Prev, 0);
        assert_eq!(offsets.len(), 1);
        assert_eq!(&text[offsets[0].0..offsets[0].1], b"Hello World");
    }

    #[test]
    fn test_split_empty_delimiters() {
        let text = b"Hello World";
        let offsets = split_at_delimiters(text, b"", IncludeDelim::Prev, 0);
        assert_eq!(offsets.len(), 1);
        assert_eq!(&text[offsets[0].0..offsets[0].1], b"Hello World");
    }

    #[test]
    fn test_split_newlines() {
        let text = b"Line 1\nLine 2\nLine 3";
        let offsets = split_at_delimiters(text, b"\n", IncludeDelim::Prev, 0);
        assert_eq!(offsets.len(), 3);
        assert_eq!(&text[offsets[0].0..offsets[0].1], b"Line 1\n");
        assert_eq!(&text[offsets[1].0..offsets[1].1], b"Line 2\n");
        assert_eq!(&text[offsets[2].0..offsets[2].1], b"Line 3");
    }

    #[test]
    fn test_split_builder() {
        let text = b"Hello. World? Test!";
        let slices = split(text)
            .delimiters(b".?!")
            .include_prev()
            .collect_slices();
        assert_eq!(slices.len(), 3);
        assert_eq!(slices[0], b"Hello.");
        assert_eq!(slices[1], b" World?");
        assert_eq!(slices[2], b" Test!");
    }

    #[test]
    fn test_split_builder_include_next() {
        let text = b"Hello. World.";
        let slices = split(text).delimiters(b".").include_next().collect_slices();
        // Splits at EVERY delimiter, so trailing "." becomes its own segment
        assert_eq!(slices.len(), 3);
        assert_eq!(slices[0], b"Hello");
        assert_eq!(slices[1], b". World");
        assert_eq!(slices[2], b".");
    }

    #[test]
    fn test_split_preserves_all_bytes() {
        let text = b"The quick brown fox. Jumps over? The lazy dog!";
        let offsets = split_at_delimiters(text, b".?!", IncludeDelim::Prev, 0);

        // Verify all bytes are accounted for
        let total: usize = offsets.iter().map(|(s, e)| e - s).sum();
        assert_eq!(total, text.len());

        // Verify offsets are contiguous
        for i in 1..offsets.len() {
            assert_eq!(offsets[i - 1].1, offsets[i].0);
        }
    }

    #[test]
    fn test_split_four_delimiters() {
        // 4+ delimiters use lookup table instead of memchr
        let text = b"A. B? C! D; E";
        let offsets = split_at_delimiters(text, b".?!;", IncludeDelim::Prev, 0);
        assert_eq!(offsets.len(), 5);
    }
}
