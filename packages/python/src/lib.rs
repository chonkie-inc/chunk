use memchunk::{OwnedChunker, DEFAULT_DELIMITERS, DEFAULT_TARGET_SIZE};
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
/// Example:
///     >>> from memchunk import Chunker
///     >>> text = b"Hello. World. Test."
///     >>> for chunk in Chunker(text, size=10, delimiters=b"."):
///     ...     print(chunk)
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
    #[pyo3(signature = (text, size=DEFAULT_TARGET_SIZE, delimiters=None))]
    fn new(
        text: &Bound<'_, PyAny>,
        size: usize,
        delimiters: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let text_bytes = extract_bytes(text)?;
        let delims = match delimiters {
            Some(d) => extract_bytes(d)?,
            None => DEFAULT_DELIMITERS.to_vec(),
        };
        let inner = OwnedChunker::new(text_bytes).size(size).delimiters(delims);
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
/// Example:
///     >>> text = b"Hello. World. Test."
///     >>> offsets = chunk_offsets(text, size=10, delimiters=b".")
///     >>> chunks = [text[start:end] for start, end in offsets]
#[pyfunction]
#[pyo3(signature = (text, size=DEFAULT_TARGET_SIZE, delimiters=None))]
fn chunk_offsets(
    text: &Bound<'_, PyAny>,
    size: usize,
    delimiters: Option<&Bound<'_, PyAny>>,
) -> PyResult<Vec<(usize, usize)>> {
    let text_bytes = extract_bytes(text)?;
    let delims = match delimiters {
        Some(d) => extract_bytes(d)?,
        None => DEFAULT_DELIMITERS.to_vec(),
    };
    let mut chunker = OwnedChunker::new(text_bytes).size(size).delimiters(delims);
    Ok(chunker.collect_offsets())
}

#[pymodule]
fn _memchunk(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Chunker>()?;
    m.add_function(wrap_pyfunction!(chunk_offsets, m)?)?;
    m.add("DEFAULT_TARGET_SIZE", DEFAULT_TARGET_SIZE)?;
    m.add("DEFAULT_DELIMITERS", DEFAULT_DELIMITERS)?;
    Ok(())
}
