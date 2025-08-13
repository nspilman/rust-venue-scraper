pub mod domain {
    // Core business types and rules. Keep this pure and free of I/O concerns.
}

pub mod application {
    // Use-cases and ports (traits) that the domain needs to be executed.
    // Define traits for external concerns the use-cases depend on.
    use std::sync::Arc;

    #[allow(dead_code)]
    #[async_trait::async_trait]
    pub trait StoragePort: Send + Sync {
        async fn save_raw(&self, key: &str, bytes: Vec<u8>) -> Result<(), String>;
        async fn load_raw(&self, key: &str) -> Result<Option<Vec<u8>>, String>;
    }

    #[allow(dead_code)]
    #[async_trait::async_trait]
    pub trait HttpClientPort: Send + Sync {
        async fn get(&self, url: &str) -> Result<Vec<u8>, String>;
    }

    #[allow(dead_code)]
    pub trait ClockPort: Send + Sync {
        fn now_utc(&self) -> std::time::SystemTime;
    }

    #[allow(dead_code)]
    pub struct UseCases<P: StoragePort + ?Sized> {
        pub storage: Arc<P>,
    }
}

pub mod infrastructure {
    // Adapters that will implement application ports using concrete tech (reqwest, fs, db, etc.)
}

pub mod interface {
    // HTTP/CLI adapters that translate requests to application use-cases.
}

