extern crate vita;
use clap::{App, Arg};
use regex::{RegexSet, RegexSetBuilder};
use std::collections::HashSet;
use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use vita::error::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let args = create_clap_app("v0.1.13");
    let matches = args.get_matches();
    let mut all_sources = false;
    let mut hosts: Vec<String> = Vec::new();
    let max_concurrent: usize = matches.value_of("concurrency").unwrap().parse()?;
    let timeout: u64 = matches.value_of("timeout").unwrap().parse()?;
    let verbosity = matches.value_of("verbosity").unwrap();
    env::set_var("RUST_APP_LOG", verbosity);
    pretty_env_logger::init_custom_env("RUST_APP_LOG");

    if matches.is_present("all_sources") {
        all_sources = true;
    }

    if matches.is_present("file") {
        let input = matches.value_of("input").unwrap();
        hosts = read_input(Some(input))?;
    } else if matches.is_present("domain") {
        hosts.push(matches.value_of("input").unwrap().to_string());
    } else {
        hosts = read_input(None)?;
    }

    let host_regexs = build_host_regex(&hosts);

    let subdomains = vita::runner(hosts, all_sources, max_concurrent, timeout).await?;
    subdomains
        .iter()
        .flat_map(|a| a.split_whitespace())
        .filter(|b| host_regexs.is_match(&b) && !b.starts_with('*'))
        .map(|d| d.into())
        .collect::<HashSet<String>>()
        .iter()
        .for_each(|s| println!("{}", s)); // why not e? because s is for subdomain xD

    Ok(())
}

/// Reads input from stdin or a file
fn read_input(path: Option<&str>) -> Result<Vec<String>> {
    let mut contents = Vec::new();
    let reader: Box<dyn BufRead> = match path {
        Some(filepath) => {
            Box::new(BufReader::new(File::open(filepath).map_err(|e| {
                format!("tried to read filepath {} got {}", &filepath, e)
            })?))
        }
        None => Box::new(BufReader::new(io::stdin())),
    };

    for line in reader.lines() {
        contents.push(line?)
    }

    Ok(contents)
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
                .help(
                    "The number of domains to fetch data for concurrently. This is actually
                    the size of the channel buffer which in some way limits the number of concurrent
                    tasks",
                )
                .short("c")
                .long("concurrency")
                .default_value("200")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("verbosity")
                .help(
                    "different levels of verbosity you can set for debugging, 
                    values include: debug,info and warn",
                )
                .short("v")
                .long("verbosity")
                .default_value("")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("timeout")
                .help(
                    "connection timeouts can be useful if you don't want to wait
                    for sources like wayback archive which quite a while. Default is 10 seconds.",
                )
                .short("t")
                .long("timeout")
                .default_value("15")
                .takes_value(true),
        )
}
