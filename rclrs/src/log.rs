use std::sync::{Arc, Mutex};

use lazy_static::lazy_static;

use crate::RclrsError;

lazy_static! {
    // rcl itself hold a NON-THREAD-SAFE global logging context.
    // Therefore, it is our job to ensure thread safety when calling rcl logging functions.
    // Concretely, we must ensure thread safety when:
    //     * Initializing rcl logging.
    //     * Uninitializing rcl logging.
    //     * Sending logs the output handlers (calls to rcl_logging_multiple_output_handler).
    // It is also our job to ensure rcl logging cannot be initialized if it is already initialized.
    // Option signifies whether logging is initialized or not.
    pub(crate) static ref GLOBAL_LOG_CONTEXT: Arc<Mutex<Option<LogContext>>> = Arc::new(Mutex::new(None));
}

// 
struct LogContext;

impl Drop for LogContext {
    fn drop(&mut self) {
        todo!();

        // Marks logging as uninitialized
        let global_context = GLOBAL_LOG_CONTEXT.clone();
        let mut global_context_guard = global_context.lock().unwrap();
        *global_context_guard = None;
    }
}

impl LogContext {
    pub(crate) fn new() -> Self {
        Self {}
    }

    pub(crate) fn init() -> Result<(), RclrsError> {
        
        
        Ok(())
    }
}
