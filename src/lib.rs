use addr::DomainName;
use error::Result;
use futures::future::BoxFuture;
use futures::future::FutureExt;
use reqwest::Client;
use sources::*;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tracing::{error, info};

pub mod error;
pub mod sources;

const QUEUE_SIZE: usize = 100_000;
const CHAN_SIZE: usize = 255;

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

    /// Collects data from the sources which don't require an API key.
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

    /// Takes a collection of hosts and spawns a new task to collect data from the free or paid
    /// sources for each host.
    pub async fn run<T>(self, hosts: T) -> Result<Vec<String>>
    where
        T: IntoIterator<Item = String>,
    {
        use futures::stream::StreamExt;

        let (tx, mut rx) = mpsc::channel::<Vec<String>>(CHAN_SIZE);
        let subdomains = Arc::new(Mutex::new(Vec::new()));
        let runner = Arc::new(self);

        // Consumer thread which pushes results into a queue and writes them to the output vec
        // periodically
        let subs = Arc::clone(&subdomains);
        let consumer = tokio::spawn(async move {
            info!("spawning consumer thread");
            let mut queue = Vec::with_capacity(QUEUE_SIZE);

            while let Some(v) = rx.recv().await {
                // if the buffer reaches capacity then empty it into the subdomains vec.
                if queue.len() == QUEUE_SIZE {
                    info!("queue reached capacity writing to results vec");
                    let mut lock = subs.lock().await;
                    lock.extend(queue.drain(..));
                };

                info!("pushing {} items into the queue", &v.len());
                queue.extend(v.into_iter());
            }

            // if anything is remaning in the queue push it into the results vec
            if !queue.is_empty() {
                info!("draining the last {} items out of the queue", queue.len());
                let mut lock = subs.lock().await;
                lock.extend(queue.drain(..));
            }
        });

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
        consumer.await?;
        let res = Arc::try_unwrap(subdomains).unwrap();
        Ok(res.into_inner())
    }
}

/// Represents the filtering applied to the output
enum Filter {
    /// Return any result that matches the same subdomain
    SubOnly,
    /// Return any result that has the same root domain
    RootOnly,
}

/// `PostProcessor` is responsible for filtering the raw data from each of the data sources into
/// only those results which are relevant.
pub struct PostProcessor {
    roots: HashSet<String>,
    filter: Filter,
}

impl PostProcessor {
    pub fn new() -> Self {
        Self {
            roots: HashSet::new(),
            filter: Filter::RootOnly,
        }
    }

    /// Sets the `PostProcessor` to return any result which matches the same root domain
    pub fn any_root(&mut self, hosts: &HashSet<String>) -> &mut Self {
        self.roots = hosts
            .iter()
            .filter_map(|d| d.parse::<DomainName>().ok())
            .map(|d| d.root().to_string())
            .collect();
        self.filter = Filter::RootOnly;
        self
    }

    /// Sets the `PostProcessor` to return any result which matches the same subdomain
    pub fn any_subdomain(&mut self, hosts: &HashSet<String>) -> &mut Self {
        self.roots = hosts.clone();
        self.filter = Filter::SubOnly;
        self
    }

    /// Strips invalid characters from the domain, used before attempting to parse a domain into a
    /// `DomainName`. If we didn't strip these characters any attempt to parse a domain into
    /// `DomainName` would return `InvalidDomain` error.
    fn strip_invalid(domain: &str) -> String {
        let blacklisted = vec!["\"", "\\", "*"];
        // iter over the blacklisted chars and return a string that has been cleaned.
        blacklisted.iter().fold(domain.to_string(), |mut res, c| {
            res = res.replace(c, "");
            res.strip_prefix('.').unwrap_or(&res).to_lowercase()
        })
    }

    /// Checks if a domain belongs to any of the root domains provided in the input
    fn is_relevant(&self, domain: &str) -> bool {
        match self.filter {
            Filter::RootOnly => {
                if let Ok(d) = domain.parse::<DomainName>() {
                    self.roots.contains(d.root().to_str())
                } else {
                    false
                }
            }
            Filter::SubOnly => self.roots.iter().any(|root| domain.ends_with(root)),
        }
    }

    /// Takes the results from the `Runner.run` and filters them for relevant subdomains. Relevant is
    /// any result which has a root domain that was present in the input file. In other words, if you
    /// passed in `hackerone.com` as the input it will only return subdomains that belong to that root
    /// domain e.g. `docs.hackerone.com`
    pub fn clean(&mut self, results: Vec<String>) -> Result<()> {
        let filtered: HashSet<String> = results
            .iter()
            .flat_map(|a| a.split_whitespace())
            .map(Self::strip_invalid)
            .filter(|d| self.is_relevant(d))
            .collect();

        filtered.iter().for_each(|r| println!("{}", r));

        Ok(())
    }
}

trait IntoSubdomain {
    fn subdomains(&self) -> Vec<String>;
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
