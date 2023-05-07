use std::sync::{Arc, Mutex};

use lazy_static::lazy_static;

use crate::rcl_bindings::*;
use crate::{RclrsError, ToResult};
use crate::context::Context;

lazy_static! {
    // rcl itself holds a NON-THREAD-SAFE global logging context.
    // Therefore, it is our job to ensure thread safety when calling rcl logging functions.
    // Concretely, we must ensure thread safety when:
    //     * Initializing rcl logging.
    //     * Uninitializing rcl logging.
    //     * Sending logs the output handlers (calls to rcl_logging_multiple_output_handler).
    // It is also our job to ensure rcl logging cannot be initialized if it is already initialized.
    // Option signifies whether logging is initialized or not.
    pub(crate) static ref GLOBAL_LOG_CONTEXT: Arc<Mutex<Option<LogContext>>> = Arc::new(Mutex::new(None));
}

// Exists only to uninitialize the logging system on exit.
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

pub(crate) fn rclrs_initialize_logging(rcl_context: &Context) -> Result<(), RclrsError> {

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

#[allow(dead_code)]
unsafe extern "C" fn rclrc_logging_output_handler(
    location: *const rcutils_log_location_t,
    severity: ::std::os::raw::c_int,
    name: *const ::std::os::raw::c_char,
    timestamp: rcutils_time_point_value_t,
    format: *const ::std::os::raw::c_char,
    args: *mut va_list) {

    // THREAD SAFETY: Satisfies requirement to lock on output handling.
    let global_context = GLOBAL_LOG_CONTEXT.clone();
    let global_context_guard = global_context.lock().unwrap();

    if global_context_guard.is_none() {
        // Logging not initialized.
        return;
    }

    // SAFETY: This call is safe if the call to rcutils_log is safe.
    //         We simply forward the parameters and apply a mutex.
    //         TODO?: Find a way to verify instead of assuming here.
    rcl_logging_multiple_output_handler(location, severity, name, timestamp, format, args);
}
