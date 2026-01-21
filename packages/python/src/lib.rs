use chunk::{
    DEFAULT_DELIMITERS, DEFAULT_TARGET_SIZE, IncludeDelim, OwnedChunker,
    merge_splits as rust_merge_splits, split_at_delimiters,
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

/// Result of merge_splits operation.
///
/// Attributes:
///     indices: List of end indices for each merged chunk (exclusive).
///              Use with slicing: segments[prev_end:end]
///     token_counts: List of token counts for each merged chunk.
#[pyclass]
#[derive(Clone)]
pub struct MergeResult {
    #[pyo3(get)]
    indices: Vec<usize>,
    #[pyo3(get)]
    token_counts: Vec<usize>,
}

#[pymethods]
impl MergeResult {
    fn __repr__(&self) -> String {
        format!(
            "MergeResult(indices={:?}, token_counts={:?})",
            self.indices, self.token_counts
        )
    }

    fn __len__(&self) -> usize {
        self.indices.len()
    }
}

/// Merge segments based on token counts, respecting chunk size limits.
///
/// This is the Rust equivalent of Chonkie's Cython `_merge_splits` function.
/// Used by RecursiveChunker to merge small segments into larger chunks
/// that fit within a token budget.
///
/// Args:
///     token_counts: List of token counts for each segment.
///     chunk_size: Maximum tokens per merged chunk.
///     combine_whitespace: If True, adds +1 token per join for whitespace (default: False).
///
/// Returns:
///     MergeResult with:
///     - indices: End indices for slicing segments (exclusive)
///     - token_counts: Token count for each merged chunk
///
/// Example:
///     >>> from chonkie_core import merge_splits
///     >>> token_counts = [1, 1, 1, 1, 1, 1, 1]
///     >>> result = merge_splits(token_counts, chunk_size=3)
///     >>> result.indices  # [3, 6, 7]
///     >>> result.token_counts  # [3, 3, 1]
///
/// Example with whitespace:
///     >>> result = merge_splits(token_counts, chunk_size=5, combine_whitespace=True)
///     >>> result.indices  # [3, 6, 7]
///     >>> result.token_counts  # [5, 5, 1] (3 tokens + 2 whitespace joins per chunk)
#[pyfunction]
#[pyo3(signature = (token_counts, chunk_size, combine_whitespace=false))]
fn merge_splits(
    token_counts: Vec<usize>,
    chunk_size: usize,
    combine_whitespace: bool,
) -> MergeResult {
    let result = rust_merge_splits(&token_counts, chunk_size, combine_whitespace);
    MergeResult {
        indices: result.indices,
        token_counts: result.token_counts,
    }
}

#[pymodule]
fn _chunk(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Chunker>()?;
    m.add_class::<MergeResult>()?;
    m.add_function(wrap_pyfunction!(chunk_offsets, m)?)?;
    m.add_function(wrap_pyfunction!(split_offsets, m)?)?;
    m.add_function(wrap_pyfunction!(merge_splits, m)?)?;
    m.add("DEFAULT_TARGET_SIZE", DEFAULT_TARGET_SIZE)?;
    m.add("DEFAULT_DELIMITERS", DEFAULT_DELIMITERS)?;
    Ok(())
}
