use std::time::Instant;

fn main() {
    // Generate test data
    let base = b"Hello. World? Test! ";
    let text: Vec<u8> = base.iter().cycle().take(2_000_000).copied().collect();
    let patterns: &[&[u8]] = &[b". ", b"? ", b"! "];
    
    // Warmup
    for _ in 0..3 {
        let _ = chunk::split_at_patterns(&text, patterns, chunk::IncludeDelim::Prev, 0);
    }
    
    // Benchmark
    let iterations = 100;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = chunk::split_at_patterns(&text, patterns, chunk::IncludeDelim::Prev, 0);
    }
    let elapsed = start.elapsed();
    let avg_ms = elapsed.as_secs_f64() * 1000.0 / iterations as f64;
    let throughput = text.len() as f64 / avg_ms / 1000.0;
    
    println!("Text size: {} bytes", text.len());
    println!("split_at_patterns: {:.2} ms ({:.1} MB/s)", avg_ms, throughput);
    
    // Also benchmark single-byte split
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = chunk::split_at_delimiters(&text, b".?!", chunk::IncludeDelim::Prev, 0);
    }
    let elapsed = start.elapsed();
    let avg_ms = elapsed.as_secs_f64() * 1000.0 / iterations as f64;
    let throughput = text.len() as f64 / avg_ms / 1000.0;
    println!("split_at_delimiters: {:.2} ms ({:.1} MB/s)", avg_ms, throughput);
}
