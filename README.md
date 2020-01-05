
# see 

[![Crates.io](https://img.shields.io/crates/l/see.svg?style=flat-square)](https://github.com/wyhaya/see/blob/master/LICENSE)

`see` is a static web server, developed using rust.

## Feature

## Install

## Use

Quick Start

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

More

```
USAGE:
    see [OPTIONS] [FLAGS] [--] ...

FLAGS:
    -d, detach          Running in the background
    -h, help            Print help information
    -r, restart         Restart the service program
    -s, stop            Stop the daemon
    -t, test            Test the config file for error
    -v, version         Print version number

OPTIONS:
    -c    <FILE>        Specify a configuration file
    start <PORT?>       Quick Start
```

## Config

Use `yaml` format as a configuration file, You can use `see -c /your/config.yml` to specify the configuration file location.

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

## Options

* [host](#host)
* [listen](#listen)
* [https](#https)
* [root](#root)  
* [echo](#echo)
* [file](#file)
* [index](#index)
* [directory](#directory)
* [header](#header)
* [rewrite](#rewrite)
* [compress](#compress)
* [method](#method)
* [auth](#auth)
* [extension](#extension)
* [status](#status)
* [proxy](#proxy)
* [location](#location)
* [var](#var)

### host

Host binding to the site

```yaml
host: exmaple.com    # Exact match
# or
host: ~/*.go+gle.com  # Regular match => photo.goooogle.com
# or
host:                # Multiple
  - example.*
  - *.exmaple.com
```

### listen

Port to be monitored

```yaml
listen: 80  # 0.0.0.0
# or
listen: 80 8080
# or
listen: 127.0.0.1:1234
```

### https

HTTPS option

```yaml
https:
  key: /root/ssl.key
  cert: /root/ssl.pem
```

### root

Directory that requires service

```yaml
root: /root/www 
```

### echo

Output plain text

```yaml
echo: Hello wrold
# or
echo: Hello, ${request_path}
```

### file

Output specified file

```yaml
file: ./www/index.html
```

### index

Index file, default: index.html index.htm
 
```yaml
index: []
# or
index: index.html
# or
index:
  - index.html
  - index.htm
```

### directory

File list option

```yaml
# Show directory only
directory: true | false | [option]

# Display creation time
directory:
  time: true | false | "%Y-%m-%d %H:%M"

# Display file size
directory:
  size: true | false
```

### header

Header in response

```yaml
header:    
  Access-Control-Allow-Origin: "*"
  Set-Cookie: "12345"
```

### rewrite
  
HTTP rewrite config

```yaml
rewrite: https://example.com     # 
# or
rewrite: /example 301
# or
rewrite: /example 302
```

### compress

File compression options

```yaml
compress: true
# or
compress:         
  mode: gzip  # optional value 'auto' 'gzip' 'deflate' 'br'
  level: 3    # Integer with compression level 0-9
  extension:  # Which files are compressed, default: html css js json xml
    - css
    - js
```

### method

Method of allowing requests, default: GET HEAD

```yaml
method:
  - POST
  - PUT
```

### auth
  
HTTP user and password verification

```yaml
auth:  
  user: username
  password: password
```

### extension

Sets file extension fallbacks

```yaml
extension:  
  - html
  - htm
```

### status

Custom status page

```yaml
status:  
  403: 403.html
  404: 404.html
  500: 500.html
  502: 502.html
```

### proxy

Proxy options

```yaml
proxy:
  uri: http://example.com  # Proxy address
  method: GET              # Change the method of the proxy
  timeout: 3s              # timeout: 2d 2m 2h 2s 2ms format
  header:                  # Header in proxy request
    key: value
# or
proxy:                     # Rand proxy
  uri:
    - http://example1.com
    - http://example2.com

```

### location

Separately configured for certain locations

Match pattern

* Use `glob` matching by default
* `~` Use regular expressions
* `^` Match start character
* `$` Match end character

```yaml
location: 
  /public:
    directory:
      time: true
      size: true
  /private:
    auth:
      user: username
      password: password
```

### var

Built-in variables can be used in `echo` `rewrite` `header` `proxy`

```
${VAR}
```

```yaml
echo: Hello ${request_path}, ${request_header_host}
```

* `${request_path}`
* `${request_query}`
* `${request_uri}`
* `${request_method}`
* `${request_query_NAME}`
* `${request_header_NAME}`

---

