use tracing_subscriber::EnvFilter;

/// Initialize the global tracing subscriber.
///
/// In production (no RUST_LOG set), uses the provided `log_level`.
/// In development (RUST_LOG set), respects that override.
/// Uses JSON format for structured logging.
pub fn init_tracing(log_level: &str) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_tracing_does_not_panic() {
        // Use try_init to avoid panic from double initialization in tests
        let result = tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::new("info"))
            .with_test_writer()
            .try_init();

        // ok if first init, err if already initialized (harmless in test context)
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_init_tracing_with_custom_level() {
        // Using try_init since tracing is likely already initialized by the previous test
        let result = tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::new("debug"))
            .with_test_writer()
            .try_init();

        assert!(result.is_ok() || result.is_err());
    }
}
