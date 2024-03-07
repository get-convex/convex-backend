//! Helpers for system tracing.

pub use cstr::cstr;

#[cfg(feature = "tracy-tracing")]
mod tracing_on {
    use std::{
        cell::RefCell,
        cmp::Reverse,
        collections::BinaryHeap,
        ffi::{
            CStr,
            CString,
        },
        future::Future,
        mem,
        pin::Pin,
        sync::{
            atomic::{
                AtomicU64,
                Ordering,
            },
            LazyLock,
        },
        task::{
            Context,
            Poll,
        },
    };

    use parking_lot::Mutex;
    use pin_project::{
        pin_project,
        pinned_drop,
    };
    use tracy_client::{
        sys,
        Client,
        SpanLocation,
    };

    pub fn initialize() {
        Client::start();
    }

    // Tracy has a notion of "fibers" which are intended to last for the duration of
    // the program. For example, fiber names must be `&'static CStr` in their
    // interface. We'll cheat a little bit here and have a `static` that
    // maintains heap allocated `CString`s internally.
    //
    // Then, when starting a new future, we find the lowest free fiber number and
    // use its name.
    struct FreeFibers {
        names: Vec<CString>,
        free: BinaryHeap<Reverse<usize>>,
    }

    impl FreeFibers {
        fn new() -> Self {
            Self {
                names: vec![],
                free: BinaryHeap::new(),
            }
        }

        fn alloc(&mut self) -> (usize, &CStr) {
            if let Some(Reverse(i)) = self.free.pop() {
                return (i, &self.names[i][..]);
            }
            let i = self.names.len();
            self.names
                .push(CString::new(format!("fiber-{}", i + 1).into_bytes()).unwrap());
            (i, &self.names[i][..])
        }

        fn get(&self, i: usize) -> &CStr {
            &self.names[i][..]
        }

        fn free(&mut self, i: usize) {
            self.free.push(Reverse(i));
        }
    }

    static FIBER_NAMES: LazyLock<Mutex<FreeFibers>> =
        LazyLock::new(|| Mutex::new(FreeFibers::new()));

    // Allocate a unique ID for each future to help find them in traces.
    static NEXT_FUTURE_ID: AtomicU64 = AtomicU64::new(0);

    // Tracy's fiber scheduling model is a little different than `Future`'s. It
    // expects only a single fiber to be scheduled on a thread at a time, where
    // a `Future`'s `poll` method will invoke other futures, causing "nesting"
    // of these asynchronous objects. We turn the nested `Future::poll` calls
    // into the non-overlapping fibers Tracy expects by using a thread-local to
    // keep track of the current fiber, stash it when entering a new one, and
    // restore it whne existing the old one.
    thread_local!(static CURRENT_FIBER: RefCell<Option<usize>> = RefCell::new(None));

    #[pin_project(PinnedDrop)]
    pub struct InstrumentedFuture<F: Future> {
        name: &'static CStr,
        loc: &'static SpanLocation,

        future_id: u64,
        // Filled out on first `poll`.
        st: Option<State>,

        #[pin]
        inner: F,
    }

    struct State {
        fiber_ix: usize,
        ctx: sys::___tracy_c_zone_context,
    }

    impl<F: Future> InstrumentedFuture<F> {
        pub fn new(inner: F, name: &'static CStr, loc: &'static SpanLocation) -> Self {
            initialize();
            Self {
                name,
                loc,
                future_id: NEXT_FUTURE_ID.fetch_add(1, Ordering::SeqCst),
                st: None,
                inner,
            }
        }
    }

    impl<F: Future> Future for InstrumentedFuture<F> {
        type Output = F::Output;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let this = self.project();

            // First, leave our parent fiber's execution, and stash its number on the stack
            // in `current_fiber`.
            let current_fiber = CURRENT_FIBER.with(|f| f.borrow_mut().take());
            if current_fiber.is_some() {
                unsafe {
                    sys::___tracy_fiber_leave();
                }
            }

            // Next, get our fiber number, initializing it if we're being polled for the
            // first time, and enter the fiber.
            let fiber_ix = match this.st {
                Some(State { fiber_ix, .. }) => {
                    let fiber_names = FIBER_NAMES.lock();
                    let fiber_name = fiber_names.get(*fiber_ix);
                    unsafe { sys::___tracy_fiber_enter(fiber_name.as_ptr()) };
                    *fiber_ix
                },
                None => {
                    let mut fiber_names = FIBER_NAMES.lock();
                    let (fiber_ix, fiber_name) = fiber_names.alloc();

                    // NB: Tracy's APIs that only take a single pointer expect its lifetime to be
                    // static. APIs that take in a pointer and length do a copy internally and only
                    // expect a borrow.
                    unsafe { sys::___tracy_fiber_enter(fiber_name.as_ptr()) };

                    // Create a zone for the fiber's entire duration. We'll finish it when the
                    // `Future` completes below or in the destructor.
                    let ctx = unsafe {
                        // Major HAX: Get at the private `data` field with `mem::transmute`.
                        struct _SpanLocation {
                            _function_name: CString,
                            data: sys::___tracy_source_location_data,
                        }
                        assert_eq!(
                            mem::size_of::<SpanLocation>(),
                            mem::size_of::<_SpanLocation>()
                        );
                        let loc = mem::transmute::<&SpanLocation, &_SpanLocation>(*this.loc);
                        sys::___tracy_emit_zone_begin(&loc.data as *const _, 1)
                    };
                    unsafe {
                        sys::___tracy_emit_zone_name(
                            ctx,
                            this.name.as_ptr(),
                            this.name.to_bytes().len(),
                        );
                        sys::___tracy_emit_zone_value(ctx, *this.future_id);
                    }
                    *this.st = Some(State { fiber_ix, ctx });
                    fiber_ix
                },
            };

            // Set ourselves as the current fiber in the thread local.
            CURRENT_FIBER.with(|f| *f.borrow_mut() = Some(fiber_ix));

            // Poll our future, which may end up polling other instrumented futures.
            let r = this.inner.poll(cx);

            // Finish our future's zone and free our fiber name if we're done.
            if r.is_ready() {
                let mut fiber_names = FIBER_NAMES.lock();
                let st = this.st.take().unwrap();
                unsafe {
                    sys::___tracy_emit_zone_end(st.ctx);
                }
                fiber_names.free(st.fiber_ix);
            }

            // Leave our current fiber and reenter our parent.
            unsafe { sys::___tracy_fiber_leave() }
            if let Some(current_fiber) = current_fiber {
                let fiber_names = FIBER_NAMES.lock();
                let fiber_name = fiber_names.get(current_fiber);
                unsafe { sys::___tracy_fiber_enter(fiber_name.as_ptr()) };
            }

            // Restore our parent fiber in the thread local.
            CURRENT_FIBER.with(|f| {
                let mut f = f.borrow_mut();
                assert_eq!(*f, Some(fiber_ix));
                *f = current_fiber;
            });

            r
        }
    }

    #[pinned_drop]
    impl<F: Future> PinnedDrop for InstrumentedFuture<F> {
        fn drop(self: Pin<&mut Self>) {
            let this = self.project();
            if let Some(st) = this.st.take() {
                let mut fiber_names = FIBER_NAMES.lock();
                unsafe {
                    let fiber_name = fiber_names.get(st.fiber_ix);
                    sys::___tracy_fiber_enter(fiber_name.as_ptr());
                    sys::___tracy_emit_zone_end(st.ctx);
                    sys::___tracy_fiber_leave();
                }
                fiber_names.free(st.fiber_ix);
            }
        }
    }

    #[macro_export]
    macro_rules! static_span {
        () => {{
            $crate::tracing::initialize();
            $crate::tracing::tracy_client::span!()
        }};
        ($name:expr) => {{
            $crate::tracing::initialize();
            $crate::tracing::tracy_client::span!($name)
        }};
    }

    #[macro_export]
    macro_rules! span_location {
        () => {
            $crate::tracing::tracy_client::span_location!()
        };
    }

    #[macro_export]
    macro_rules! instrument {
        ($name:expr, $future:expr) => {
            $crate::tracing::InstrumentedFuture::new(
                $future,
                $crate::tracing::cstr!($name),
                $crate::span_location!(),
            )
        };
    }
}
#[cfg(feature = "tracy-tracing")]
pub use tracy_client;

#[cfg(feature = "tracy-tracing")]
pub use self::tracing_on::{
    initialize,
    InstrumentedFuture,
};

#[cfg(not(feature = "tracy-tracing"))]
mod tracing_off {
    use std::{
        ffi::CStr,
        future::Future,
        pin::Pin,
        task::{
            Context,
            Poll,
        },
    };

    use pin_project::pin_project;

    pub fn initialize() {}

    #[pin_project]
    pub struct InstrumentedFuture<F: Future> {
        #[pin]
        inner: F,
    }

    pub struct NoopLocation;

    impl<F: Future> InstrumentedFuture<F> {
        pub fn new(inner: F, _name: &'static CStr, _loc: &'static NoopLocation) -> Self {
            Self { inner }
        }
    }

    impl<F: Future> Future for InstrumentedFuture<F> {
        type Output = F::Output;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let this = self.project();
            this.inner.poll(cx)
        }
    }

    pub struct NoopSpan;

    #[macro_export]
    macro_rules! static_span {
        () => {
            $crate::tracing::NoopSpan
        };
        ($name:expr) => {
            $crate::tracing::NoopSpan
        };
    }

    #[macro_export]
    macro_rules! span_location {
        () => {{
            const LOC: $crate::tracing::NoopLocation = $crate::tracing::NoopLocation;
            &LOC
        }};
    }

    #[macro_export]
    macro_rules! instrument {
        ($name:expr, $future:expr) => {
            $future
        };
    }
}

#[cfg(not(feature = "tracy-tracing"))]
pub use self::tracing_off::{
    initialize,
    InstrumentedFuture,
    NoopLocation,
    NoopSpan,
};
