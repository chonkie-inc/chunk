use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use memchunk::chunk;

fn bench_enwik8(c: &mut Criterion) {
    let text = std::fs::read("benches/data/enwik8").expect(
        "Failed to load enwik8. Run: cd benches/data && curl -O https://mattmahoney.net/dc/enwik8.zip && unzip enwik8.zip",
    );

    let mut group = c.benchmark_group("enwik8");
    group.throughput(Throughput::Bytes(text.len() as u64));

    group.bench_function("4KB chunks", |b| {
        b.iter(|| {
            let chunks: Vec<_> = chunk(black_box(&text)).size(4096).collect();
            black_box(chunks)
        })
    });

    group.bench_function("16KB chunks", |b| {
        b.iter(|| {
            let chunks: Vec<_> = chunk(black_box(&text)).size(16384).collect();
            black_box(chunks)
        })
    });

    group.bench_function("32KB chunks", |b| {
        b.iter(|| {
            let chunks: Vec<_> = chunk(black_box(&text)).size(32768).collect();
            black_box(chunks)
        })
    });

    group.finish();
}

fn bench_enwik9(c: &mut Criterion) {
    let text = std::fs::read("benches/data/enwik9").expect(
        "Failed to load enwik9. Run: cd benches/data && curl -O https://mattmahoney.net/dc/enwik9.zip && unzip enwik9.zip",
    );

    let mut group = c.benchmark_group("enwik9");
    group.throughput(Throughput::Bytes(text.len() as u64));

    group.bench_function("4KB chunks", |b| {
        b.iter(|| {
            let chunks: Vec<_> = chunk(black_box(&text)).size(4096).collect();
            black_box(chunks)
        })
    });

    group.bench_function("16KB chunks", |b| {
        b.iter(|| {
            let chunks: Vec<_> = chunk(black_box(&text)).size(16384).collect();
            black_box(chunks)
        })
    });

    group.bench_function("32KB chunks", |b| {
        b.iter(|| {
            let chunks: Vec<_> = chunk(black_box(&text)).size(32768).collect();
            black_box(chunks)
        })
    });

    group.finish();
}

criterion_group!(benches, bench_enwik8, bench_enwik9);
criterion_main!(benches);
