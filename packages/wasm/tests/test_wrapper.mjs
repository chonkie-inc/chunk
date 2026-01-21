import { test } from 'node:test';
import assert from 'node:assert';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));

// We need to manually initialize since we're in Node.js
const wasmPath = join(__dirname, '..', 'pkg', 'chonkiejs_chunk_bg.wasm');
const wasmBuffer = await readFile(wasmPath);

// Patch the init function to work in Node.js
import { initSync } from '../pkg/chonkiejs_chunk.js';
initSync({ module: wasmBuffer });

// Now import our wrapper
import { chunk, chunk_offsets, split, split_offsets, Chunker, default_target_size, default_delimiters } from '../index.js';

const encoder = new TextEncoder();
const decoder = new TextDecoder();

test('wrapper: basic chunking with iterator', () => {
    const text = encoder.encode("Hello. World. Test.");
    const chunker = new Chunker(text, { size: 10, delimiters: "." });
    const chunks = [...chunker].map(c => decoder.decode(c));
    assert.strictEqual(chunks.length, 3);
    assert.strictEqual(chunks[0], "Hello.");
    assert.strictEqual(chunks[1], " World.");
    assert.strictEqual(chunks[2], " Test.");
    chunker.free();
});

test('wrapper: for...of loop', () => {
    const text = encoder.encode("Hello. World. Test.");
    const chunker = new Chunker(text, { size: 10, delimiters: "." });
    const chunks = [];
    for (const chunk of chunker) {
        chunks.push(decoder.decode(chunk));
    }
    assert.strictEqual(chunks.length, 3);
    chunker.free();
});

test('wrapper: default options', () => {
    const text = encoder.encode("Hello world.");
    const chunker = new Chunker(text);
    const chunks = [...chunker];
    assert.strictEqual(chunks.length, 1);
    chunker.free();
});

test('wrapper: next method', () => {
    const text = encoder.encode("Hello. World.");
    const chunker = new Chunker(text, { size: 10, delimiters: "." });
    assert.strictEqual(decoder.decode(chunker.next()), "Hello.");
    assert.strictEqual(decoder.decode(chunker.next()), " World.");
    assert.strictEqual(chunker.next(), undefined);
    chunker.free();
});

test('wrapper: reset', () => {
    const text = encoder.encode("Hello. World.");
    const chunker = new Chunker(text, { size: 10, delimiters: "." });
    const chunks1 = [...chunker].map(c => decoder.decode(c));
    chunker.reset();
    const chunks2 = [...chunker].map(c => decoder.decode(c));
    assert.deepStrictEqual(chunks1, chunks2);
    chunker.free();
});

test('wrapper: constants exported', () => {
    assert.strictEqual(default_target_size(), 4096);
    assert.strictEqual(decoder.decode(default_delimiters()), "\n.?");
});

test('wrapper: chunk_offsets function', () => {
    const text = encoder.encode("Hello. World. Test.");
    const offsets = chunk_offsets(text, { size: 10, delimiters: "." });
    assert.strictEqual(offsets.length, 3);
    assert.deepStrictEqual(offsets[0], [0, 6]);
    assert.deepStrictEqual(offsets[1], [6, 13]);
    assert.deepStrictEqual(offsets[2], [13, 19]);

    // Verify slicing works
    const chunks = offsets.map(([start, end]) => decoder.decode(text.subarray(start, end)));
    assert.strictEqual(chunks[0], "Hello.");
    assert.strictEqual(chunks[1], " World.");
    assert.strictEqual(chunks[2], " Test.");
});

test('wrapper: Chunker.collectOffsets method', () => {
    const text = encoder.encode("Hello. World. Test.");
    const chunker = new Chunker(text, { size: 10, delimiters: "." });
    const offsets = chunker.collectOffsets();
    assert.strictEqual(offsets.length, 3);
    assert.deepStrictEqual(offsets[0], [0, 6]);
    assert.deepStrictEqual(offsets[1], [6, 13]);
    assert.deepStrictEqual(offsets[2], [13, 19]);
    chunker.free();
});

test('wrapper: chunk function returns zero-copy subarrays', () => {
    const text = encoder.encode("Hello. World. Test.");
    const chunks = [...chunk(text, { size: 10, delimiters: "." })];
    assert.strictEqual(chunks.length, 3);
    assert.strictEqual(decoder.decode(chunks[0]), "Hello.");
    assert.strictEqual(decoder.decode(chunks[1]), " World.");
    assert.strictEqual(decoder.decode(chunks[2]), " Test.");

    // Verify it's a subarray (shares buffer with original)
    assert.strictEqual(chunks[0].buffer, text.buffer);
});

test('wrapper: chunk function with for...of', () => {
    const text = encoder.encode("Hello. World. Test.");
    const results = [];
    for (const slice of chunk(text, { size: 10, delimiters: "." })) {
        results.push(decoder.decode(slice));
    }
    assert.deepStrictEqual(results, ["Hello.", " World.", " Test."]);
});

// Split tests
test('wrapper: split function basic', () => {
    const text = encoder.encode("Hello. World. Test.");
    const segments = [...split(text, { delimiters: "." })];
    assert.strictEqual(segments.length, 3);
    assert.strictEqual(decoder.decode(segments[0]), "Hello.");
    assert.strictEqual(decoder.decode(segments[1]), " World.");
    assert.strictEqual(decoder.decode(segments[2]), " Test.");
});

test('wrapper: split function with string input', () => {
    const segments = [...split("Hello. World. Test.", { delimiters: "." })];
    assert.strictEqual(segments.length, 3);
    assert.strictEqual(segments[0], "Hello.");
    assert.strictEqual(segments[1], " World.");
    assert.strictEqual(segments[2], " Test.");
});

test('wrapper: split function with includeDelim=next', () => {
    const segments = [...split("Hello. World. Test.", { delimiters: ".", includeDelim: "next" })];
    assert.strictEqual(segments.length, 4);
    assert.strictEqual(segments[0], "Hello");
    assert.strictEqual(segments[1], ". World");
    assert.strictEqual(segments[2], ". Test");
    assert.strictEqual(segments[3], ".");
});

test('wrapper: split function with includeDelim=none', () => {
    const segments = [...split("Hello. World. Test.", { delimiters: ".", includeDelim: "none" })];
    assert.strictEqual(segments.length, 3);
    assert.strictEqual(segments[0], "Hello");
    assert.strictEqual(segments[1], " World");
    assert.strictEqual(segments[2], " Test");
});

test('wrapper: split_offsets function', () => {
    const text = encoder.encode("Hello. World. Test.");
    const offsets = split_offsets(text, { delimiters: "." });
    assert.strictEqual(offsets.length, 3);
    assert.deepStrictEqual(offsets[0], [0, 6]);
    assert.deepStrictEqual(offsets[1], [6, 13]);
    assert.deepStrictEqual(offsets[2], [13, 19]);

    // Verify slicing works
    const segments = offsets.map(([start, end]) => decoder.decode(text.subarray(start, end)));
    assert.strictEqual(segments[0], "Hello.");
    assert.strictEqual(segments[1], " World.");
    assert.strictEqual(segments[2], " Test.");
});

test('wrapper: split_offsets with minChars', () => {
    const text = "A. B. C. D. E.";
    // Without minChars, we get 5 segments
    const offsets1 = split_offsets(text, { delimiters: "." });
    assert.strictEqual(offsets1.length, 5);

    // With minChars=4, short segments get merged
    const offsets2 = split_offsets(text, { delimiters: ".", minChars: 4 });
    assert.ok(offsets2.length < 5);
});

test('wrapper: split preserves all bytes', () => {
    const text = encoder.encode("The quick brown fox. Jumps over? The lazy dog!");
    const offsets = split_offsets(text, { delimiters: ".?!" });

    // Verify all bytes are accounted for
    const total = offsets.reduce((sum, [start, end]) => sum + (end - start), 0);
    assert.strictEqual(total, text.length);

    // Verify offsets are contiguous
    for (let i = 1; i < offsets.length; i++) {
        assert.strictEqual(offsets[i - 1][1], offsets[i][0]);
    }
});
