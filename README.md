
# see 

[![GitHub Workflow Status](https://img.shields.io/github/workflow/status/wyhaya/see/Build?style=flat-square)](https://github.com/wyhaya/see/actions)
[![Crates.io](https://img.shields.io/crates/v/see.svg?style=flat-square)](https://crates.io/crates/see)
[![LICENSE](https://img.shields.io/crates/l/see.svg?style=flat-square)](https://github.com/wyhaya/see/blob/master/LICENSE)

---

This is a static http file server built on `tokio` and `hyper`

## Feature

*
*
*

## Install

[Download](https://github.com/wyhaya/see/releases) the binary from the release page

Or use `cargo` to install

```bash
cargo install see
```

<details>
<summary>More</summary>
</details>

## Use

Quick start

```bash
see start
# or
see start 8080
```

Start according to the configuration file

```bash
see
# or
see -c /your/config.yml
```

## Config

Use `yaml` format as a configuration file, You can use `see -c config.yml` to specify the configuration file location.

The default configuration file is in `~/.see/config.yml`

A simple example: 

```yaml
- server:
    listen: 80
    root: /root/www

- server:
    listen: 443
    host: domain.com
    root: /root/www
    https:
      key: /root/ssl.key
      cert: /root/ssl.pem
```

Complete configuration documentation: [doc.md](./doc.md)

---