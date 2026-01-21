<p align="center">
  <img src="../../assets/memchunk_wide.png" alt="@chonkiejs/chunk" width="500">
</p>

<h1 align="center">@chonkiejs/chunk</h1>

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

**@chonkiejs/chunk** splits text at semantic boundaries (periods, newlines, the usual suspects) and does it stupid fast. we're talking "chunk the entire english wikipedia in 120ms" fast.

want to know how? [read the blog post](https://minha.sh/posts/so,-you-want-to-chunk-really-fast) where we nerd out about SIMD instructions and lookup tables.

## 📦 installation

```bash
npm install @chonkiejs/chunk
```

looking for [rust](https://github.com/chonkie-inc/chunk) or [python](https://github.com/chonkie-inc/chunk/tree/main/packages/python)?

## 🚀 usage

```javascript
import { init, chunk } from '@chonkiejs/chunk';

// initialize wasm (required once)
await init();

const text = "Hello world. How are you? I'm fine.\nThanks for asking.";

// with defaults (4KB chunks, split at \n . ?)
for (const slice of chunk(text)) {
    console.log(slice);
}

// with custom size
for (const slice of chunk(text, { size: 1024 })) {
    console.log(slice);
}

// with custom delimiters
for (const slice of chunk(text, { delimiters: ".?!\n" })) {
    console.log(slice);
}

// with multi-byte pattern (e.g., metaspace ▁ for SentencePiece tokenizers)
for (const slice of chunk(text, { pattern: "▁", prefix: true })) {
    console.log(slice);
}

// with consecutive pattern handling (split at START of runs, not middle)
for (const slice of chunk("word   next", { pattern: " ", consecutive: true })) {
    console.log(slice);
}

// with forward fallback (search forward if no pattern in backward window)
for (const slice of chunk(text, { pattern: " ", forwardFallback: true })) {
    console.log(slice);
}

// collect all chunks
const chunks = [...chunk(text)];
```

pass strings and get strings back. for zero-copy performance with binary data, pass `Uint8Array` and you'll get `Uint8Array` views back.

## 📝 citation

if you use @chonkiejs/chunk in your research, please cite it as follows:

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
