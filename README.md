
# see [![GitHub Workflow Status](https://img.shields.io/github/workflow/status/wyhaya/see/Build?style=flat-square)](https://github.com/wyhaya/see/actions) [![Crates.io](https://img.shields.io/crates/v/see.svg?style=flat-square)](https://crates.io/crates/see) [![LICENSE](https://img.shields.io/crates/l/see.svg?style=flat-square)](https://github.com/wyhaya/see/blob/master/LICENSE) [![Document](https://img.shields.io/badge/config-document-success.svg?style=flat-square)](./docs)

A simple and fast web server

---

## Features

* Built on [tokio](https://github.com/tokio-rs/tokio) and [hyper](https://github.com/hyperium/hyper)
* TLS verification based on [rustls](https://github.com/ctz/rustls)
* Supports `HTTP/1` and `HTTP/2`
* Content compression `auto` `gzip` `deflate` `br`
* HTTP request proxy
* ...

## Install

#### Binary

[Download](https://github.com/wyhaya/see/releases) the binary from the release page

#### Cargo

```bash
cargo install see
# or
cargo install --git https://github.com/wyhaya/see
```

#### Docker

```bash
docker pull wyhaya/see
```

<details>
    <summary>Example</summary>

---
Add the following to `see.conf`

```
server {
    listen 80
    echo Hello world
}
```

```
mkdir see && vim see/see.conf
```

Run container

```bash
docker run -idt --name see -p 80:80 -p 443:443 -v '$PWD'/see:/ wyhaya/see
```

Open [localhost](http://127.0.0.1) and you should see `hello world`

</details>

## Usage

Quick start in current directory

```bash
see start
```

Use specified port and directory

```bash
see start -b 80 -p /root/www
```

## Config

You can use `see -c [FILE]` to specify the configuration file location

The default configuration file is in `~/.see.conf` 

[Documents](./docs/)

#### A simple example: 

```
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

