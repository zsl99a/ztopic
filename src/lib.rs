mod buffer;
mod empty;
mod stream;
mod time;
mod topic;

pub use {empty::*, stream::*, time::*, topic::*};

pub(crate) static mut GLOBAL_CAPACITY: usize = 128;
pub(crate) static mut GLOBAL_BATCH_SIZE: usize = 16;

pub fn set_capacity(capacity: usize) {
    unsafe {
        GLOBAL_CAPACITY = capacity;
    }
}

pub fn set_batch_size(batch_size: usize) {
    unsafe {
        GLOBAL_BATCH_SIZE = batch_size;
    }
}
