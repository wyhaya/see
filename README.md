# see

[![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/wyhaya/see/ci.yml?style=flat-square&branch=main)](https://github.com/wyhaya/see/actions)
[![Crates.io](https://img.shields.io/crates/v/see.svg?style=flat-square)](https://crates.io/crates/see)
[![LICENSE](https://img.shields.io/crates/l/see.svg?style=flat-square)](LICENSE)
[![Document](https://img.shields.io/badge/config-document-success.svg?style=flat-square)](docs/)

> [!WARNING]  
> * !!! This project contains severe bugs
> * !!! No plans for fixes or patches
> * !!! DO NOT use it in any environment

## Overview

Simple and fast web server as a single executable with no extra dependencies
required.

## Features

- Built with [Tokio](https://github.com/tokio-rs/tokio) and
  [Hyper](https://github.com/hyperium/hyper)
- TLS encryption through [Rustls](https://github.com/ctz/rustls)
- `HTTP/1` and `HTTP/2` support
- Content compression `auto`, `gzip`, `deflate` or `br`
- Rewrite rules for redirection
- Allow/deny addresses allowing wildcards
- Location with [regex](https://en.wikipedia.org/wiki/Regular_expression)
  matching
- Reverse proxy
- Basic authentication
- Error handling
- Customized logs
- And more

## Usage

Quick start in current directory:

```bash
see start
```

or specify the port and directory via parameters:

```bash
see start -b 80 -p /root/www
```

Also, you can use `see -c [FILE]` to specify a configuration file or just use
the default one in `~/.see.conf`. Below, a simple configuration example to start
the HTTPS server:

```sh
server {
    listen 80
    root /root/www
}

server {
    listen 443
    root /root/www
    host example.com
    https {
        key ./ssl.key
        cert ./ssl.pem
    }
}
```

## Documentation

The documentation is available at [docs/](docs/). Take a look at it to get more
information about more configuration options.

## Installation

Download the compiled executable corresponding to your system from the
[release page](https://github.com/wyhaya/see/releases).

### Cargo

```bash
cargo install see
# or
cargo install --git https://github.com/wyhaya/see
```

### Docker

```bash
docker pull wyhaya/see
```

#### Container

Add the following to `see.conf`:

```sh
server {
    listen 80
    echo Hello, world!
}
```

and run the container:

```bash
docker run -idt --name see -p 80:80 -p 443:443 -v '$PWD'/see:/ wyhaya/see
```

lastly, open the link <http://localhost> and you should see `Hello, world!`.

## Licensing

`see` is released under MIT license. Check the [LICENSE](LICENSE) file for
more details.

---

## ToDo

- [ ] Fix docker container (ubuntu, ca-certificates)
- [ ] Fix the bug of matching https and http on the same port
- [ ] Support global configuration
- [ ] Support certificate with password
- [ ] Daemon for Unix systems and service for Windows
