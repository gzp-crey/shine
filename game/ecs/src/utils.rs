/// Imagine macro parameters, but more like those Russian dolls.
///
/// Calls m!(A, B, C), m!(A, B), m!(B), and m!() for i.e. (m, A, B, C)
/// where m is any macro, for any number of parameters.
#[macro_export]
macro_rules! smaller_tuples_too {
    ($m: ident, $ty: ident) => {
        $m!{$ty}
        $m!{}
    };
    ($m: ident, $ty: ident, $($tt: ident),*) => {
        $m!{$ty, $($tt),*}
        smaller_tuples_too!{$m, $($tt),*}
    };
}

/// Simmilar to debug_assert! but code is not validated when !cfg(debug_assertions)
#[macro_export]
macro_rules! dbg_assert {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        { debug_assert!($($arg)*) }
    };
}
