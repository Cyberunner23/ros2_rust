use std::ffi::CString;
use std::os::raw::c_int;
use std::sync::{Arc, Mutex};

use lazy_static::lazy_static;

use crate::rcl_bindings::*;
use crate::{RclrsError, ToResult};
use crate::context::Context;

use super::LogSeverity;

lazy_static! {
    // rcl itself holds a NON-THREAD-SAFE global logging context.
    // Therefore, it is our job to ensure thread safety when calling rcl logging functions.
    // Concretely, we must ensure thread safety when:
    //   * Initializing rcl logging.
    //   * Uninitializing rcl logging.
    //   * Setting the log level
    //   * Sending logs the output handlers (calls to rcl_logging_multiple_output_handler).
    // It is also our job to ensure rcl logging cannot be initialized if it is already initialized.
    // Option signifies whether logging is initialized or not.
    static ref GLOBAL_LOG_CONTEXT: Arc<Mutex<Option<LogContext>>> = Arc::new(Mutex::new(None));
}

pub(crate) struct LogContext;

impl Drop for LogContext {
    fn drop(&mut self) {

        // THREAD SAFETY: Satisfies requirement to lock on uninitialize.
        let global_context = GLOBAL_LOG_CONTEXT.clone();
        let mut global_context_guard = global_context.lock().unwrap();

        if global_context_guard.is_none() {
            // Context already uninitialized.
            return;
        }

        unsafe {
            // SAFETY: Fini is expected for an initialized logging system.
            let _unused = rcl_logging_fini().ok();
        }

        // Marks rcl logging as uninitialized.
        *global_context_guard = None;
    }
}

impl LogContext {
    pub(crate) fn init(rcl_context: &Context) -> Result<(), RclrsError> {
    // THREAD SAFETY:
    //   Satisfies requirement to lock on initialize.
    //   There exists a dependency on rcl context,
    //   however rcl context does not depend on our (rclrs) logging mutex.
    //   Therefore, deadlocks are not possible.
    let global_context = GLOBAL_LOG_CONTEXT.clone();
    let mut global_context_guard = global_context.lock().unwrap();

    if global_context_guard.is_some() {
        // Context already created.
        return Ok(());
    }

    {
        // THREAD SAFETY:
        //   This is our dependency on rcl context,
        //   however rcl context does not depend on our (rclrs) logging mutex.
        //   Therefore, deadlocks are not possible.
        let rcl_context_mtx = rcl_context.rcl_context_mtx.clone();
        let rcl_context_mtx_guard = rcl_context_mtx.lock().unwrap();

        unsafe {
            // SAFETY: No preconditions for this function.
            let allocator = rcutils_get_default_allocator();

            // SAFETY:
            //   It is expected to pass the global_arguments of a non-zero initialized rcl context.
            //   It is expected to pass a non-zero initialized allocator.
            //   It is expected to pass a callback for output handling.
            rcl_logging_configure_with_output_handler(
                &rcl_context_mtx_guard.global_arguments,
                &allocator,
                Some(rclrc_logging_output_handler)
            ).ok()?;
        }
    }

    *global_context_guard = Some(LogContext {});
    Ok(())
    }

    pub(crate) fn set_logger_level(logger_name: &str, severity: LogSeverity) -> Result<(), RclrsError> {

        // THREAD SAFETY: Satisfires requirement to lock when setting the logger level.
        let global_context = GLOBAL_LOG_CONTEXT.clone();
        let _unused = global_context.lock().unwrap();

        let logger_name = CString::new(logger_name).unwrap();
        let logger_severity = severity as c_int;

        unsafe {
            // SAFETY:
            //   name is safe as it expects a non-null C style string.
            //   level is safe as it expects a value that is one of RCUTILS_LOG_SEVERITY_*.
            rcutils_logging_set_logger_level(logger_name.as_ptr(), logger_severity).ok()
        }
    }

    pub(crate) fn log(fn_name: &str, file_name: &str, line_num: u32, name: &str, severity: LogSeverity, message: &str) {

        // THREAD SAFETY:
        //   Satisfires requirement to lock on log output handling.
        //   Normally the mutex is only applied in rclrc_logging_output_handler,
        //     however, upon further analysis, it seems that there is thread unsafe code
        //     between the call to rcutils_log and the call to rclrc_logging_output_handler.
        let global_logging_context = GLOBAL_LOG_CONTEXT.clone();
        let global_logging_context_guard = global_logging_context.lock().unwrap();
        if global_logging_context_guard.is_none() {
            // Logging not initialized.
            return;
        }

        let log_function_name = CString::new(fn_name).unwrap();
        let log_file_name = CString::new(file_name).unwrap();
        let log_location_ptr = Box::into_raw(
            Box::new(rcutils_log_location_t {
                function_name: log_function_name.as_ptr(),
                file_name: log_file_name.as_ptr(),
                line_number: line_num as usize
        }));

        let log_name = CString::new(name).unwrap();
        let log_message = CString::new(message).unwrap();
        unsafe {
            // SAFETY:
            //   location is safe as it expects a non-null,
            //     initialized *const rcutils_log_location_t.
            //   severity is safe as it expects a value that is one of RCUTILS_LOG_SEVERITY_*.
            //   name is safe as it expects a non-null C style string.
            //   format is safe as it expects a non-null C style string.
            //   ... is safe as it is used for C style string formatting and may be empty.
            rcutils_log(log_location_ptr, severity as c_int, log_name.as_ptr(), log_message.as_ptr());

            // SAFETY: Safe as rcutils_log does not deallocate log_location_ptr.
            // NOTE: Used to safely drop the rcutils_log_location_t instance.
            let _ = Box::from_raw(log_location_ptr);
        }
    }
}

#[allow(dead_code)]
unsafe extern "C" fn rclrc_logging_output_handler(
    location: *const rcutils_log_location_t,
    severity: ::std::os::raw::c_int,
    name: *const ::std::os::raw::c_char,
    timestamp: rcutils_time_point_value_t,
    format: *const ::std::os::raw::c_char,
    args: *mut va_list) {

    // SAFETY:
    //   This call is safe if the call to rcutils_log is safe.
    //   We simply forward the parameters and apply a mutex.
    //   TODO?: Find a way to verify instead of assuming here.
    // THREAD SAFETY:
    //   Requirement to lock on output handling already satisfied
    //     as the call to rcutils_log is mutexed.
    //   Normally the mutex is only applied here, however, upon further analysis,
    //     it seems that there may be thread unsafe code between the call to rcutils_log
    //     and the call to the present callback.
    rcl_logging_multiple_output_handler(location, severity, name, timestamp, format, args);
}
