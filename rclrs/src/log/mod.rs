use crate::rcl_bindings::RCUTILS_LOG_SEVERITY;

pub(crate) mod log_context;
pub(crate) mod logger;

pub enum LogSeverity {
    Debug = RCUTILS_LOG_SEVERITY::RCUTILS_LOG_SEVERITY_DEBUG as isize,
    Info = RCUTILS_LOG_SEVERITY::RCUTILS_LOG_SEVERITY_INFO as isize,
    Warn = RCUTILS_LOG_SEVERITY::RCUTILS_LOG_SEVERITY_WARN as isize,
    Error = RCUTILS_LOG_SEVERITY::RCUTILS_LOG_SEVERITY_ERROR as isize,
    Fatal = RCUTILS_LOG_SEVERITY::RCUTILS_LOG_SEVERITY_FATAL as isize,
}

// Hack to get the name of a function as rust has no built in method to do so.
macro_rules! function {
    () => {{
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of(f);
        name.strip_suffix("::f").unwrap()
    }};
}

macro_rules! log {
    ($severity: expr, $name: expr, $($arg:tt)*) => {
        crate::log::log_utils::rclrs_log(
            function!(), 
            file!(), 
            line!(), 
            $severity, 
            $name, 
            format!($($arg:tt)*).as_str()
        );
    };
}
