#[macro_use]
extern crate lazy_static;
use async_std::task;
pub mod sources;
use futures::future::{join_all, BoxFuture};
use sources::{
    anubisdb, bufferover, certspotter, crtsh, facebook, hackertarget, spyse, threatcrowd,
    threatminer, urlscan, virustotal, wayback,
};
use std::collections::HashSet;
use std::pin::Pin;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

// use futures::future::LocalBoxFuture;

pub async fn free_sources(h: &str) -> HashSet<String> {
    //https://stackoverflow.com/questions/58354633/cannot-use-impl-future-to-store-async-function-in-a-vector
    let mut host = h.to_string();
    let mut tasks = Vec::new();
    let v: Vec<BoxFuture<Result<HashSet<String>>>> = vec![
        Box::pin(anubisdb::run(host.to_owned())),
        Box::pin(bufferover::run(host.to_owned())),
        Box::pin(certspotter::run(host.to_owned())),
        Box::pin(hackertarget::run(host)),
    ];

    for f in v {
        tasks.push(task::spawn(async { f.await }));
    }

    let res = join_all(tasks).await;
    res.into_iter()
        .flat_map(|s| s.into_iter())
        .flatten()
        .map(|x| x)
        .collect()
}

pub async fn runner(urls: Vec<String>) -> Vec<HashSet<String>> {
    const ACTIVE_REQUESTS: usize = 100;
    use futures::stream::StreamExt;

    let responses = futures::stream::iter(
        urls.into_iter()
            .map(|url| task::spawn(async move { free_sources(&url.to_string()).await })),
    )
    .buffer_unordered(ACTIVE_REQUESTS)
    .collect::<Vec<HashSet<String>>>();
    responses.await
}
//pub async fn get_data(hosts: Vec<String>) -> Result<()> {
//    const ACTIVE_REQUESTS: usize = 100;
//    let mut subdomains: HashSet<String> = HashSet::new();
//
//    Ok(())
//}
