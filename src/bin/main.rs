extern crate vita;
use clap::{App, Arg};
use regex::Regex;
use std::fs;
use std::io::{self, Read};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;

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
    let regexs: Vec<Regex> = hosts.clone().into_iter().map(|s| host_regex(&s)).collect();
    let results = vita::runner(hosts, all_sources).await;
    for r in results {
        if is_relevant(&regexs, &r) {
            println!("{}", r);
        }
    }
    Ok(())
}

fn create_clap_app(version: &str) -> clap::App {
    // Add support to not include subdomains.
    App::new("vita")
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
        )
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

// instead of returning the match, we could just return a bool if it matches.
// fine for 1 host but what about many?
fn is_relevant(reg: &[Regex], target: &str) -> bool {
    for re in reg.iter() {
        if re.is_match(target) {
            return true;
        };
    }
    false
}

// builds a regex that filters junk results
fn host_regex(host: &str) -> Regex {
    let mut prefix = r".*\.".to_owned();
    let h = host.replace(".", r"\.");
    prefix.push_str(&h);
    Regex::new(&prefix).unwrap()
}
