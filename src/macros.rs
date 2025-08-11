macro_rules! assert_matches {
    ($left:expr, $(|)? $( $pattern:pat_param )|+ $( if $guard: expr )? $(,)?) => {
        match $left {
            $( $pattern )|+ $( if $guard )? => {}
            ref left_val => {
                $crate::macros::assert_matches_failed(
                    left_val,
                    stringify!($($pattern)|+ $(if $guard)?),
                    ::std::option::Option::None,
                );
            }
        }
    };
    ($left:expr, $(|)? $( $pattern:pat_param )|+ $( if $guard: expr )?, $($arg:tt)+) => {
        match $left {
            $( $pattern )|+ $( if $guard )? => {}
            ref left_val => {
                $crate::macros::assert_matches_failed(
                    left_val,
                    stringify!($($pattern)|+ $(if $guard)?),
                    ::std::option::Option::Some(format_args!($($arg)+)),
                );
            }
        }
    };
}

#[inline(never)]
#[cold]
#[track_caller]
#[doc(hidden)]
pub fn assert_matches_failed(
    left: &dyn core::fmt::Debug,
    right: &str,
    args: Option<std::fmt::Arguments<'_>>,
) {
    match args {
        Some(args) => panic!(
            r#"assertion `left matches right` failed: {args}
  left: {left:?}
 right: {right:?}"#,
            args = args,
            left = left,
            right = right,
        ),
        None => panic!(
            r#"assertion `left matches right` failed:
  left: {left:?}
 right: {right:?}"#,
            left = left,
            right = right,
        ),
    }
}

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

pub(crate) use {assert_matches, debug_unreachable};
