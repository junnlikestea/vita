# Vita
A tool to find subdomains or domains from passive sources.

### Installation
I'll have some binaries uploaded for different platforms in the next few days, if you want to go ahead and install

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

With a list of domains from a file
```
vita -f path/to/domains.txt
```

With a list of domains from stdin
```
vita < /path/to/domains.txt
```

if you want to include sources which require API keys, add the `-a` or `-all` flag, for example:
```
vita -d hackerone.com -a
``` 

### Sources
* AnubisDB
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
* wayback machine
* dns.bufferover.run

### How to set your Api Keys
Add a `.env` file to the directory, and set the following variables:
* Facebook:
	* Needs `FB_APP_ID` and `FB_APP_SECRET` set.
* Spyse:
	* Needs `SPYSE_TOKEN` set.
* Security Trails:
	* Needs `SECURITY_TRAILS_KEY` set.

If you hit rate limits or authentication fails, you will get a message in `stderror`, this will not
be printed in the final output.

Thanks to [0xatul](https://twitter.com/atul_hax) for the feedback!
