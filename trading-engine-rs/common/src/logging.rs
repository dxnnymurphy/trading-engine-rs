use env_logger::Builder;

/// Initialise logger for the application with the standard level INFO unless overridden
/// by environment variable RUST_LOG
pub fn init_logging() {
    Builder::new()
        .filter_level(log::LevelFilter::Info)
        .parse_default_env()
        .init();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;

    static INIT: Once = Once::new();

    // Note: init_logging() can only be called once per test process
    // because env_logger::init() panics if called twice.
    // This is a limitation of env_logger, not our code.

    #[test]
    fn test_init_logging_does_not_panic() {
        // Ensure we only initialize once across all tests
        INIT.call_once(|| {
            init_logging();
        });

        // Test that logging calls work after initialization
        log::info!("Test info message");
        log::debug!("Test debug message");
        log::warn!("Test warning message");

        // If we got here without panicking, the test passes
    }

    #[test]
    fn test_logging_macros_available() {
        // This test verifies that the log macros are available and compile
        // It doesn't verify output because that requires runtime inspection
        
        INIT.call_once(|| {
            init_logging();
        });

        // These should compile and not panic
        log::error!("Error level: {}", 1);
        log::warn!("Warn level: {}", 2);
        log::info!("Info level: {}", 3);
        log::debug!("Debug level: {}", 4);
        log::trace!("Trace level: {}", 5);
    }
}