extern crate vita;
use self::vita::*;
use clap::{App, Arg};
use std::fs;
use std::io::{self, Read};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;

fn create_clap_app(version: &str) -> clap::App {
    // Add support to not include subdomains.
    let app = App::new("vita")
        .version(version)
        .about("Gather subdomains from passive sources")
        .usage("vita <domain.com>")
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
        );

    app
}

fn read_stdin() -> Result<Vec<String>> {
    let mut buffer = String::new();
    let mut res = Vec::new();
    io::stdin().read_to_string(&mut buffer)?;
    for line in buffer.split_whitespace() {
        res.push(line.to_string())
    }
    Ok(res)
}

#[async_std::main]
async fn main() -> Result<()> {
    let args = create_clap_app("v0.1.0");
    let matches = args.get_matches();
    let mut all_sources = false;
    let mut hosts: Vec<String> = Vec::new();

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

    let results = vita::runner(hosts, all_sources).await;
    for r in results {
        println!("{}", r);
    }
    Ok(())
}
