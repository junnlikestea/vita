extern crate lazy_static;
extern crate pretty_env_logger;
#[macro_use]
extern crate log;
use error::Result;
use futures::future::BoxFuture;
use futures::future::FutureExt;
use reqwest::Client;
use sources::{
    alienvault, anubisdb, binaryedge, c99, certspotter, chaos, crtsh, facebook, hackertarget,
    intelx, passivetotal, sonarsearch, spyse, sublister, threatcrowd, threatminer, urlscan,
    virustotal, wayback,
};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

pub mod error;
pub mod sources;

trait IntoSubdomain {
    fn subdomains(&self) -> Vec<String>;
}

// Configuration options for the `Runner`
struct Config {
    // Use paid and free sources
    include_all: bool,
    // The maximum number of conurrent tasks
    concurrency: usize,
}

// The `Runner` is responsible for collecting data from all the sources.
pub struct Runner {
    config: Config,
    client: Client,
}

impl Runner {
    pub fn new(include_all: bool, concurrency: usize, timeout: u64) -> Self {
        let config = Config {
            include_all,
            concurrency,
        };

        Self {
            config,
            client: client!(timeout, timeout),
        }
    }

    // Collects data from the sources which don't require an API key.
    async fn free(&self, host: Arc<String>, mut sender: mpsc::Sender<Vec<String>>) -> Result<()> {
        let (tx, mut rx) = mpsc::channel(self.config.concurrency);

        // TODO: Is there a better way to do this?
        let sources: Vec<BoxFuture<Result<()>>> = vec![
            anubisdb::run(self.client.clone(), host.clone(), tx.clone()).boxed(),
            alienvault::run(self.client.clone(), host.clone(), tx.clone()).boxed(),
            certspotter::run(self.client.clone(), host.clone(), tx.clone()).boxed(),
            crtsh::run(self.client.clone(), host.clone(), tx.clone()).boxed(),
            threatcrowd::run(self.client.clone(), host.clone(), tx.clone()).boxed(),
            urlscan::run(self.client.clone(), host.clone(), tx.clone()).boxed(),
            virustotal::run(self.client.clone(), host.clone(), tx.clone()).boxed(),
            threatminer::run(self.client.clone(), host.clone(), tx.clone()).boxed(),
            sublister::run(self.client.clone(), host.clone(), tx.clone()).boxed(),
            wayback::run(self.client.clone(), host.clone(), tx.clone()).boxed(),
            hackertarget::run(self.client.clone(), host.clone(), tx.clone()).boxed(),
            sonarsearch::run(host.clone(), tx).boxed(),
        ];

        let producer = tokio::spawn(async move {
            for s in sources {
                tokio::spawn(async move { s.await });
            }
        });

        let consumer = tokio::spawn(async move {
            while let Some(v) = rx.recv().await {
                if let Err(e) = sender.send(v).await {
                    error!("got error {} when sending to channel", e)
                }
            }
        });

        producer.await?;
        consumer.await?;
        Ok(())
    }

    // Collects data from paid and free sources.
    async fn all(
        self: Arc<Self>,
        host: Arc<String>,
        mut sender: mpsc::Sender<Vec<String>>,
    ) -> Result<()> {
        let (tx, mut rx) = mpsc::channel(self.config.concurrency);
        let sources: Vec<BoxFuture<Result<()>>> = vec![
            anubisdb::run(self.client.clone(), host.clone(), tx.clone()).boxed(),
            binaryedge::run(self.client.clone(), host.clone(), tx.clone()).boxed(),
            alienvault::run(self.client.clone(), host.clone(), tx.clone()).boxed(),
            certspotter::run(self.client.clone(), host.clone(), tx.clone()).boxed(),
            threatcrowd::run(self.client.clone(), host.clone(), tx.clone()).boxed(),
            virustotal::run(self.client.clone(), host.clone(), tx.clone()).boxed(),
            threatminer::run(self.client.clone(), host.clone(), tx.clone()).boxed(),
            sublister::run(self.client.clone(), host.clone(), tx.clone()).boxed(),
            passivetotal::run(self.client.clone(), host.clone(), tx.clone()).boxed(),
            hackertarget::run(self.client.clone(), host.clone(), tx.clone()).boxed(),
            urlscan::run(self.client.clone(), host.clone(), tx.clone()).boxed(),
            crtsh::run(self.client.clone(), host.clone(), tx.clone()).boxed(),
            wayback::run(self.client.clone(), host.clone(), tx.clone()).boxed(),
            facebook::run(self.client.clone(), host.clone(), tx.clone()).boxed(),
            spyse::run(self.client.clone(), host.clone(), tx.clone()).boxed(),
            c99::run(self.client.clone(), host.clone(), tx.clone()).boxed(),
            intelx::run(self.client.clone(), host.clone(), tx.clone()).boxed(),
            sonarsearch::run(host.clone(), tx.clone()).boxed(),
            chaos::run(self.client.clone(), host.clone(), tx).boxed(),
        ];

        let producer = tokio::spawn(async move {
            for s in sources {
                tokio::spawn(async { s.await });
            }
        });

        let consumer = tokio::spawn(async move {
            while let Some(v) = rx.recv().await {
                if let Err(e) = sender.send(v).await {
                    error!("got error {} when sending to channel", e)
                }
            }
        });

        producer.await?;
        consumer.await?;
        Ok(())
    }

    // Takes a collection of hosts and spawns a new task to collect data from the free or paid
    // sources for each host.
    pub async fn run(self, hosts: Vec<String>) -> Result<HashSet<String>> {
        use futures::stream::StreamExt;

        let (tx, mut rx) = mpsc::channel(124);
        let mut subdomains = HashSet::new();
        let runner = Arc::new(self);

        let producer = futures::stream::iter(hosts)
            .map(|host| {
                let h = Arc::new(host);
                let tx = tx.clone();
                let runner = Arc::clone(&runner);
                if runner.config.include_all {
                    tokio::spawn(async move {
                        info!("spawning task for {}", &h);
                        runner.all(h, tx).await
                    })
                } else {
                    tokio::spawn(async move {
                        info!("spawning task for {}", &h);
                        runner.free(h, tx).await
                    })
                }
            })
            .buffer_unordered(runner.config.concurrency)
            .collect::<Vec<_>>();
        // explicitly drop the remaning sender
        producer.await;
        drop(tx);

        while let Some(v) = rx.recv().await {
            v.into_iter().map(|s| subdomains.insert(s)).for_each(drop);
        }

        Ok(subdomains)
    }
}

#[macro_export]
//https://stackoverflow.com/questions/24047686/default-function-arguments-in-rust
macro_rules! client {
    ($timeout:expr, $ptimeout:expr) => {
        reqwest::ClientBuilder::new()
            .timeout(Duration::from_secs($timeout))
            .pool_idle_timeout(Duration::from_secs($ptimeout))
            .build()
            .unwrap()
    };
    () => {
        reqwest::ClientBuilder::new()
            .timeout(Duration::from_secs(10))
            .pool_idle_timeout(Duration::from_secs(20))
            .build()
            .unwrap()
    };
}
