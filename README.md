# Vita
![release](https://github.com/junnlikestea/vita/workflows/release/badge.svg)
[![Build status](https://github.com/junnlikestea/vita/workflows/Continuous%20integration/badge.svg)](https://github.com/junnlikestea/vita/actions)

Vita is a tool to gather subdomains from passive sources much like [tomnomnom's assetfinder](https://github.com/tomnomnom/assetfinder).


### Installation
Precompiled binaries for vita are available in the [releases](https://github.com/junnlikestea/vita/releases) tab. Just pick your platform and extract the archive that contains the binary.

## Building it yourself 
If you want to build it yourself you will need to install Rust, you can get the [official installation](https://www.rust-lang.org/tools/install) from the Rust website.

To build Vita:
```
$ git clone https://github.com/junnlikestea/vita
$ cd vita
$ cargo build --release
$ ./target/release/vita --version
```

### Usage
With a single domain, and collect data from Apis' which don't require keys.:
```
vita -d hackerone.com
```
by default the results will be unique, and will filter subdomains not related to your root domain, or domains if you choose to supply multiple.

With a list of domains from a file:
```
vita -f path/to/domains.txt
```

With a list of domains from stdin:
```
vita < /path/to/domains.txt
```

If you want to include sources which require API keys, add the `-a` or `-all` flag, for example:
```
vita -d hackerone.com -a
``` 
By default it will just ignore services you don't supply keys for.


### Sources
* SonarSearch
* C99
* ProjectDiscovery Chaos
* AnubisDB
* Alienvault
* Binaryedge 
* Certspotter
* Crt.sh
* Hackertarget
* Threatcrowd
* VirusTotal
* Sublis3r
* Security Trails
* Spyse
* Urlscan.io
* Facebook
* Threatminer
* Wayback Machine
* dns.bufferover.run
* tls.bufferover.run
* IntelligenceX
* PassiveTotal

### How to set your Api Keys
Add a `.env` file to the tool directory or add the following to your existing `.env` file:
* Binaryedge:
	* Needs `BINARYEDGE_TOKEN` set
* ProjectDiscovery Chaos
	* Needs `CHAOS_KEY` set
* Facebook:
	* Needs `FB_APP_ID` and `FB_APP_SECRET` set.
* Spyse:
	* Needs `SPYSE_TOKEN` set.
* Security Trails:
	* Needs `SECURITY_TRAILS_KEY` set.
* C99: 
	* Needs `C99_KEY` set.
* PassiveTotal:
	* Needs `PASSIVETOTAL_KEY` and `PASSIVETOTAL_SECRET` set
	* Can be found under the account settings page.
* IntelligenceX:
	* Needs `INTELX_KEY` and `INTELX_URL` to be set
	* Can be found udner the [developer tab](https://intelx.io/account?tab=developer)

If you hit rate limits or authentication fails, the source will just be ignored from the list of potential sources.

### A note on tuning the concurrency
Vita uses Rust's [async-std](https://docs.rs/async-std/1.6.2/async_std/) library under the hood. 
Several environment variables can be used to tune the performance:

* `ASYNC_STD_THREAD_COUNT`: The number of threads that the async-std runtime will start. By default, 
this is one per logical cpu. which may be different than the number of physical cpus. Async-std 
will panic if this is set to any value other than a positive integer.

* `ASYNC_STD_THREAD_NAME`: The name that async-std's runtime threads report to the operating system. 
* The default value is "async-std/runtime".

Currently Vita will limit the search for data to 200 root domains concurrently. If you would like to 
change that limit you can use the `-c` flag:

```
vita -f /path/to/roots.txt -c 400
``` 

### Thanks
[0xatul](https://twitter.com/0xatul) For constant feedback and improvement ideas.
[TomNomNom](https://twitter.com/TomNomNom) For inspiring me to write and release open source tools.
[Cgboal](https://twitter.com/CalumBoal) For [SonarSearch](https://github.com/Cgboal/SonarSearch) 
which is a data source for Vita.


Thanks to all the data source providers, and everyone else I can't seem to remember at this point 
in time. I'll make sure to add you in the future.

### To-do
* Add more paid sources.
* Write some documentation for the underlying library that Vita uses, and prepare publish to crates.io.
* Optimise performance further.

### Disclaimer
Developers have/has no responsibility or authority over any kind of:
* Legal or Law infringement by third parties and users.
* Malicious use capable of causing damage to third parties.
* Illegal or unlawful use of vita.

Thanks to [0xatul](https://twitter.com/atul_hax) for the feedback!
