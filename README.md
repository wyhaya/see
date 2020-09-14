
# see

[![GitHub Workflow Status](https://img.shields.io/github/workflow/status/wyhaya/see/Build?style=flat-square)](https://github.com/wyhaya/see/actions)
[![Crates.io](https://img.shields.io/crates/v/see.svg?style=flat-square)](https://crates.io/crates/see)
[![LICENSE](https://img.shields.io/crates/l/see.svg?style=flat-square)](https://github.com/wyhaya/see/blob/master/LICENSE)

---

A simple and fast web server

## Features

* Supported HTTP/1 and HTTP/2
* Content compression (auto, gzip, deflate, br)
* Support directory list
* HTTP request proxy
* ...

## Install

### Binary

[Download](https://github.com/wyhaya/see/releases) the binary from the release page

### Cargo

```bash
cargo install see
# or
cargo install --git https://github.com/wyhaya/see
```

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

[Click](./docs/config.md) to view document
