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
npm install memchunk
```

## 🚀 usage

```javascript
import { init, chunk } from 'memchunk';

// initialize wasm (required once)
await init();

const text = new TextEncoder().encode("Hello world. How are you? I'm fine.\nThanks for asking.");

// with defaults (4KB chunks, split at \n . ?)
for (const slice of chunk(text)) {
    console.log(new TextDecoder().decode(slice));
}

// with custom size
for (const slice of chunk(text, { size: 1024 })) {
    console.log(new TextDecoder().decode(slice));
}

// with custom delimiters
for (const slice of chunk(text, { delimiters: ".?!\n" })) {
    console.log(new TextDecoder().decode(slice));
}

// with both
for (const slice of chunk(text, { size: 8192, delimiters: "\n" })) {
    console.log(new TextDecoder().decode(slice));
}

// collect all chunks
const chunks = [...chunk(text)];
```

chunks are returned as `Uint8Array` subarrays (zero-copy views of the original text).

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
