#![allow(clippy::rc_buffer)]

use async_trait::async_trait;
use error::Result;
pub use postprocessor::{CleanExt, PostProcessor, PostProcessorIter};
use std::sync::Arc;
use tokio::sync::mpsc;
pub use vita::Runner;

pub mod error;
pub mod postprocessor;
pub mod sources;
pub mod vita;

// Arbitrary number for the queue capacity
pub(crate) const QUEUE_SIZE: usize = 1024;

trait IntoSubdomain {
    fn subdomains(&self) -> Vec<String>;
}

#[async_trait]
trait DataSource: Send + Sync {
    async fn run(&self, host: Arc<String>, mut tx: mpsc::Sender<Vec<String>>) -> Result<()>;
}

#[macro_export]
//https://stackoverflow.com/questions/24047686/default-function-arguments-in-rust
macro_rules! client {
    ($timeout:expr, $ptimeout:expr) => {
        reqwest::ClientBuilder::new()
            .timeout(std::time::Duration::from_secs($timeout))
            .pool_idle_timeout(std::time::Duration::from_secs($ptimeout))
            .build()
            .unwrap()
    };
    () => {
        reqwest::ClientBuilder::new()
            .timeout(std::time::Duration::from_secs(20))
            .pool_idle_timeout(std::time::Duration::from_secs(20))
            .build()
            .unwrap()
    };
}
