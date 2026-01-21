"""chonkie-core - The fastest semantic text chunking library."""

from chonkie_core._chunk import (
    Chunker,
    chunk_offsets,
    DEFAULT_TARGET_SIZE,
    DEFAULT_DELIMITERS,
)

__all__ = ["chunk", "Chunker", "chunk_offsets", "DEFAULT_TARGET_SIZE", "DEFAULT_DELIMITERS"]
__version__ = "0.5.0"


def chunk(text, *, size=DEFAULT_TARGET_SIZE, delimiters=None):
    """
    Split text into chunks at delimiter boundaries.
    Returns an iterator of zero-copy memoryview slices.

    Args:
        text: bytes or str to chunk
        size: Target chunk size in bytes (default: 4096)
        delimiters: bytes or str of delimiter characters (default: "\\n.?")

    Yields:
        memoryview slices of the original text

    Example:
        >>> text = b"Hello. World. Test."
        >>> for chunk in chunk(text, size=10, delimiters=b"."):
        ...     print(bytes(chunk))
        b'Hello.'
        b' World.'
        b' Test.'
    """
    # Convert str to bytes if needed
    if isinstance(text, str):
        text = text.encode("utf-8")

    # Get offsets from Rust (single FFI call)
    offsets = chunk_offsets(text, size, delimiters)

    # Return memoryview slices (zero-copy)
    mv = memoryview(text)
    for start, end in offsets:
        yield mv[start:end]
