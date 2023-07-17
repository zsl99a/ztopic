use criterion::{black_box, criterion_group, criterion_main, Criterion};
use helium::VLock;

fn lock(n: usize) {
    let x = VLock::new();

    for _ in 0..n {
        x.lock();
    }
}

fn lock_benchmark(c: &mut Criterion) {
    c.bench_function("lock", |b| b.iter(|| lock(black_box(10000))));
}

criterion_group!(benches, lock_benchmark);
criterion_main!(benches);
