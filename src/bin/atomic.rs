use std::sync::{
    atomic::{AtomicPtr, Ordering},
    Arc,
};

#[tokio::main]
async fn main() {
    let x = Arc::new(AtomicPtr::new(Box::into_raw(Box::new(Xx::new()))));

    for _ in 0..10000 {
        x.load(Ordering::Relaxed);
    }
}

pub struct Xx {
    _a: i32,
}

impl Xx {
    pub fn new() -> Self {
        Self { _a: 0 }
    }
}
