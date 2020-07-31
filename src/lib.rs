extern crate lazy_static;
pub mod error;
pub mod sources;

use error::Result;
use futures::future::BoxFuture;
use sources::{
    alienvault, anubisdb, binaryedge, bufferover, c99, certspotter, chaos, crtsh, facebook,
    hackertarget, intelx, passivetotal, sonarsearch, spyse, sublister, threatcrowd, threatminer,
    urlscan, virustotal, wayback,
};
use std::collections::HashSet;
use std::sync::Arc;

trait IntoSubdomain {
    fn subdomains(&self) -> HashSet<String>;
}

// Collects data from all sources which don't require an API key
async fn free_sources(host: Arc<String>) -> HashSet<String> {
    let mut tasks = Vec::new();
    let mut results = HashSet::new();
    let sources: Vec<BoxFuture<Result<HashSet<String>>>> = vec![
        Box::pin(anubisdb::run(host.clone())),
        Box::pin(alienvault::run(host.clone())),
        Box::pin(bufferover::run(host.clone(), true)),
        Box::pin(bufferover::run(host.clone(), false)),
        Box::pin(certspotter::run(host.clone())),
        Box::pin(crtsh::run(host.clone())),
        Box::pin(threatcrowd::run(host.clone())),
        Box::pin(urlscan::run(host.clone())),
        Box::pin(virustotal::run(host.clone())),
        Box::pin(threatminer::run(host.clone())),
        Box::pin(sublister::run(host.clone())),
        Box::pin(wayback::run(host.clone())),
        Box::pin(sonarsearch::run(host.clone())),
        Box::pin(hackertarget::run(host.clone())),
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
async fn all_sources(host: Arc<String>) -> HashSet<String> {
    let mut tasks = Vec::new();
    let mut results = HashSet::new();
    let sources: Vec<BoxFuture<Result<HashSet<String>>>> = vec![
        Box::pin(anubisdb::run(host.clone())),
        Box::pin(binaryedge::run(host.clone())),
        Box::pin(alienvault::run(host.clone())),
        Box::pin(bufferover::run(host.clone(), true)),
        Box::pin(bufferover::run(host.clone(), false)),
        Box::pin(certspotter::run(host.clone())),
        Box::pin(crtsh::run(host.clone())),
        Box::pin(threatcrowd::run(host.clone())),
        Box::pin(urlscan::run(host.clone())),
        Box::pin(virustotal::run(host.clone())),
        Box::pin(threatminer::run(host.clone())),
        Box::pin(sublister::run(host.clone())),
        Box::pin(wayback::run(host.clone())),
        Box::pin(facebook::run(host.clone())),
        Box::pin(spyse::run(host.clone())),
        Box::pin(c99::run(host.clone())),
        Box::pin(intelx::run(host.clone())),
        Box::pin(passivetotal::run(host.clone())),
        Box::pin(hackertarget::run(host.clone())),
        Box::pin(sonarsearch::run(host.clone())),
        Box::pin(chaos::run(host.clone())),
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

    let responses = futures::stream::iter(hosts.into_iter().map(|host| {
        let host = Arc::new(host);
        tokio::task::spawn(async move {
            if all {
                all_sources(host).await
            } else {
                free_sources(host).await
            }
        })
    }))
    .buffer_unordered(max_concurrent)
    .collect::<Vec<_>>();

    // this might need to be converted to hashset string
    responses.await.into_iter().flatten().flatten().collect()
}
