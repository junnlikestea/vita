pub mod sources;
use async_std::task;
use futures::future::{join_all, BoxFuture};
use sources::{
    anubisdb, bufferover, certspotter, crtsh, facebook, hackertarget, spyse, sublister,
    threatcrowd, threatminer, urlscan, virustotal, wayback,
};
use std::collections::HashSet;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

// Collects data from all sources which don't require and API key
async fn free_sources(host: String) -> HashSet<String> {
    let mut tasks = Vec::new();
    let v: Vec<BoxFuture<Result<HashSet<String>>>> = vec![
        Box::pin(anubisdb::run(host.to_owned())),
        Box::pin(bufferover::run(host.to_owned())),
        Box::pin(certspotter::run(host.to_owned())),
        Box::pin(crtsh::run(host.to_owned())),
        Box::pin(threatcrowd::run(host.to_owned())),
        Box::pin(urlscan::run(host.to_owned())),
        Box::pin(virustotal::run(host.to_owned())),
        Box::pin(threatminer::run(host.to_owned())),
        Box::pin(sublister::run(host.to_owned())),
        Box::pin(wayback::run(host.to_owned())),
        Box::pin(hackertarget::run(host)),
    ];

    for f in v {
        tasks.push(task::spawn(async { f.await }));
    }

    let res = join_all(tasks).await;
    res.into_iter().flatten().flatten().collect()
}

// Collects data from all sources
async fn all_sources(host: String) -> HashSet<String> {
    let mut tasks = Vec::new();
    let v: Vec<BoxFuture<Result<HashSet<String>>>> = vec![
        Box::pin(anubisdb::run(host.to_owned())),
        Box::pin(bufferover::run(host.to_owned())),
        Box::pin(certspotter::run(host.to_owned())),
        Box::pin(crtsh::run(host.to_owned())),
        Box::pin(threatcrowd::run(host.to_owned())),
        Box::pin(urlscan::run(host.to_owned())),
        Box::pin(virustotal::run(host.to_owned())),
        Box::pin(threatminer::run(host.to_owned())),
        Box::pin(sublister::run(host.to_owned())),
        Box::pin(wayback::run(host.to_owned())),
        Box::pin(facebook::run(host.to_owned())),
        Box::pin(spyse::run(host.to_owned())),
        Box::pin(hackertarget::run(host)),
    ];

    for f in v {
        tasks.push(task::spawn(async { f.await }));
    }

    let res = join_all(tasks).await;
    res.into_iter().flatten().flatten().collect()
}

// Takes a bunch of hosts and collects data on them
pub async fn runner(hosts: Vec<String>, all: bool) -> Vec<String> {
    const ACTIVE_REQUESTS: usize = 40;
    use futures::stream::StreamExt;

    let responses = futures::stream::iter(hosts.into_iter().map(|host| {
        task::spawn(async move {
            if all {
                all_sources(host).await
            } else {
                free_sources(host).await
            }
        })
    }))
    .buffer_unordered(ACTIVE_REQUESTS)
    .collect::<Vec<HashSet<String>>>();

    responses.await.into_iter().flatten().collect()
}
