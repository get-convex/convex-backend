use std::{
    alloc::{
        alloc,
        alloc_zeroed,
        dealloc,
        Layout,
    },
    ffi::c_void,
    mem,
    ptr,
    sync::{
        atomic::{
            AtomicUsize,
            Ordering,
        },
        Arc,
    },
};

use deno_core::v8::{
    new_rust_allocator,
    Allocator,
    RustAllocatorVtable,
    UniqueRef,
};

pub struct ArrayBufferMemoryLimit {
    available: AtomicUsize,
    limit: usize,
}

impl ArrayBufferMemoryLimit {
    /// Returns the amount of memory used by ArrayBuffers.
    /// This can be an overestimate since it includes buffers that would be GC'd
    /// but haven't yet.
    pub fn used(&self) -> usize {
        self.limit
            .saturating_sub(self.available.load(Ordering::Relaxed))
    }

    fn consume(&self, amount: usize) -> bool {
        let mut limit = self.available.load(Ordering::Relaxed);
        loop {
            if limit < amount {
                crate::metrics::log_array_buffer_oom();
                return false;
            }
            match self.available.compare_exchange_weak(
                limit,
                limit - amount,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return true,
                Err(v) => limit = v,
            }
        }
    }
}

const ALIGNMENT: usize = mem::align_of::<libc::max_align_t>();

unsafe extern "C" fn allocate(handle: &ArrayBufferMemoryLimit, len: usize) -> *mut c_void {
    if handle.consume(len) {
        alloc_zeroed(Layout::from_size_align_unchecked(len, ALIGNMENT)).cast()
    } else {
        ptr::null_mut()
    }
}
unsafe extern "C" fn allocate_uninitialized(
    handle: &ArrayBufferMemoryLimit,
    len: usize,
) -> *mut c_void {
    if handle.consume(len) {
        alloc(Layout::from_size_align_unchecked(len, ALIGNMENT)).cast()
    } else {
        ptr::null_mut()
    }
}
unsafe extern "C" fn free(handle: &ArrayBufferMemoryLimit, data: *mut c_void, len: usize) {
    handle.available.fetch_add(len, Ordering::Relaxed);
    dealloc(
        data.cast(),
        Layout::from_size_align_unchecked(len, ALIGNMENT),
    );
}
unsafe extern "C" fn drop(handle: *const ArrayBufferMemoryLimit) {
    Arc::from_raw(handle);
}

pub fn limited_array_buffer_allocator(
    limit: usize,
) -> (Arc<ArrayBufferMemoryLimit>, UniqueRef<Allocator>) {
    unsafe {
        let limit = Arc::new(ArrayBufferMemoryLimit {
            available: AtomicUsize::new(limit),
            limit,
        });
        const VTABLE: RustAllocatorVtable<ArrayBufferMemoryLimit> = RustAllocatorVtable {
            allocate,
            allocate_uninitialized,
            free,
            drop,
        };
        let allocator = new_rust_allocator(Arc::into_raw(limit.clone()), &VTABLE);
        (limit, allocator)
    }
}
