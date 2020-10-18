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

If you would like some more verbose output you can use the `-v` flag. There are
different levels of verbosity ranging from noisy to informational, most of the
time I just use `info`. This is all printing to stderr, so it won't be captured
in the results.
* `info`: General information like how many results each source returned.
* `debug`: Lots and lots of information about what's going on under the hood.
```
vita -d hackerone.com -v info
```

### Common error - Too many open files
Vita uses async concurrent http requests under the hood. If you encounter an error 
similar to *"Too many open files"* it means that there isn't enough available file descriptors on 
your system. You can fix this by increasing the hard and soft limits. There are 
lots of different guides available to increase the limits [but here is one for linux](https://www.tecmint.com/increase-set-open-file-limits-in-linux/). 

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
	* Can be found under the [developer tab](https://intelx.io/account?tab=developer)

If you hit rate limits or authentication fails, the source will just be ignored from the list of potential sources.

### A note on tuning the concurrency
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

[ProjectDiscovery](https://chaos.projectdiscovery.io/#/) For Chaos which is a great data source.




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

