#![allow(clippy::rc_buffer)]

use addr::DomainName;
use async_trait::async_trait;
use error::Result;
use futures::stream::FuturesUnordered;
use futures::stream::StreamExt;
use futures_core::stream::Stream;
use reqwest::Client;
use sources::{
    alienvault::AlienVault, anubisdb::AnubisDB, binaryedge::BinaryEdge, c99::C99,
    certspotter::CertSpotter, chaos::Chaos, crtsh::Crtsh, facebook::Facebook,
    hackertarget::HackerTarget, intelx::Intelx, passivetotal::PassiveTotal,
    securitytrails::SecurityTrails, sonarsearch::SonarSearch, spyse::Spyse, sublister::Sublister,
    threatcrowd::ThreatCrowd, threatminer::ThreatMiner, urlscan::UrlScan, virustotal::VirusTotal,
    wayback::Wayback,
};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{info, warn};

pub mod error;
pub mod sources;

const QUEUE_SIZE: usize = 100_000;
const CHAN_SIZE: usize = 255;

trait IntoSubdomain {
    fn subdomains(&self) -> Vec<String>;
}

#[async_trait]
trait DataSource: Send + Sync {
    async fn run(&self, host: Arc<String>, mut tx: mpsc::Sender<Vec<String>>) -> Result<()>;
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
    sources: Vec<Arc<dyn DataSource>>,
    config: Config,
}

impl Default for Runner {
    fn default() -> Self {
        let config = Config::default();
        Self {
            client: client!(config.timeout, config.timeout),
            sources: Vec::new(),
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

    /// Sets the sources to be all those which do not require an api key to use.
    pub fn free_sources(mut self) -> Self {
        // Client uses Arc internally so we're just cloning pointers
        let free: Vec<Arc<dyn DataSource>> = vec![
            Arc::new(AnubisDB::new(self.client.clone())),
            Arc::new(AlienVault::new(self.client.clone())),
            Arc::new(CertSpotter::new(self.client.clone())),
            Arc::new(Crtsh::new(self.client.clone())),
            Arc::new(ThreatCrowd::new(self.client.clone())),
            Arc::new(UrlScan::new(self.client.clone())),
            Arc::new(VirusTotal::new(self.client.clone())),
            Arc::new(ThreatMiner::new(self.client.clone())),
            Arc::new(Sublister::new(self.client.clone())),
            Arc::new(Wayback::new(self.client.clone())),
            Arc::new(HackerTarget::new(self.client.clone())),
            Arc::new(SonarSearch::new(self.client.clone())),
        ];

        self.sources.extend(free.into_iter());
        self
    }

    /// Sets the sources to include api keys in addition to the free sources.
    pub fn all_sources(mut self) -> Self {
        let all: Vec<Arc<dyn DataSource>> = vec![
            Arc::new(AnubisDB::new(self.client.clone())),
            Arc::new(AlienVault::new(self.client.clone())),
            Arc::new(CertSpotter::new(self.client.clone())),
            Arc::new(Crtsh::new(self.client.clone())),
            Arc::new(ThreatCrowd::new(self.client.clone())),
            Arc::new(UrlScan::new(self.client.clone())),
            Arc::new(VirusTotal::new(self.client.clone())),
            Arc::new(ThreatMiner::new(self.client.clone())),
            Arc::new(Sublister::new(self.client.clone())),
            Arc::new(SecurityTrails::new(self.client.clone())),
            Arc::new(Wayback::new(self.client.clone())),
            Arc::new(HackerTarget::new(self.client.clone())),
            Arc::new(SonarSearch::new(self.client.clone())),
            Arc::new(BinaryEdge::new(self.client.clone())),
            Arc::new(PassiveTotal::new(self.client.clone())),
            Arc::new(Facebook::new(self.client.clone())),
            Arc::new(Spyse::new(self.client.clone())),
            Arc::new(C99::new(self.client.clone())),
            Arc::new(Intelx::new(self.client.clone())),
            Arc::new(Chaos::new(self.client.clone())),
        ];

        self.sources.extend(all.into_iter());
        self
    }

    /// Fetches data from the sources concurrently
    pub async fn run<T>(self, hosts: T) -> Result<impl Stream<Item = Vec<String>>>
    where
        T: IntoIterator<Item = String>,
    {
        let (tx, rx) = mpsc::channel::<Vec<String>>(CHAN_SIZE);
        let sources = Arc::new(self.sources);

        let mut futures = FuturesUnordered::new();
        let mut outs = Vec::new();
        for host in hosts.into_iter() {
            let host = Arc::new(host);

            if futures.len() >= self.config.concurrency {
                outs.push(futures.next().await.unwrap());
            }

            for source in sources.iter() {
                let source = Arc::clone(source);
                let host = Arc::clone(&host);
                let tx = tx.clone();
                futures.push(tokio::spawn(async move { source.run(host, tx).await }));
            }
        }

        // Get the remaining futures
        while let Some(res) = futures.next().await {
            if let Err(e) = res {
                warn!("got error {} when trying to recv remaining futures", e)
            }
        }

        // explicitly drop the remaning sender
        drop(tx);
        Ok(rx)
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

impl Default for PostProcessor {
    fn default() -> Self {
        Self {
            roots: HashSet::new(),
            filter: Filter::RootOnly,
        }
    }
}

impl PostProcessor {
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
    fn strip_invalid<T: Into<String>>(domain: T) -> String {
        let blacklisted = vec!["\"", "\\", "*"];
        // iter over the blacklisted chars and return a string that has been cleaned.
        blacklisted.iter().fold(domain.into(), |mut res, c| {
            res = res.replace(c, "");
            res.strip_prefix('.').unwrap_or(&res).to_lowercase()
        })
    }

    /// Checks if a domain belongs to any of the root domains provided in the input
    fn is_relevant<T: AsRef<str>>(&self, domain: T) -> bool {
        let cleaned_domain = Self::strip_invalid(domain.as_ref());
        match self.filter {
            Filter::RootOnly => {
                if let Ok(d) = cleaned_domain.parse::<DomainName>() {
                    self.roots.contains(d.root().to_str())
                } else {
                    false
                }
            }
            Filter::SubOnly => self
                .roots
                .iter()
                .any(|root| cleaned_domain.ends_with(root) && !cleaned_domain.eq(root)),
        }
    }

    ///// Takes the results from the `Runner.run` and filters them for relevant subdomains.
    /////
    ///// TODO: explain relevant results
    //pub fn clean(&mut self, results: Vec<String>) -> Option<Vec<String>> {
    //    let filtered: Vec<String> = results
    //        .iter()
    //        .flat_map(|a| a.split_whitespace())
    //        .map(Self::strip_invalid)
    //        .filter(|d| self.is_relevant(d))
    //        .collect();

    //    if !filtered.is_empty() {
    //        return Some(filtered);
    //    }

    //    None
    //}
}

pub struct CleanIter<'a, I>
where
    I: Iterator,
{
    cleaner: &'a PostProcessor,
    inner: I,
}

impl<'a, I> Iterator for CleanIter<'a, I>
where
    I: Iterator,
    I::Item: std::hash::Hash + Eq + Clone + AsRef<str>,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(d) = self.inner.next() {
            if self.cleaner.is_relevant(d.as_ref()) {
                return Some(d);
            }
        }
        None
    }
}

pub trait CleanExt: Iterator {
    fn clean(self, postprocessor: &PostProcessor) -> CleanIter<Self>
    where
        Self::Item: std::hash::Hash + Eq + Clone + AsRef<str>,
        Self: Sized,
    {
        CleanIter {
            cleaner: postprocessor,
            inner: self,
        }
    }
}

impl<I: Iterator> CleanExt for I {}

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
            .timeout(Duration::from_secs(20))
            .pool_idle_timeout(Duration::from_secs(20))
            .build()
            .unwrap()
    };
}
