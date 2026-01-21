use chunk::{
    DEFAULT_DELIMITERS, DEFAULT_TARGET_SIZE, IncludeDelim, OwnedChunker,
    PatternSplitter as RustPatternSplitter,
    find_merge_indices as rust_find_merge_indices, merge_splits as rust_merge_splits,
    split_at_delimiters, split_at_patterns,
};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyString};

/// Extract bytes from either bytes or str Python object.
fn extract_bytes(obj: &Bound<'_, PyAny>) -> PyResult<Vec<u8>> {
    if obj.is_instance_of::<PyBytes>() {
        Ok(obj.extract::<Vec<u8>>()?)
    } else if obj.is_instance_of::<PyString>() {
        let s: String = obj.extract()?;
        Ok(s.into_bytes())
    } else {
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "expected bytes or str",
        ))
    }
}

/// Chunker splits text at delimiter boundaries.
///
/// Example with single-byte delimiters:
///     >>> from chonkie_core import Chunker
///     >>> text = b"Hello. World. Test."
///     >>> for chunk in Chunker(text, size=10, delimiters=b"."):
///     ...     print(chunk)
///
/// Example with multi-byte pattern (e.g., metaspace for SentencePiece):
///     >>> text = "Hello▁World▁Test"
///     >>> metaspace = "▁"
///     >>> for chunk in Chunker(text, size=15, pattern=metaspace, prefix=True):
///     ...     print(chunk)
///
/// Example with consecutive pattern handling:
///     >>> text = "word   next"  # Three spaces
///     >>> for chunk in Chunker(text, pattern=" ", consecutive=True):
///     ...     print(chunk)  # Splits at START of "   ", not middle
///
/// Also accepts str (encoded as UTF-8):
///     >>> text = "Hello. World. Test."
///     >>> for chunk in Chunker(text, size=10, delimiters="."):
///     ...     print(chunk)
#[pyclass]
pub struct Chunker {
    inner: OwnedChunker,
}

#[pymethods]
impl Chunker {
    #[new]
    #[pyo3(signature = (text, size=DEFAULT_TARGET_SIZE, delimiters=None, pattern=None, prefix=false, consecutive=false, forward_fallback=false))]
    fn new(
        text: &Bound<'_, PyAny>,
        size: usize,
        delimiters: Option<&Bound<'_, PyAny>>,
        pattern: Option<&Bound<'_, PyAny>>,
        prefix: bool,
        consecutive: bool,
        forward_fallback: bool,
    ) -> PyResult<Self> {
        let text_bytes = extract_bytes(text)?;

        let mut inner = OwnedChunker::new(text_bytes).size(size);

        // Pattern takes precedence over delimiters if both specified
        if let Some(p) = pattern {
            let pattern_bytes = extract_bytes(p)?;
            inner = inner.pattern(pattern_bytes);
        } else {
            let delims = match delimiters {
                Some(d) => extract_bytes(d)?,
                None => DEFAULT_DELIMITERS.to_vec(),
            };
            inner = inner.delimiters(delims);
        }

        if prefix {
            inner = inner.prefix();
        }
        if consecutive {
            inner = inner.consecutive();
        }
        if forward_fallback {
            inner = inner.forward_fallback();
        }

        Ok(Self { inner })
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<Py<PyBytes>> {
        slf.inner
            .next_chunk()
            .map(|chunk| PyBytes::new(slf.py(), &chunk).unbind())
    }

    /// Reset the chunker to iterate from the beginning.
    fn reset(&mut self) {
        self.inner.reset();
    }

    /// Collect all chunk offsets as a list of (start, end) tuples.
    /// This is faster than iterating as it makes a single Rust call.
    fn collect_offsets(&mut self) -> Vec<(usize, usize)> {
        self.inner.collect_offsets()
    }
}

/// Fast chunking function that returns offsets in a single call.
/// Use this with slicing for maximum performance.
///
/// Example with single-byte delimiters:
///     >>> text = b"Hello. World. Test."
///     >>> offsets = chunk_offsets(text, size=10, delimiters=b".")
///     >>> chunks = [text[start:end] for start, end in offsets]
///
/// Example with multi-byte pattern:
///     >>> text = "Hello▁World▁Test".encode()
///     >>> offsets = chunk_offsets(text, size=15, pattern="▁", prefix=True)
///     >>> chunks = [text[start:end] for start, end in offsets]
#[pyfunction]
#[pyo3(signature = (text, size=DEFAULT_TARGET_SIZE, delimiters=None, pattern=None, prefix=false, consecutive=false, forward_fallback=false))]
fn chunk_offsets(
    text: &Bound<'_, PyAny>,
    size: usize,
    delimiters: Option<&Bound<'_, PyAny>>,
    pattern: Option<&Bound<'_, PyAny>>,
    prefix: bool,
    consecutive: bool,
    forward_fallback: bool,
) -> PyResult<Vec<(usize, usize)>> {
    let text_bytes = extract_bytes(text)?;

    let mut chunker = OwnedChunker::new(text_bytes).size(size);

    // Pattern takes precedence over delimiters if both specified
    if let Some(p) = pattern {
        let pattern_bytes = extract_bytes(p)?;
        chunker = chunker.pattern(pattern_bytes);
    } else {
        let delims = match delimiters {
            Some(d) => extract_bytes(d)?,
            None => DEFAULT_DELIMITERS.to_vec(),
        };
        chunker = chunker.delimiters(delims);
    }

    if prefix {
        chunker = chunker.prefix();
    }
    if consecutive {
        chunker = chunker.consecutive();
    }
    if forward_fallback {
        chunker = chunker.forward_fallback();
    }

    Ok(chunker.collect_offsets())
}

/// Split text at every delimiter occurrence, returning offsets.
///
/// This is the Rust equivalent of Cython's `split_text` function.
/// Unlike chunk_offsets() which creates size-based chunks, this splits at
/// **every** delimiter occurrence.
///
/// Args:
///     text: bytes or str to split
///     delimiters: bytes or str of delimiter characters (default: "\\n.?")
///     include_delim: Where to attach delimiter - "prev" (default), "next", or "none"
///     min_chars: Minimum characters per segment (default: 0). Shorter segments are merged.
///
/// Returns:
///     List of (start, end) byte offsets for each segment.
///
/// Example:
///     >>> text = b"Hello. World. Test."
///     >>> offsets = split_offsets(text, delimiters=b".")
///     >>> segments = [text[start:end] for start, end in offsets]
///     >>> # ["Hello.", " World.", " Test."]
///
/// Example with include_delim="next":
///     >>> offsets = split_offsets(text, delimiters=b".", include_delim="next")
///     >>> segments = [text[start:end] for start, end in offsets]
///     >>> # ["Hello", ". World", ". Test", "."]
#[pyfunction]
#[pyo3(signature = (text, delimiters=None, include_delim="prev", min_chars=0))]
fn split_offsets(
    text: &Bound<'_, PyAny>,
    delimiters: Option<&Bound<'_, PyAny>>,
    include_delim: &str,
    min_chars: usize,
) -> PyResult<Vec<(usize, usize)>> {
    let text_bytes = extract_bytes(text)?;

    let delims = match delimiters {
        Some(d) => extract_bytes(d)?,
        None => DEFAULT_DELIMITERS.to_vec(),
    };

    let include = match include_delim {
        "prev" => IncludeDelim::Prev,
        "next" => IncludeDelim::Next,
        "none" => IncludeDelim::None,
        _ => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "include_delim must be 'prev', 'next', or 'none'",
            ));
        }
    };

    Ok(split_at_delimiters(
        &text_bytes,
        &delims,
        include,
        min_chars,
    ))
}

/// Split text at every occurrence of multi-byte patterns, returning offsets.
///
/// Unlike split_offsets() which only handles single-byte delimiters,
/// this function supports multi-byte patterns like ". ", "? ", "\n\n", etc.
/// using the Aho-Corasick algorithm for efficient matching.
///
/// Args:
///     text: bytes or str to split
///     patterns: List of bytes or str patterns to split on (e.g., [". ", "? ", "! "])
///     include_delim: Where to attach pattern - "prev" (default), "next", or "none"
///     min_chars: Minimum characters per segment (default: 0). Shorter segments are merged.
///
/// Returns:
///     List of (start, end) byte offsets for each segment.
///
/// Example:
///     >>> text = b"Hello. World? Test!"
///     >>> offsets = split_pattern_offsets(text, patterns=[b". ", b"? ", b"! "])
///     >>> segments = [text[start:end] for start, end in offsets]
///     >>> # [b"Hello. ", b"World? ", b"Test!"]
///
/// Example with paragraph splitting:
///     >>> text = b"Para 1\n\nPara 2\n\nPara 3"
///     >>> offsets = split_pattern_offsets(text, patterns=[b"\n\n"])
///     >>> segments = [text[start:end] for start, end in offsets]
///     >>> # [b"Para 1\n\n", b"Para 2\n\n", b"Para 3"]
#[pyfunction]
#[pyo3(signature = (text, patterns, include_delim="prev", min_chars=0))]
fn split_pattern_offsets(
    text: &Bound<'_, PyAny>,
    patterns: Vec<Bound<'_, PyAny>>,
    include_delim: &str,
    min_chars: usize,
) -> PyResult<Vec<(usize, usize)>> {
    let text_bytes = extract_bytes(text)?;

    // Convert Python patterns to Vec<Vec<u8>>
    let pattern_bytes: Vec<Vec<u8>> = patterns
        .iter()
        .map(|p| extract_bytes(p))
        .collect::<PyResult<Vec<Vec<u8>>>>()?;

    // Convert to slice of slices for the Rust function
    let pattern_slices: Vec<&[u8]> = pattern_bytes.iter().map(|p| p.as_slice()).collect();

    let include = match include_delim {
        "prev" => IncludeDelim::Prev,
        "next" => IncludeDelim::Next,
        "none" => IncludeDelim::None,
        _ => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "include_delim must be 'prev', 'next', or 'none'",
            ));
        }
    };

    Ok(split_at_patterns(
        &text_bytes,
        &pattern_slices,
        include,
        min_chars,
    ))
}

/// A compiled multi-pattern splitter for efficient repeated splitting.
///
/// Unlike split_pattern_offsets() which rebuilds the automaton on each call,
/// PatternSplitter compiles once and reuses. This is ~25x faster when splitting
/// multiple texts with the same patterns.
///
/// Example:
///     >>> from chonkie_core import PatternSplitter
///     >>> splitter = PatternSplitter([b". ", b"? ", b"! "])
///     >>> offsets1 = splitter.split(b"Hello. World?")
///     >>> offsets2 = splitter.split(b"Another. Text!")
#[pyclass]
pub struct PatternSplitter {
    inner: RustPatternSplitter,
}

#[pymethods]
impl PatternSplitter {
    #[new]
    fn new(patterns: Vec<Bound<'_, PyAny>>) -> PyResult<Self> {
        let pattern_bytes: Vec<Vec<u8>> = patterns
            .iter()
            .map(|p| extract_bytes(p))
            .collect::<PyResult<Vec<Vec<u8>>>>()?;

        let pattern_slices: Vec<&[u8]> = pattern_bytes.iter().map(|p| p.as_slice()).collect();
        let inner = RustPatternSplitter::new(&pattern_slices);

        Ok(Self { inner })
    }

    /// Split text using the compiled patterns.
    ///
    /// Args:
    ///     text: bytes or str to split
    ///     include_delim: Where to attach pattern - "prev" (default), "next", or "none"
    ///     min_chars: Minimum characters per segment (default: 0)
    ///
    /// Returns:
    ///     List of (start, end) byte offsets for each segment.
    #[pyo3(signature = (text, include_delim="prev", min_chars=0))]
    fn split(
        &self,
        text: &Bound<'_, PyAny>,
        include_delim: &str,
        min_chars: usize,
    ) -> PyResult<Vec<(usize, usize)>> {
        let text_bytes = extract_bytes(text)?;

        let include = match include_delim {
            "prev" => IncludeDelim::Prev,
            "next" => IncludeDelim::Next,
            "none" => IncludeDelim::None,
            _ => {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "include_delim must be 'prev', 'next', or 'none'",
                ));
            }
        };

        Ok(self.inner.split(&text_bytes, include, min_chars))
    }
}

/// Result of merge_splits operation.
///
/// Attributes:
///     merged: List of merged text strings.
///     token_counts: List of token counts for each merged chunk.
#[pyclass]
#[derive(Clone)]
pub struct MergeResult {
    #[pyo3(get)]
    merged: Vec<String>,
    #[pyo3(get)]
    token_counts: Vec<usize>,
}

#[pymethods]
impl MergeResult {
    fn __repr__(&self) -> String {
        format!(
            "MergeResult(merged=[...{} items], token_counts={:?})",
            self.merged.len(),
            self.token_counts
        )
    }

    fn __len__(&self) -> usize {
        self.merged.len()
    }
}

/// Find merge indices for combining segments within token limits.
///
/// Returns indices marking where to split segments into chunks that
/// respect the token budget. Use this when you only need indices,
/// not the actual merged text.
///
/// Args:
///     token_counts: List of token counts for each segment.
///     chunk_size: Maximum tokens per merged chunk.
///
/// Returns:
///     List of end indices (exclusive) for each chunk.
///
/// Example:
///     >>> from chonkie_core import find_merge_indices
///     >>> token_counts = [1, 1, 1, 1, 1, 1, 1]
///     >>> indices = find_merge_indices(token_counts, chunk_size=3)
///     >>> indices  # [3, 6, 7]
#[pyfunction]
#[pyo3(signature = (token_counts, chunk_size))]
fn find_merge_indices(token_counts: Vec<usize>, chunk_size: usize) -> Vec<usize> {
    rust_find_merge_indices(&token_counts, chunk_size)
}

/// Merge text segments based on token counts, respecting chunk size limits.
///
/// This is the Rust equivalent of Chonkie's Cython `_merge_splits` function.
/// Performs string concatenation in Rust for optimal performance.
///
/// Args:
///     splits: List of text segments to merge.
///     token_counts: List of token counts for each segment.
///     chunk_size: Maximum tokens per merged chunk.
///
/// Returns:
///     MergeResult with:
///     - merged: List of merged text strings
///     - token_counts: Token count for each merged chunk
///
/// Example:
///     >>> from chonkie_core import merge_splits
///     >>> splits = ["Hello", "world", "!", "How", "are", "you"]
///     >>> token_counts = [1, 1, 1, 1, 1, 1]
///     >>> result = merge_splits(splits, token_counts, chunk_size=3)
///     >>> result.merged  # ["Helloworld!", "Howareyou"]
///     >>> result.token_counts  # [3, 3]
#[pyfunction]
#[pyo3(signature = (splits, token_counts, chunk_size))]
fn merge_splits(
    splits: Vec<String>,
    token_counts: Vec<usize>,
    chunk_size: usize,
) -> MergeResult {
    let split_refs: Vec<&str> = splits.iter().map(|s| s.as_str()).collect();
    let result = rust_merge_splits(&split_refs, &token_counts, chunk_size);
    MergeResult {
        merged: result.merged,
        token_counts: result.token_counts,
    }
}

#[pymodule]
fn _chunk(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Chunker>()?;
    m.add_class::<MergeResult>()?;
    m.add_class::<PatternSplitter>()?;
    m.add_function(wrap_pyfunction!(chunk_offsets, m)?)?;
    m.add_function(wrap_pyfunction!(split_offsets, m)?)?;
    m.add_function(wrap_pyfunction!(split_pattern_offsets, m)?)?;
    m.add_function(wrap_pyfunction!(find_merge_indices, m)?)?;
    m.add_function(wrap_pyfunction!(merge_splits, m)?)?;
    m.add("DEFAULT_TARGET_SIZE", DEFAULT_TARGET_SIZE)?;
    m.add("DEFAULT_DELIMITERS", DEFAULT_DELIMITERS)?;
    Ok(())
}
