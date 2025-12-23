<p align="center">
  <img src="assets/memchunk_wide.png" alt="memchunk" width="500">
</p>

<h1 align="center">memchunk</h1>

<p align="center">
  <em>the fastest text chunking library — up to 1 TB/s throughput</em>
</p>

<p align="center">
  <a href="https://crates.io/crates/memchunk"><img src="https://img.shields.io/crates/v/memchunk.svg" alt="crates.io"></a>
  <a href="https://pypi.org/project/memchunk"><img src="https://img.shields.io/pypi/v/memchunk.svg" alt="PyPI"></a>
  <a href="https://www.npmjs.com/package/memchunk"><img src="https://img.shields.io/npm/v/memchunk.svg" alt="npm"></a>
  <a href="https://docs.rs/memchunk"><img src="https://docs.rs/memchunk/badge.svg" alt="docs.rs"></a>
  <a href="LICENSE-MIT"><img src="https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg" alt="License"></a>
</p>

---

you know how every chunking library claims to be fast? yeah, we actually meant it.

**memchunk** splits text at semantic boundaries (periods, newlines, the usual suspects) and does it stupid fast. we're talking "chunk the entire english wikipedia in 120ms" fast.

want to know how? [read the blog post](https://minha.sh/posts/so-you-want-to-chunk-really-fast) where we nerd out about SIMD instructions and lookup tables.

<p align="center">
  <img src="assets/benchmark.png" alt="Benchmark comparison" width="700">
</p>

<p align="center">
  <em>See <a href="benches/">benches/</a> for detailed benchmarks.</em>
</p>

## 📦 Installation

```bash
cargo add memchunk
```

## 🚀 Usage

```rust
use memchunk::chunk;

let text = b"Hello world. How are you? I'm fine.\nThanks for asking.";

// With defaults (4KB chunks, split at \n . ?)
let chunks: Vec<&[u8]> = chunk(text).collect();

// With custom size
let chunks: Vec<&[u8]> = chunk(text).size(1024).collect();

// With custom delimiters
let chunks: Vec<&[u8]> = chunk(text).delimiters(b"\n.?!").collect();

// With both
let chunks: Vec<&[u8]> = chunk(text).size(8192).delimiters(b"\n").collect();
```

## 📝 Citation

If you use memchunk in your research, please cite it as follows:

```bibtex
@software{memchunk2025,
  author = {Minhas, Bhavnick},
  title = {memchunk: The fastest text chunking library},
  year = {2025},
  publisher = {GitHub},
  howpublished = {\url{https://github.com/chonkie-inc/memchunk}},
}
```

## 📄 License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.
