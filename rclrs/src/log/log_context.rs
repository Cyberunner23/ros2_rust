use std::ptr::null;
use std::sync::{Arc, Mutex};

use lazy_static::lazy_static;

use crate::rcl_bindings::*;
use crate::{RclrsError, ToResult};
use crate::context::Context;

lazy_static! {
    // rcl itself hold a NON-THREAD-SAFE global logging context.
    // Therefore, it is our job to ensure thread safety when calling rcl logging functions.
    // Concretely, we must ensure thread safety when:
    //     * Initializing rcl logging.
    //     * Uninitializing rcl logging.
    //     * Sending logs the output handlers (calls to rcl_logging_multiple_output_handler).
    // It is also our job to ensure rcl logging cannot be initialized if it is already initialized.
    // Option signifies whether logging is initialized or not.
    static ref GLOBAL_LOG_CONTEXT: Arc<Mutex<Option<LogContext>>> = Arc::new(Mutex::new(None));
}

// 
pub(crate) struct LogContext;

impl Drop for LogContext {
    fn drop(&mut self) {

        // THREAD SAFETY: Satisfies requirement to lock on uninitialize
        let global_context = GLOBAL_LOG_CONTEXT.clone();
        let mut global_context_guard = global_context.lock().unwrap();

        if global_context_guard.is_none() {
            // Context already uninitialized
            return;
        }

        unsafe {
            // SAFETY: Fini is expected for an initialized logging system
            let _unused = rcl_logging_fini().ok();
        }

        // Marks rcl logging as uninitialized
        *global_context_guard = None;
    }
}

impl LogContext {
    pub(crate) fn new() -> Self {
        Self {}
    }

    pub(crate) fn init(rcl_context: &Context, allocator: &rcutils_allocator_t) -> Result<(), RclrsError> {

        // THREAD SAFETY: Satisfies requirement to lock on initialize
        let global_context = GLOBAL_LOG_CONTEXT.clone();
        let mut global_context_guard = global_context.lock().unwrap();

        if global_context_guard.is_some() {
            // Context already created
            return Ok(());
        }
        
        {
            let rcl_context_mtx = rcl_context.rcl_context_mtx.clone();
            let rcl_context_mtx_guard = rcl_context_mtx.lock().unwrap();

            unsafe {
                // SAFETY: !! TODO: SAFETY ANALYSIS !!
                rcl_logging_configure_with_output_handler(
                    &rcl_context_mtx_guard.global_arguments,
                    allocator,
                    Some(rclrc_logging_output_handler)).ok()?;
            }
        }

        *global_context_guard = Some(Self::new());
        Ok(())
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

    // THREAD SAFETY: Satisfies requirement to lock on output handling
    let global_context = GLOBAL_LOG_CONTEXT.clone();
    let _unused = global_context.lock().unwrap();

    // SAFETY: This call is safe if the call to rcutils_log is safe
    //         We simply forward the parameters and apply a mutex
    //         TODO?: Find a way to verify instead of assuming here
    rcl_logging_multiple_output_handler(location, severity, name, timestamp, format, args);
}

// NOTE(Cyberunner23): Logging macro implementation will use this
// rcutils_log


