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

macro_rules! wait {
    ($e:expr) => {
        tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on($e))
    };
}

pub(crate) use {debug_unreachable, wait};
