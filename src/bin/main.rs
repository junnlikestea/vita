extern crate vita;
use clap::{App, Arg};
use regex::{RegexSet, RegexSetBuilder};
use std::collections::HashSet;
use std::fs;
use std::io::{self, Read};
use vita::error::Result;

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();
    let args = create_clap_app("v0.1.10");
    let matches = args.get_matches();
    let mut all_sources = false;
    let mut hosts: Vec<String> = Vec::new();
    let max_concurrent: usize = matches.value_of("concurrency").unwrap().parse()?;

    if matches.is_present("all_sources") {
        all_sources = true;
    }

    if matches.is_present("file") {
        let input = matches.value_of("input").unwrap();
        let contents = fs::read_to_string(input)?;
        hosts = contents.lines().map(|l| l.to_string()).collect();
    } else if matches.is_present("domain") {
        hosts.push(matches.value_of("input").unwrap().to_string());
    } else {
        hosts = read_stdin()?;
    }

    let host_regexs = build_host_regex(&hosts);

    vita::runner(hosts, all_sources, max_concurrent)
        .await
        .iter()
        .flat_map(|a| a.split_whitespace())
        .filter(|b| host_regexs.is_match(&b) && !b.starts_with('*'))
        .map(|d| d.into())
        .collect::<HashSet<String>>()
        .iter()
        .for_each(|s| println!("{}", s)); // why not e? because s is for subdomain xD

    Ok(())
}

fn read_stdin() -> Result<Vec<String>> {
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;
    Ok(buffer.split_whitespace().map(|s| s.to_string()).collect())
}

/// Builds a regex that filters irrelevant subdomains from the results.
/// `.*\.host\.com$`
fn host_regex(host: &str) -> String {
    format!(r".*\.{}$", host.replace(".", r"\."))
}

/// Builds a `RegexSet` to use for filtering irrelevant results.
fn build_host_regex(hosts: &[String]) -> RegexSet {
    // The maximum allowed size of a compiled regex.
    const MAX_SIZE: usize = 10485760;

    RegexSetBuilder::new(
        hosts
            .iter()
            .map(|s| host_regex(&s))
            .collect::<Vec<String>>(),
    )
    .size_limit(MAX_SIZE * 2)
    .dfa_size_limit(MAX_SIZE * 2)
    .build()
    .unwrap()
}

/// Creates the Clap app to use Vita library as a cli tool
fn create_clap_app(version: &str) -> clap::App {
    App::new("vita")
        .version(version)
        .about("Gather subdomains from passive sources")
        .usage("vita -d <domain.com>")
        .arg(Arg::with_name("input").index(1).required(false))
        .arg(
            Arg::with_name("file")
                .help("vita -f <roots.txt>")
                .short("f")
                .long("file"),
        )
        .arg(
            Arg::with_name("domain")
                .help("vita -d domain.com")
                .short("d")
                .long("domain"),
        )
        .arg(
            Arg::with_name("all_sources")
                .help("use sources which require an Api key")
                .short("a")
                .long("all"),
        )
        .arg(
            Arg::with_name("concurrency")
                .help("The number of domains to fetch data for concurrently")
                .short("c")
                .long("concurrency")
                .default_value("100")
                .takes_value(true),
        )
}
