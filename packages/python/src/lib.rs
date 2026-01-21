use chunk::{DEFAULT_DELIMITERS, DEFAULT_TARGET_SIZE, OwnedChunker};
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

#[pymodule]
fn _chunk(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Chunker>()?;
    m.add_function(wrap_pyfunction!(chunk_offsets, m)?)?;
    m.add("DEFAULT_TARGET_SIZE", DEFAULT_TARGET_SIZE)?;
    m.add("DEFAULT_DELIMITERS", DEFAULT_DELIMITERS)?;
    Ok(())
}
