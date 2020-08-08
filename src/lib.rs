extern crate lazy_static;
extern crate pretty_env_logger;
#[macro_use]
extern crate log;
pub mod error;
pub mod sources;

use error::Result;
use futures::future::BoxFuture;
use reqwest::Client;
use sources::{
    alienvault, anubisdb, binaryedge, c99, certspotter, chaos, crtsh, facebook, hackertarget,
    intelx, passivetotal, sonarsearch, spyse, sublister, threatcrowd, threatminer, urlscan,
    virustotal, wayback,
};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

trait IntoSubdomain {
    fn subdomains(&self) -> HashSet<String>;
}

// Collects data from all sources which don't require an API key
async fn free_sources(host: Arc<String>, client: Client) -> HashSet<String> {
    let mut tasks = Vec::new();
    let mut results = HashSet::new();
    let sources: Vec<BoxFuture<Result<HashSet<String>>>> = vec![
        Box::pin(anubisdb::run(client.clone(), host.clone())),
        Box::pin(alienvault::run(client.clone(), host.clone())),
        Box::pin(certspotter::run(client.clone(), host.clone())),
        Box::pin(crtsh::run(client.clone(), host.clone())),
        Box::pin(threatcrowd::run(client.clone(), host.clone())),
        Box::pin(urlscan::run(client.clone(), host.clone())),
        Box::pin(virustotal::run(client.clone(), host.clone())),
        Box::pin(threatminer::run(client.clone(), host.clone())),
        Box::pin(sublister::run(client.clone(), host.clone())),
        Box::pin(wayback::run(client.clone(), host.clone())),
        Box::pin(sonarsearch::run(host.clone())),
        Box::pin(hackertarget::run(client.clone(), host.clone())),
    ];

    for s in sources {
        tasks.push(tokio::task::spawn(async { s.await }));
    }

    for t in tasks {
        t.await
            .iter()
            .flatten()
            .flatten()
            .map(|s| results.insert(s.into()))
            .for_each(drop);
    }

    results
}

// Collects data from all sources
async fn all_sources(host: Arc<String>, client: Client) -> HashSet<String> {
    let mut tasks = Vec::new();
    let mut results = HashSet::new();
    let sources: Vec<BoxFuture<Result<HashSet<String>>>> = vec![
        Box::pin(anubisdb::run(client.clone(), host.clone())),
        Box::pin(binaryedge::run(client.clone(), host.clone())),
        Box::pin(alienvault::run(client.clone(), host.clone())),
        Box::pin(certspotter::run(client.clone(), host.clone())),
        Box::pin(crtsh::run(client.clone(), host.clone())),
        Box::pin(threatcrowd::run(client.clone(), host.clone())),
        Box::pin(urlscan::run(client.clone(), host.clone())),
        Box::pin(virustotal::run(client.clone(), host.clone())),
        Box::pin(threatminer::run(client.clone(), host.clone())),
        Box::pin(sublister::run(client.clone(), host.clone())),
        Box::pin(wayback::run(client.clone(), host.clone())),
        Box::pin(facebook::run(client.clone(), host.clone())),
        Box::pin(spyse::run(client.clone(), host.clone())),
        Box::pin(c99::run(client.clone(), host.clone())),
        Box::pin(intelx::run(client.clone(), host.clone())),
        Box::pin(passivetotal::run(client.clone(), host.clone())),
        Box::pin(hackertarget::run(client.clone(), host.clone())),
        Box::pin(sonarsearch::run(host.clone())),
        Box::pin(chaos::run(client.clone(), host.clone())),
    ];

    for s in sources {
        tasks.push(tokio::task::spawn(async { s.await }));
    }

    for t in tasks {
        t.await
            .iter()
            .flatten()
            .flatten()
            .map(|s| results.insert(s.into()))
            .for_each(drop);
    }

    results
}

// Takes a bunch of hosts and collects data on them
pub async fn runner(hosts: Vec<String>, all: bool, max_concurrent: usize) -> Vec<String> {
    use futures::stream::StreamExt;

    let client = client!();
    let responses = futures::stream::iter(hosts.into_iter().map(|host| {
        let host = Arc::new(host);
        let client = client.clone();
        tokio::task::spawn(async move {
            if all {
                all_sources(host, client).await
            } else {
                free_sources(host, client).await
            }
        })
    }))
    .buffer_unordered(max_concurrent)
    .collect::<Vec<_>>();

    // this might need to be converted to hashset string
    responses.await.into_iter().flatten().flatten().collect()
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
            .pool_idle_timeout(Duration::from_secs(4))
            .build()
            .unwrap()
    };
}
