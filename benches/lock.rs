use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use criterion::{black_box, criterion_group, criterion_main, Criterion};

struct AtomicLock {
    is_locked: Arc<AtomicBool>,
}

impl AtomicLock {
    pub fn new() -> Self {
        Self {
            is_locked: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn try_lock(&self) -> Option<Self> {
        if !self.is_locked.load(Ordering::Relaxed) {
            if let Ok(_) = self.is_locked.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire) {
                return Some(Self {
                    is_locked: self.is_locked.clone(),
                });
            }
        }
        return None;
    }
}

impl Drop for AtomicLock {
    fn drop(&mut self) {
        self.is_locked.store(false, Ordering::Relaxed);
    }
}

fn lock(n: usize) {
    let x = AtomicLock::new();

    for _ in 0..n {
        x.try_lock();
    }
}

fn lock_benchmark(c: &mut Criterion) {
    c.bench_function("lock", |b| b.iter(|| lock(black_box(10000))));
}

criterion_group!(benches, lock_benchmark);
criterion_main!(benches);
