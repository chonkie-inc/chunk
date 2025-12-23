use std::time::Duration;

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use text_splitter::TextSplitter;

fn bench_text_splitter(c: &mut Criterion) {
    let text = std::fs::read_to_string("benches/data/enwik8").expect(
        "Failed to load enwik8. Run: cd benches/data && curl -O https://mattmahoney.net/dc/enwik8.zip && unzip enwik8.zip",
    );

    let mut group = c.benchmark_group("text_splitter_4KB");
    group.throughput(Throughput::Bytes(text.len() as u64));
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(60));

    let splitter = TextSplitter::new(4096);
    group.bench_function("text-splitter", |b| {
        b.iter(|| {
            let chunks: Vec<_> = splitter.chunks(black_box(&text)).collect();
            black_box(chunks)
        })
    });

    group.finish();
}

criterion_group!(benches, bench_text_splitter);
criterion_main!(benches);
