#![allow(unused_macros, unused_imports)]

// TODO: consider making this local to cfg(test)

macro_rules! assert_matches {
    ($expression:expr, $pattern:pat) => {
        assert!(
            matches!($expression, $pattern),
            "{:?} does not match {}",
            $expression,
            stringify!($pattern)
        )
    };
    ($expression:expr, $pattern:pat, $($arg:tt)+) => {
        assert!(
            matches!($expression, $pattern),
            "{:?} does not match {}: {}",
            $expression,
            stringify!($pattern),
            format_args!($($arg)+)
        )
    };
}

#[cfg(channel = "nightly")]
pub(crate) use std::assert_matches::assert_matches;

#[cfg(not(channel = "nightly"))]
pub(crate) use assert_matches;

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
