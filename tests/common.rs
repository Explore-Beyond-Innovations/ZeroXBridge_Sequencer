use std::sync::Once;
use tracing_subscriber::{self, EnvFilter};

static INIT: Once = Once::new();

pub fn init_tracing() {
    INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_test_writer()
            .with_env_filter(EnvFilter::from_default_env())
            .init();
    });
}

