use std::sync::{
    atomic::{AtomicPtr, Ordering},
    Arc,
};

use criterion::{black_box, criterion_group, criterion_main, Criterion};

pub struct Xx {
    _a: i32,
}

impl Xx {
    pub fn new() -> Self {
        Self { _a: 0 }
    }
}

fn atomic(n: usize) {
    let x = Arc::new(AtomicPtr::new(Box::into_raw(Box::new(Xx::new()))));

    for e in 0..n {
        black_box(e);
        x.load(Ordering::Relaxed);
    }
}

fn atomic_benchmark(c: &mut Criterion) {
    c.bench_function("atomic", |b| b.iter(|| atomic(black_box(10000))));
}

criterion_group!(benches, atomic_benchmark);
criterion_main!(benches);
