
# see

[![GitHub Workflow Status](https://img.shields.io/github/workflow/status/wyhaya/see/Build?style=flat-square)](https://github.com/wyhaya/see/actions)
[![Crates.io](https://img.shields.io/crates/v/see.svg?style=flat-square)](https://crates.io/crates/see)
[![LICENSE](https://img.shields.io/crates/l/see.svg?style=flat-square)](https://github.com/wyhaya/see/blob/master/LICENSE)

---

An HTTP server for hosting static files

## Features

* Supported HTTP/1 and HTTP/2
* Content compression (auto, gzip, deflate, br)
* Support directory list
* HTTP request proxy
* Multiple website binding
* [More...](https://see.wyhaya.com/config.html)

## Install

[Download](https://github.com/wyhaya/see/releases) the binary from the release page

Or use `cargo` to install

```bash
cargo install see
```

More ways to install [click](https://see.wyhaya.com/install.html)

## Usage

Quick start in current directory

```bash
see start
```

Use specified port and directory

```bash
see start -b 0.0.0.0:80 -p /root/www
```

## Config

> Complete configuration documentation: [click](https://see.wyhaya.com/config.html)

Use `yaml` format as a configuration file, You can use `see -c config.yml` to specify the configuration file location

The default configuration file is in `~/.see/config.yml`

#### A simple example: 

```yaml
- server:
    listen: 80
    root: /root/www

- server:
    listen: 443
    root: /root/www
    https:
      name: domain.com
      key: /your/ssl.key
      cert: /your/ssl.pem
```

---