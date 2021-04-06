use parking_lot::Mutex;
use pin_project_lite::pin_project;
use std::{cell::RefCell, future::Future, mem, sync::Arc};

pub use profiling_futures_macros::wrap;
pub use tracing;

thread_local! {
    pub static SIDE_CHANNEL: RefCell<Arc<Mutex<Storage>>> = RefCell::new(Storage::new());
}

#[macro_export]
macro_rules! enter {
    ($name:expr) => {
        $crate::enter_unguarded!($name);
        let _guard = $crate::Guard;
    };
}

#[macro_export]
macro_rules! enter_unguarded {
    ($name:expr) => {
        let span = $crate::tracing::span!($crate::tracing::Level::INFO, $name);
        $crate::SIDE_CHANNEL.with(|side| {
            let borrow1 = side.borrow();
            let mut borrow2 = borrow1.try_lock().expect("no contention is expected");
            borrow2.active.push($crate::Active { span: span.entered() });
        });
    };
}

#[macro_export]
macro_rules! exit {
    () => {
        $crate::SIDE_CHANNEL.with(|side| {
            let borrow1 = side.borrow();
            let mut borrow2 = borrow1.try_lock().expect("no contention is expected");
            borrow2.active.pop();
        })
    };
}

pub struct Guard;

impl Drop for Guard {
    fn drop(&mut self) {
        crate::exit!();
    }
}

#[derive(Debug)]
pub struct Active {
    pub span: tracing::span::EnteredSpan,
}

unsafe impl Send for Active {}
unsafe impl Sync for Active {}

impl From<Waiting> for Active {
    fn from(w: Waiting) -> Self {
        Self { span: w.span.entered() }
    }
}

#[derive(Debug)]
pub struct Waiting {
    pub span: tracing::Span,
}

impl From<Active> for Waiting {
    fn from(a: Active) -> Self {
        Self { span: a.span.exit() }
    }
}

#[derive(Debug)]
pub struct Storage {
    pub active: Vec<Active>,
    pub waiting: Vec<Waiting>,
}

impl Storage {
    fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            active: Vec::new(),
            waiting: Vec::new(),
        }))
    }
}

pin_project! {
    #[derive(Debug)]
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct FutureWrapper<T> {
        #[pin]
        inner: T,
        storage: Arc<Mutex<Storage>>,
    }
}

impl<T> FutureWrapper<T> {
    pub fn new(fut: T) -> Self {
        Self {
            inner: fut,
            storage: Storage::new(),
        }
    }
}

impl<T> Future for FutureWrapper<T>
where
    T: Future,
{
    type Output = T::Output;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        let this = self.project();

        let old_side_channel = SIDE_CHANNEL.with(|side| {
            let mut borrow = side.borrow_mut();
            mem::replace(&mut *borrow, Arc::clone(&this.storage))
        });

        let mut borrow_guard = this.storage.try_lock().expect("no contention is expected");
        let borrow = &mut *borrow_guard;
        borrow.active.extend(borrow.waiting.drain(..).map(Active::from));
        drop(borrow_guard);

        let res = this.inner.poll(cx);

        let mut borrow_guard = this.storage.try_lock().expect("no contention is expected");
        let borrow = &mut *borrow_guard;
        borrow.waiting.extend(borrow.active.drain(..).map(Waiting::from));
        drop(borrow_guard);

        SIDE_CHANNEL.with(|side| {
            let mut borrow = side.borrow_mut();
            *borrow = old_side_channel
        });

        res
    }
}
