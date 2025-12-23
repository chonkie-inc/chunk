import { test } from 'node:test';
import assert from 'node:assert';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));

// Load WASM module
const wasmPath = join(__dirname, '..', 'pkg', 'memchunk_wasm_bg.wasm');
const wasmBuffer = await readFile(wasmPath);

// Import the generated WASM module
const { Chunker, default_target_size, default_delimiters, initSync } = await import('../pkg/memchunk_wasm.js');
initSync(wasmBuffer);

const encoder = new TextEncoder();
const decoder = new TextDecoder();

test('basic chunking', () => {
    const text = encoder.encode("Hello. World. Test.");
    const chunker = new Chunker(text, 10, ".");
    const chunks = [];
    let chunk;
    while ((chunk = chunker.next()) !== undefined) {
        chunks.push(decoder.decode(chunk));
    }
    assert.strictEqual(chunks.length, 3);
    assert.strictEqual(chunks[0], "Hello.");
    assert.strictEqual(chunks[1], " World.");
    assert.strictEqual(chunks[2], " Test.");
    chunker.free();
});

test('newline delimiter', () => {
    const text = encoder.encode("Line one\nLine two\nLine three");
    const chunker = new Chunker(text, 15, "\n");
    const chunks = [];
    let chunk;
    while ((chunk = chunker.next()) !== undefined) {
        chunks.push(decoder.decode(chunk));
    }
    assert.strictEqual(chunks[0], "Line one\n");
    assert.strictEqual(chunks[1], "Line two\n");
    assert.strictEqual(chunks[2], "Line three");
    chunker.free();
});

test('no delimiter hard split', () => {
    const text = encoder.encode("abcdefghij");
    const chunker = new Chunker(text, 5, ".");
    const chunks = [];
    let chunk;
    while ((chunk = chunker.next()) !== undefined) {
        chunks.push(decoder.decode(chunk));
    }
    assert.strictEqual(chunks[0], "abcde");
    assert.strictEqual(chunks[1], "fghij");
    chunker.free();
});

test('empty text', () => {
    const text = encoder.encode("");
    const chunker = new Chunker(text, 10, ".");
    const chunks = [];
    let chunk;
    while ((chunk = chunker.next()) !== undefined) {
        chunks.push(chunk);
    }
    assert.strictEqual(chunks.length, 0);
    chunker.free();
});

test('text smaller than target', () => {
    const text = encoder.encode("Small");
    const chunker = new Chunker(text, 100, ".");
    const chunks = [];
    let chunk;
    while ((chunk = chunker.next()) !== undefined) {
        chunks.push(decoder.decode(chunk));
    }
    assert.strictEqual(chunks.length, 1);
    assert.strictEqual(chunks[0], "Small");
    chunker.free();
});

test('reset', () => {
    const text = encoder.encode("Hello. World.");
    const chunker = new Chunker(text, 10, ".");

    // First iteration
    const chunks1 = [];
    let chunk;
    while ((chunk = chunker.next()) !== undefined) {
        chunks1.push(decoder.decode(chunk));
    }

    // Reset and iterate again
    chunker.reset();
    const chunks2 = [];
    while ((chunk = chunker.next()) !== undefined) {
        chunks2.push(decoder.decode(chunk));
    }

    assert.deepStrictEqual(chunks1, chunks2);
    chunker.free();
});

test('default constants', () => {
    assert.strictEqual(default_target_size(), 4096);
    const delims = default_delimiters();
    assert.strictEqual(decoder.decode(delims), "\n.?");
});

test('total bytes preserved', () => {
    const text = encoder.encode("The quick brown fox jumps over the lazy dog. How vexingly quick!");
    const chunker = new Chunker(text, 20, "\n.?!");
    let total = 0;
    let chunk;
    while ((chunk = chunker.next()) !== undefined) {
        total += chunk.length;
    }
    assert.strictEqual(total, text.length);
    chunker.free();
});
