extern crate vita;
use clap::{App, Arg};
use regex::RegexSet;
use std::collections::HashSet;
use std::fs;
use std::io::{self, Read};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;

#[async_std::main]
async fn main() -> Result<()> {
    let args = create_clap_app("v0.1.6");
    let matches = args.get_matches();
    let mut all_sources = false;
    let mut exclude_rapidns = false;
    let mut hosts: Vec<String> = Vec::new();

    if matches.is_present("all_sources") {
        all_sources = true;
    }

    if matches.is_present("exclude_rapidns") {
        exclude_rapidns = true;
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

    //TODO: can we avoid making this second vec ?
    let ree_host: Vec<String> = hosts.iter().map(|s| host_regex(&s)).collect();
    let host_regexs = RegexSet::new(&ree_host).unwrap();

    vita::runner(hosts, all_sources, exclude_rapidns)
        .await
        .iter()
        .flat_map(|a| a.split_whitespace())
        .filter(|b| host_regexs.is_match(&b))
        .filter(|c| !c.starts_with('*'))
        .map(|d| d.into())
        .collect::<HashSet<String>>()
        .iter()
        .for_each(|s| println!("{}", s)); // why not e? because s is for subdomain xD

    Ok(())
}

// Creates the Clap app to use Vita library as a cli tool
fn create_clap_app(version: &str) -> clap::App {
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
        .arg(
            Arg::with_name("exclude_rapidns")
                .help("exclude using RapidDNS as a source")
                .short("e")
                .long("exclude-rapidns"),
        )
}

fn read_stdin() -> Result<Vec<String>> {
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;
    Ok(buffer.split_whitespace().map(|s| s.to_string()).collect())
}

// builds a regex that filters junk results
// .*\.host\.com$
fn host_regex(host: &str) -> String {
    format!(r".*\.{}$", host.replace(".", r"\."))
}
