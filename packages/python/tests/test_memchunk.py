import pytest
from memchunk import Chunker, DEFAULT_TARGET_SIZE, DEFAULT_DELIMITERS


class TestChunker:
    def test_basic_chunking(self):
        text = b"Hello. World. Test."
        chunks = list(Chunker(text, size=10, delimiters=b"."))
        assert len(chunks) == 3
        assert chunks[0] == b"Hello."
        assert chunks[1] == b" World."
        assert chunks[2] == b" Test."

    def test_newline_delimiter(self):
        text = b"Line one\nLine two\nLine three"
        chunks = list(Chunker(text, size=15, delimiters=b"\n"))
        assert chunks[0] == b"Line one\n"
        assert chunks[1] == b"Line two\n"
        assert chunks[2] == b"Line three"

    def test_multiple_delimiters(self):
        text = b"Hello? World. Yes!"
        chunks = list(Chunker(text, size=10, delimiters=b".?!"))
        assert chunks[0] == b"Hello?"

    def test_no_delimiter_hard_split(self):
        text = b"abcdefghij"
        chunks = list(Chunker(text, size=5, delimiters=b"."))
        assert chunks[0] == b"abcde"
        assert chunks[1] == b"fghij"

    def test_empty_text(self):
        text = b""
        chunks = list(Chunker(text, size=10, delimiters=b"."))
        assert len(chunks) == 0

    def test_text_smaller_than_target(self):
        text = b"Small"
        chunks = list(Chunker(text, size=100, delimiters=b"."))
        assert len(chunks) == 1
        assert chunks[0] == b"Small"

    def test_total_bytes_preserved(self):
        text = b"The quick brown fox jumps over the lazy dog. How vexingly quick!"
        chunks = list(Chunker(text, size=20, delimiters=b"\n.?!"))
        total = sum(len(c) for c in chunks)
        assert total == len(text)

    def test_defaults(self):
        text = b"Hello world. This is a test."
        chunks = list(Chunker(text))
        assert len(chunks) > 0

    def test_iterator_protocol(self):
        text = b"Hello. World."
        chunker = Chunker(text, size=10, delimiters=b".")
        it = iter(chunker)
        assert next(it) == b"Hello."
        assert next(it) == b" World."
        with pytest.raises(StopIteration):
            next(it)

    def test_reset(self):
        text = b"Hello. World."
        chunker = Chunker(text, size=10, delimiters=b".")
        chunks1 = list(chunker)
        chunker.reset()
        chunks2 = list(chunker)
        assert chunks1 == chunks2

    def test_four_delimiters(self):
        """Test that 4+ delimiters work (uses lookup table internally)."""
        text = b"A. B? C! D; E"
        chunks = list(Chunker(text, size=5, delimiters=b".?!;"))
        assert len(chunks) >= 2


class TestStrInput:
    """Test that str input works (encoded as UTF-8)."""

    def test_str_text(self):
        text = "Hello. World. Test."
        chunks = list(Chunker(text, size=10, delimiters=b"."))
        assert len(chunks) == 3
        assert chunks[0] == b"Hello."

    def test_str_delimiters(self):
        text = b"Hello. World. Test."
        chunks = list(Chunker(text, size=10, delimiters="."))
        assert len(chunks) == 3
        assert chunks[0] == b"Hello."

    def test_str_both(self):
        text = "Hello. World. Test."
        chunks = list(Chunker(text, size=10, delimiters="."))
        assert len(chunks) == 3
        assert chunks[0] == b"Hello."

    def test_unicode(self):
        text = "Caf\u00e9. Tea."  # é is 2 bytes in UTF-8
        chunks = list(Chunker(text, size=10, delimiters="."))
        assert len(chunks) == 2
        # Verify UTF-8 encoding is preserved
        assert b"\xc3\xa9" in chunks[0]  # é in UTF-8
        assert chunks[0] == "Café.".encode("utf-8")


class TestConstants:
    def test_default_target_size(self):
        assert DEFAULT_TARGET_SIZE == 4096

    def test_default_delimiters(self):
        assert DEFAULT_DELIMITERS == b"\n.?"
