use wasm_bindgen::prelude::*;

#[macro_export]
macro_rules! console_log {
    ($fmt:expr, $($arg1:expr),*) => {
        crate::macros::log_wrapper(&format!($fmt, $($arg1),+))
    };
    ($fmt:expr) => {
        crate::macros::log_wrapper($fmt)
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub(crate) fn log(s: &str);
}

/// rust-analyzer shows a lot of false positive errors in the editor where console_log macro is used
/// because it cannot parse procedural macros that allows extern "C" block to be non-unsafe.
/// It compiles just fine, but it's too noisy to pick up real errors in the editor, so we put an intermediate
/// layer of log function to avoid false detection.
/// Hopefully the optimizer is smart enough to remove this layer in compiled binary, and the actual calls to the
/// JS have much more overhead anyway.
///
/// Another option is to use js_sys::console, but it does not have variadic macros and it's too annoying to
/// write like `log_1(&format("...", a, b, ...))` everytime.
pub(crate) fn log_wrapper(s: &str) {
    log(s);
}

/// format-like macro that returns js_sys::String
#[macro_export]
macro_rules! js_str {
    ($fmt:expr, $($arg1:expr),*) => {
        JsValue::from_str(&format!($fmt, $($arg1),+))
    };
    ($fmt:expr) => {
        JsValue::from_str($fmt)
    }
}

/// format-like macro that returns Err(js_sys::String)
#[macro_export]
macro_rules! js_err {
    ($fmt:expr, $($arg1:expr),*) => {
        Err(JsValue::from_str(&format!($fmt, $($arg1),+)))
    };
    ($fmt:expr) => {
        Err(JsValue::from_str($fmt))
    }
}

#[macro_export]
macro_rules! hash_map {
    { $($key:expr => $value:expr),+ } => {
        {
            let mut m = ::std::collections::HashMap::new();
            $(
                m.insert($key, $value);
            )+
            m
        }
    };
    { } => {
        ::std::collections::HashMap::new()
    }
}

#[macro_export]
macro_rules! hash_set {
    { $($key:expr),+ } => {
        {
            let mut m = ::std::collections::HashSet::new();
            $(
                m.insert($key);
            )+
            m
        }
    };
    { } => {
        ::std::collections::HashSet::new()
    }
}
