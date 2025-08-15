// Observability: metrics, logging, and monitoring

pub mod logging;
pub mod metrics;
pub mod metrics_push;

// Re-export main functions for ease of use
pub use logging::init_logging;
pub use metrics::{
    heartbeat, init,
};

// Re-export metric recording functions organized by phase
pub mod sources {
    
}

pub mod gateway {
    
}

pub mod parser {
    
}

pub mod ingest_log {
    
}
