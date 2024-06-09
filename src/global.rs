pub(crate) static mut GLOBAL_CAPACITY: usize = 4096;

pub fn set_capacity(capacity: usize) {
    unsafe {
        GLOBAL_CAPACITY = capacity;
    }
}
