use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use kiru::{BytesChunker, Chunker as KiruChunker};

fn bench_kiru(c: &mut Criterion) {
    let text = std::fs::read_to_string("benches/data/enwik8").expect(
        "Failed to load enwik8. Run: cd benches/data && curl -O https://mattmahoney.net/dc/enwik8.zip && unzip enwik8.zip",
    );

    let mut group = c.benchmark_group("kiru_4KB");
    group.throughput(Throughput::Bytes(text.len() as u64));

    let kiru_chunker = BytesChunker::new(4096, 0).expect("Failed to create kiru chunker");
    group.bench_function("kiru", |b| {
        b.iter(|| {
            let chunks: Vec<_> = kiru_chunker
                .clone()
                .chunk_string(black_box(text.clone()))
                .collect();
            black_box(chunks)
        })
    });

    group.finish();
}

criterion_group!(benches, bench_kiru);
criterion_main!(benches);
