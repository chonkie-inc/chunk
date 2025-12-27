//! Benchmark pattern vs delimiter throughput

use memchunk::chunk;
use std::time::Instant;

fn main() {
    // Create test data - mix of spaces and metaspaces
    let base_text = "The quick brown fox jumps over the lazy dog. ";
    let metaspace = "▁";

    // Create ~100MB of test data with spaces
    let repeat_count = 100_000_000 / base_text.len();
    let text_with_spaces: String = base_text.repeat(repeat_count);
    let text_bytes_spaces = text_with_spaces.as_bytes();

    // Create same size text with metaspaces instead of spaces
    let base_with_metaspace = base_text.replace(' ', metaspace);
    let text_with_metaspace: String = base_with_metaspace.repeat(repeat_count);
    let text_bytes_metaspace = text_with_metaspace.as_bytes();

    let size_mb_spaces = text_bytes_spaces.len() as f64 / 1_000_000.0;
    let size_mb_metaspace = text_bytes_metaspace.len() as f64 / 1_000_000.0;

    println!("Benchmark: Pattern vs Delimiter Throughput");
    println!("==========================================");
    println!(
        "Text with spaces:    {:.2} MB ({} bytes)",
        size_mb_spaces,
        text_bytes_spaces.len()
    );
    println!(
        "Text with metaspace: {:.2} MB ({} bytes)",
        size_mb_metaspace,
        text_bytes_metaspace.len()
    );
    println!();

    const ITERATIONS: usize = 10;

    // Test with different chunk sizes
    for &chunk_size in &[1024, 4096, 16384, 65536] {
        println!("=== Chunk size: {} bytes ===", chunk_size);
        bench_delimiter(text_bytes_spaces, size_mb_spaces, chunk_size, ITERATIONS);
        bench_pattern(
            text_bytes_metaspace,
            size_mb_metaspace,
            metaspace.as_bytes(),
            chunk_size,
            ITERATIONS,
        );
        println!();
    }
}

fn bench_delimiter(text: &[u8], size_mb: f64, chunk_size: usize, iterations: usize) {
    // Warmup
    let _: Vec<_> = chunk(text)
        .size(chunk_size)
        .delimiters(b" ")
        .prefix()
        .collect();

    let mut times = Vec::new();
    let mut num_chunks = 0;
    for _ in 0..iterations {
        let start = Instant::now();
        let chunks: Vec<_> = chunk(text)
            .size(chunk_size)
            .delimiters(b" ")
            .prefix()
            .collect();
        let elapsed = start.elapsed();
        times.push(elapsed.as_secs_f64());
        num_chunks = chunks.len();
    }
    let avg = times.iter().sum::<f64>() / iterations as f64;
    let throughput = size_mb / avg / 1000.0; // GB/s
    println!(
        "  Single-byte (space):    {:>7.2} GB/s  ({:>6} chunks, {:.3}ms avg)",
        throughput,
        num_chunks,
        avg * 1000.0
    );
}

fn bench_pattern(text: &[u8], size_mb: f64, pattern: &[u8], chunk_size: usize, iterations: usize) {
    // Warmup
    let _: Vec<_> = chunk(text)
        .size(chunk_size)
        .pattern(pattern)
        .prefix()
        .collect();

    let mut times = Vec::new();
    let mut num_chunks = 0;
    for _ in 0..iterations {
        let start = Instant::now();
        let chunks: Vec<_> = chunk(text)
            .size(chunk_size)
            .pattern(pattern)
            .prefix()
            .collect();
        let elapsed = start.elapsed();
        times.push(elapsed.as_secs_f64());
        num_chunks = chunks.len();
    }
    let avg = times.iter().sum::<f64>() / iterations as f64;
    let throughput = size_mb / avg / 1000.0; // GB/s
    println!(
        "  Multi-byte (metaspace): {:>7.2} GB/s  ({:>6} chunks, {:.3}ms avg)",
        throughput,
        num_chunks,
        avg * 1000.0
    );
}
