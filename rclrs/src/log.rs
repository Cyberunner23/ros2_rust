use std::sync::{Arc, Mutex};

use lazy_static::lazy_static;

use crate::RclrsError;
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

        todo!();

        // Marks rcl logging as uninitialized
        *global_context_guard = None;
    }
}

impl LogContext {
    pub(crate) fn new() -> Self {
        Self {}
    }

    pub(crate) fn init() -> Result<(), RclrsError> {

        // THREAD SAFETY: Satisfies requirement to lock on initialize
        let global_context = GLOBAL_LOG_CONTEXT.clone();
        let mut global_context_guard = global_context.lock().unwrap();
        
        todo!();

        *global_context_guard = Some(Self::new());
        Ok(())
    }
}
