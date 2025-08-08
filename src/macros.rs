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

pub(crate) use debug_unreachable;
