<p align="center">
  <img src="../../assets/memchunk_wide.png" alt="chonkie-core" width="500">
</p>

<h1 align="center">chonkie-core</h1>

<p align="center">
  <em>the fastest text chunking library — up to 1 TB/s throughput</em>
</p>

<p align="center">
  <a href="https://crates.io/crates/chunk"><img src="https://img.shields.io/crates/v/chunk.svg?color=e74c3c" alt="crates.io"></a>
  <a href="https://pypi.org/project/chonkie-core"><img src="https://img.shields.io/pypi/v/chonkie-core.svg?color=e67e22" alt="PyPI"></a>
  <a href="https://www.npmjs.com/package/@chonkiejs/chunk"><img src="https://img.shields.io/npm/v/@chonkiejs/chunk.svg?color=2ecc71" alt="npm"></a>
  <a href="https://github.com/chonkie-inc/chunk"><img src="https://img.shields.io/badge/github-chunk-3498db" alt="GitHub"></a>
  <a href="LICENSE-MIT"><img src="https://img.shields.io/badge/license-MIT%2FApache--2.0-9b59b6.svg" alt="License"></a>
</p>

---

you know how every chunking library claims to be fast? yeah, we actually meant it.

**chonkie-core** splits text at semantic boundaries (periods, newlines, the usual suspects) and does it stupid fast. we're talking "chunk the entire english wikipedia in 120ms" fast.

want to know how? [read the blog post](https://minha.sh/posts/so,-you-want-to-chunk-really-fast) where we nerd out about SIMD instructions and lookup tables.

## 📦 installation

```bash
pip install chonkie-core
```

looking for [rust](https://github.com/chonkie-inc/chunk) or [javascript](https://github.com/chonkie-inc/chunk/tree/main/packages/wasm)?

## 🚀 usage

```python
from chonkie_core import Chunker

text = "Hello world. How are you? I'm fine.\nThanks for asking."

# with defaults (4KB chunks, split at \n . ?)
for chunk in Chunker(text):
    print(bytes(chunk))

# with custom size
for chunk in Chunker(text, size=1024):
    print(bytes(chunk))

# with custom delimiters
for chunk in Chunker(text, delimiters=".?!\n"):
    print(bytes(chunk))

# with multi-byte pattern (e.g., metaspace ▁ for SentencePiece tokenizers)
for chunk in Chunker(text, pattern="▁", prefix=True):
    print(bytes(chunk))

# with consecutive pattern handling (split at START of runs, not middle)
for chunk in Chunker("word   next", pattern=" ", consecutive=True):
    print(bytes(chunk))

# with forward fallback (search forward if no pattern in backward window)
for chunk in Chunker(text, pattern=" ", forward_fallback=True):
    print(bytes(chunk))

# collect all chunks
chunks = list(Chunker(text))
```

chunks are returned as `memoryview` objects (zero-copy slices of the original text).

## 📝 citation

if you use chonkie-core in your research, please cite it as follows:

```bibtex
@software{chunk2025,
  author = {Minhas, Bhavnick},
  title = {chunk: The fastest text chunking library},
  year = {2025},
  publisher = {GitHub},
  howpublished = {\url{https://github.com/chonkie-inc/chunk}},
}
```

## 📄 license

licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.
