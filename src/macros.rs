macro_rules! debug_unreachable {
    () => {
        debug_unreachable!("entered unreachable code")
    };
    ($e:expr) => {
        if cfg!(debug_assertions) {
            panic!($e);
        } else {
            unsafe {
                core::hint::unreachable_unchecked();
            }
        }
    };
}

/// Synchronously wait for an async expression to complete.
///
/// This macro is intended for use in contexts where async code cannot be used,
/// but the thread is running inside a Tokio runtime.
///
/// # Panics
///
/// This macro will panic if it is called outside of a Tokio runtime.
macro_rules! wait {
    ($e:expr) => {
        tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on($e))
    };
}

pub(crate) use {debug_unreachable, wait};
