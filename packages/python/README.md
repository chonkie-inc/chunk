<p align="center">
  <img src="../../assets/memchunk_wide.png" alt="memchunk" width="500">
</p>

<h1 align="center">memchunk</h1>

<p align="center">
  <em>the fastest text chunking library — up to 1 TB/s throughput</em>
</p>

<p align="center">
  <a href="https://crates.io/crates/memchunk"><img src="https://img.shields.io/crates/v/memchunk.svg" alt="crates.io"></a>
  <a href="https://pypi.org/project/memchunk"><img src="https://img.shields.io/pypi/v/memchunk.svg" alt="PyPI"></a>
  <a href="https://www.npmjs.com/package/memchunk"><img src="https://img.shields.io/npm/v/memchunk.svg" alt="npm"></a>
  <a href="https://github.com/chonkie-inc/memchunk"><img src="https://img.shields.io/badge/github-memchunk-blue" alt="GitHub"></a>
  <a href="LICENSE-MIT"><img src="https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg" alt="License"></a>
</p>

---

you know how every chunking library claims to be fast? yeah, we actually meant it.

**memchunk** splits text at semantic boundaries (periods, newlines, the usual suspects) and does it stupid fast. we're talking "chunk the entire english wikipedia in 120ms" fast.

want to know how? [read the blog post](https://minha.sh/posts/so-you-want-to-chunk-really-fast) where we nerd out about SIMD instructions and lookup tables.

## 📦 installation

```bash
pip install memchunk
```

## 🚀 usage

```python
from memchunk import chunk

text = "Hello world. How are you? I'm fine.\nThanks for asking."

# with defaults (4KB chunks, split at \n . ?)
for slice in chunk(text):
    print(bytes(slice))

# with custom size
for slice in chunk(text, size=1024):
    print(bytes(slice))

# with custom delimiters
for slice in chunk(text, delimiters=".?!\n"):
    print(bytes(slice))

# with both
for slice in chunk(text, size=8192, delimiters="\n"):
    print(bytes(slice))

# collect all chunks
chunks = list(chunk(text))
```

chunks are returned as `memoryview` objects (zero-copy slices of the original text).

## 📝 citation

if you use memchunk in your research, please cite it as follows:

```bibtex
@software{memchunk2025,
  author = {Minhas, Bhavnick},
  title = {memchunk: The fastest text chunking library},
  year = {2025},
  publisher = {GitHub},
  howpublished = {\url{https://github.com/chonkie-inc/memchunk}},
}
```

## 📄 license

licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.
