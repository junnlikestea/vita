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
use tokio::sync::mpsc;

trait IntoSubdomain {
    fn subdomains(&self) -> Vec<String>;
}

// Collects data from all sources which don't require an API key
async fn free_sources(
    host: Arc<String>,
    client: Client,
    mut sender: mpsc::Sender<Vec<String>>,
    max_concurrent: usize,
) -> Result<()> {
    let (tx, mut rx) = mpsc::channel(max_concurrent);
    let sources: Vec<BoxFuture<Result<()>>> = vec![
        Box::pin(anubisdb::run(client.clone(), host.clone(), tx.clone())),
        Box::pin(alienvault::run(client.clone(), host.clone(), tx.clone())),
        Box::pin(certspotter::run(client.clone(), host.clone(), tx.clone())),
        Box::pin(crtsh::run(client.clone(), host.clone(), tx.clone())),
        Box::pin(threatcrowd::run(client.clone(), host.clone(), tx.clone())),
        Box::pin(urlscan::run(client.clone(), host.clone(), tx.clone())),
        Box::pin(virustotal::run(client.clone(), host.clone(), tx.clone())),
        Box::pin(threatminer::run(client.clone(), host.clone(), tx.clone())),
        Box::pin(sublister::run(client.clone(), host.clone(), tx.clone())),
        Box::pin(wayback::run(client.clone(), host.clone(), tx.clone())),
        Box::pin(sonarsearch::run(host.clone(), tx.clone())),
        Box::pin(hackertarget::run(client.clone(), host.clone(), tx)),
    ];

    let t1 = tokio::spawn(async move {
        for s in sources {
            tokio::spawn(async move { s.await });
        }
    });

    let t2 = tokio::spawn(async move {
        while let Some(v) = rx.recv().await {
            if let Err(e) = sender.send(v).await {
                error!("got error {} when sending to channel", e)
            }
        }
    });

    t1.await?;
    t2.await?;
    Ok(())
}

// Collects data from all sources
async fn all_sources(
    host: Arc<String>,
    client: Client,
    mut sender: mpsc::Sender<Vec<String>>,
    max_concurrent: usize,
) -> Result<()> {
    let (tx, mut rx) = mpsc::channel(max_concurrent);
    let sources: Vec<BoxFuture<Result<()>>> = vec![
        Box::pin(anubisdb::run(client.clone(), host.clone(), tx.clone())),
        Box::pin(binaryedge::run(client.clone(), host.clone(), tx.clone())),
        Box::pin(alienvault::run(client.clone(), host.clone(), tx.clone())),
        Box::pin(certspotter::run(client.clone(), host.clone(), tx.clone())),
        Box::pin(crtsh::run(client.clone(), host.clone(), tx.clone())),
        Box::pin(threatcrowd::run(client.clone(), host.clone(), tx.clone())),
        Box::pin(urlscan::run(client.clone(), host.clone(), tx.clone())),
        Box::pin(virustotal::run(client.clone(), host.clone(), tx.clone())),
        Box::pin(threatminer::run(client.clone(), host.clone(), tx.clone())),
        Box::pin(sublister::run(client.clone(), host.clone(), tx.clone())),
        Box::pin(wayback::run(client.clone(), host.clone(), tx.clone())),
        Box::pin(facebook::run(client.clone(), host.clone(), tx.clone())),
        Box::pin(spyse::run(client.clone(), host.clone(), tx.clone())),
        Box::pin(c99::run(client.clone(), host.clone(), tx.clone())),
        Box::pin(intelx::run(client.clone(), host.clone(), tx.clone())),
        Box::pin(passivetotal::run(client.clone(), host.clone(), tx.clone())),
        Box::pin(hackertarget::run(client.clone(), host.clone(), tx.clone())),
        Box::pin(sonarsearch::run(host.clone(), tx.clone())),
        Box::pin(chaos::run(client.clone(), host.clone(), tx)),
    ];

    let t1 = tokio::spawn(async move {
        for s in sources {
            tokio::spawn(async { s.await });
        }
    });

    let t2 = tokio::spawn(async move {
        while let Some(v) = rx.recv().await {
            if let Err(e) = sender.send(v).await {
                error!("got error {} when sending to channel", e)
            }
        }
    });

    t1.await?;
    t2.await?;
    Ok(())
}

// Takes a bunch of hosts and collects data on them
pub async fn runner(
    hosts: Vec<String>,
    all: bool,
    max_concurrent: usize,
) -> Result<HashSet<String>> {
    // this isn't really a concurrency threshold, but more of a buffer on how many senders we can
    // have at one time,so kinda is also.
    let (tx, mut rx) = mpsc::channel(max_concurrent);
    let client = client!();
    let mut subdomains = HashSet::new();

    for host in hosts.into_iter() {
        let h = Arc::new(host);
        let client = client.clone();
        let tx = tx.clone();
        if all {
            tokio::spawn(async move { all_sources(h, client, tx, max_concurrent).await });
        } else {
            tokio::spawn(async move { free_sources(h, client, tx, max_concurrent).await });
        }
    }
    // explicitly drop the remaning sender
    drop(tx);

    while let Some(v) = rx.recv().await {
        v.into_iter().map(|s| subdomains.insert(s)).for_each(drop);
    }

    Ok(subdomains)
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
