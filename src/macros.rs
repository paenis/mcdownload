#![allow(unused_macros, unused_imports)]

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
