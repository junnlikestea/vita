use crate::sources::{
    alienvault::AlienVault, anubisdb::AnubisDB, binaryedge::BinaryEdge, c99::C99,
    certspotter::CertSpotter, chaos::Chaos, crtsh::Crtsh, facebook::Facebook,
    hackertarget::HackerTarget, intelx::Intelx, passivetotal::PassiveTotal,
    securitytrails::SecurityTrails, sonarsearch::SonarSearch, spyse::Spyse, sublister::Sublister,
    threatcrowd::ThreatCrowd, threatminer::ThreatMiner, urlscan::UrlScan, virustotal::VirusTotal,
    wayback::Wayback,
};
use crate::{client, error::Result, DataSource};

use futures::stream::{FuturesUnordered, StreamExt};
use futures_core::stream::Stream;
use reqwest::Client;
use std::collections::HashMap;
use std::collections::HashSet;
use std::str::FromStr;
use std::sync::Arc;
use strum_macros::EnumString;
use tokio::sync::mpsc;
use tracing::{info, warn};

const CHAN_SIZE: usize = 255;

#[derive(Debug, Eq, PartialEq, Hash, EnumString)]
enum Source {
    AlienVault,
    AnubisDB,
    BinaryEdge,
    C99,
    CertSpotter,
    Chaos,
    Crtsh,
    Facebook,
    HackerTarget,
    Intelx,
    PassiveTotal,
    SecurityTrails,
    SonarSearch,
    Spyse,
    Sublister,
    ThreatCrowd,
    ThreatMiner,
    UrlScan,
    VirusTotal,
    Wayback,
}

// Configuration options for the `Runner`
struct Config {
    timeout: u64,
    // The maximum number of conurrent tasks
    concurrency: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            timeout: 15,
            concurrency: 200,
        }
    }
}

// The `Runner` is responsible for collecting data from all the sources.
pub struct Runner {
    client: Client,
    sources: HashMap<Source, Arc<dyn DataSource>>,
    config: Config,
}

impl Default for Runner {
    fn default() -> Self {
        let config = Config::default();
        Self {
            client: client!(config.timeout, config.timeout),
            sources: HashMap::new(),
            config,
        }
    }
}

impl Runner {
    /// Sets the limit of concurrent tasks
    pub fn concurrency(mut self, limit: usize) -> Self {
        self.config.concurrency = limit;
        self
    }

    /// Sets the request timeout
    pub fn timeout(mut self, duration: u64) -> Self {
        self.config.timeout = duration;
        self
    }

    /// Excludes a collection sources from data collection
    pub fn exclude(mut self, excluded: &[&str]) -> Self {
        if !excluded.is_empty() {
            excluded.iter().for_each(|s| {
                if let Ok(source) = Source::from_str(s) {
                    info!("excluding {:?}", source);
                    self.sources.remove(&source);
                };
            });
        }

        self
    }

    /// Sets the sources to be all those which do not require an api key to use.
    pub fn free_sources(mut self) -> Self {
        // Client uses Arc internally
        let free: Vec<(Source, Arc<dyn DataSource>)> = vec![
            (
                Source::AnubisDB,
                Arc::new(AnubisDB::new(self.client.clone())),
            ),
            (
                Source::AlienVault,
                Arc::new(AlienVault::new(self.client.clone())),
            ),
            (
                Source::CertSpotter,
                Arc::new(CertSpotter::new(self.client.clone())),
            ),
            (
                Source::ThreatCrowd,
                Arc::new(ThreatCrowd::new(self.client.clone())),
            ),
            (
                Source::VirusTotal,
                Arc::new(VirusTotal::new(self.client.clone())),
            ),
            (
                Source::ThreatMiner,
                Arc::new(ThreatMiner::new(self.client.clone())),
            ),
            (
                Source::Sublister,
                Arc::new(Sublister::new(self.client.clone())),
            ),
            (
                Source::HackerTarget,
                Arc::new(HackerTarget::new(self.client.clone())),
            ),
            (
                Source::SonarSearch,
                Arc::new(SonarSearch::new(self.client.clone())),
            ),
            (Source::Wayback, Arc::new(Wayback::new(self.client.clone()))),
            (Source::UrlScan, Arc::new(UrlScan::new(self.client.clone()))),
            (Source::Crtsh, Arc::new(Crtsh::new(self.client.clone()))),
        ];

        self.sources.extend(free.into_iter());
        self
    }

    /// Sets the sources to include api keys in addition to the free sources.
    pub fn all_sources(mut self) -> Self {
        let all: Vec<(Source, Arc<dyn DataSource>)> = vec![
            (
                Source::AnubisDB,
                Arc::new(AnubisDB::new(self.client.clone())),
            ),
            (
                Source::AlienVault,
                Arc::new(AlienVault::new(self.client.clone())),
            ),
            (
                Source::CertSpotter,
                Arc::new(CertSpotter::new(self.client.clone())),
            ),
            (
                Source::ThreatCrowd,
                Arc::new(ThreatCrowd::new(self.client.clone())),
            ),
            (
                Source::VirusTotal,
                Arc::new(VirusTotal::new(self.client.clone())),
            ),
            (
                Source::ThreatMiner,
                Arc::new(ThreatMiner::new(self.client.clone())),
            ),
            (
                Source::Sublister,
                Arc::new(Sublister::new(self.client.clone())),
            ),
            (
                Source::SecurityTrails,
                Arc::new(SecurityTrails::new(self.client.clone())),
            ),
            (
                Source::HackerTarget,
                Arc::new(HackerTarget::new(self.client.clone())),
            ),
            (
                Source::SonarSearch,
                Arc::new(SonarSearch::new(self.client.clone())),
            ),
            (
                Source::BinaryEdge,
                Arc::new(BinaryEdge::new(self.client.clone())),
            ),
            (
                Source::PassiveTotal,
                Arc::new(PassiveTotal::new(self.client.clone())),
            ),
            (
                Source::Facebook,
                Arc::new(Facebook::new(self.client.clone())),
            ),
            (Source::Spyse, Arc::new(Spyse::new(self.client.clone()))),
            (Source::C99, Arc::new(C99::new(self.client.clone()))),
            (Source::Intelx, Arc::new(Intelx::new(self.client.clone()))),
            (Source::Wayback, Arc::new(Wayback::new(self.client.clone()))),
            (Source::UrlScan, Arc::new(UrlScan::new(self.client.clone()))),
            (Source::Crtsh, Arc::new(Crtsh::new(self.client.clone()))),
            (Source::Chaos, Arc::new(Chaos::new(self.client.clone()))),
        ];

        self.sources.extend(all.into_iter());
        self
    }

    /// Fetches data from the sources concurrently
    pub async fn run(self, hosts: HashSet<String>) -> Result<impl Stream<Item = Vec<String>>> {
        let (tx, rx) = mpsc::channel::<Vec<String>>(CHAN_SIZE);
        let sources = Arc::new(self.sources);
        let max_concurrent = self.config.concurrency;

        let tx2 = tx.clone();
        tokio::spawn(async move {
            let mut futures = FuturesUnordered::new();
            for host in hosts.into_iter() {
                let host = Arc::new(host);

                if futures.len() >= max_concurrent {
                    futures.next().await;
                }

                for source in sources.values() {
                    let source = Arc::clone(source);
                    let host = Arc::clone(&host);
                    let tx = tx2.clone();
                    futures.push(tokio::spawn(async move { source.run(host, tx).await }));
                }
            }

            // Get the remaining futures
            while let Some(res) = futures.next().await {
                if let Err(e) = res {
                    warn!("got error {} when trying to recv remaining futures", e)
                }
            }
        });

        // explicitly drop the remaning sender
        drop(tx);
        Ok(rx)
    }
}
