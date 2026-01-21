<p align="center">
  <img src="assets/memchunk_wide.png" alt="chunk" width="500">
</p>

<h1 align="center">chunk</h1>

<p align="center">
  <em>the fastest text chunking library — up to 1 TB/s throughput</em>
</p>

<p align="center">
  <a href="https://crates.io/crates/chunk"><img src="https://img.shields.io/crates/v/chunk.svg?color=e74c3c" alt="crates.io"></a>
  <a href="https://pypi.org/project/chonkie-core"><img src="https://img.shields.io/pypi/v/chonkie-core.svg?color=e67e22" alt="PyPI"></a>
  <a href="https://www.npmjs.com/package/@chonkiejs/chunk"><img src="https://img.shields.io/npm/v/@chonkiejs/chunk.svg?color=2ecc71" alt="npm"></a>
  <a href="https://docs.rs/chunk"><img src="https://img.shields.io/docsrs/chunk?color=3498db" alt="docs.rs"></a>
  <a href="LICENSE-MIT"><img src="https://img.shields.io/badge/license-MIT%2FApache--2.0-9b59b6.svg" alt="License"></a>
</p>

---

you know how every chunking library claims to be fast? yeah, we actually meant it.

**chunk** splits text at semantic boundaries (periods, newlines, the usual suspects) and does it stupid fast. we're talking "chunk the entire english wikipedia in 120ms" fast.

want to know how? [read the blog post](https://minha.sh/posts/so,-you-want-to-chunk-really-fast) where we nerd out about SIMD instructions and lookup tables.

<p align="center">
  <img src="assets/benchmark.png" alt="Benchmark comparison" width="700">
</p>

<p align="center">
  <em>See <a href="benches/">benches/</a> for detailed benchmarks.</em>
</p>

## 📦 Installation

```bash
cargo add chunk
```

looking for [python](https://github.com/chonkie-inc/chunk/tree/main/packages/python) or [javascript](https://github.com/chonkie-inc/chunk/tree/main/packages/wasm)?

## 🚀 Usage

```rust
use chunk::chunk;

let text = b"Hello world. How are you? I'm fine.\nThanks for asking.";

// With defaults (4KB chunks, split at \n . ?)
let chunks: Vec<&[u8]> = chunk(text).collect();

// With custom size
let chunks: Vec<&[u8]> = chunk(text).size(1024).collect();

// With custom delimiters
let chunks: Vec<&[u8]> = chunk(text).delimiters(b"\n.?!").collect();

// With multi-byte pattern (e.g., metaspace ▁ for SentencePiece tokenizers)
let metaspace = "▁".as_bytes();
let chunks: Vec<&[u8]> = chunk(text).pattern(metaspace).prefix().collect();

// With consecutive pattern handling (split at START of runs, not middle)
let chunks: Vec<&[u8]> = chunk(b"word   next")
    .pattern(b" ")
    .consecutive()
    .collect();

// With forward fallback (search forward if no pattern in backward window)
let chunks: Vec<&[u8]> = chunk(text)
    .pattern(b" ")
    .forward_fallback()
    .collect();
```

## 📝 Citation

If you use chunk in your research, please cite it as follows:

```bibtex
@software{chunk2025,
  author = {Minhas, Bhavnick},
  title = {chunk: The fastest text chunking library},
  year = {2025},
  publisher = {GitHub},
  howpublished = {\url{https://github.com/chonkie-inc/chunk}},
}
```

## 📄 License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.
