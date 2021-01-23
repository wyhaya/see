# Configuration

## listen

```sh
server {
  listen 80 # 0.0.0.0:80
  # or
  listen 80 8080
  # or
  listen 127.0.0.1:1234
}
```

## host

```sh
server {
  host example.com    # Domain name
  # or
  host *.example.com  # Wildcard
}
```

## root

```sh
server {
  root /root/www
  # or
  root ./www
}
```

## https

```sh
server {
  host example.com
  https
    key  /root/ssl.key
    cert /root/ssl.pem
}
```

## echo

```sh
server {
  echo Hello world
  # or
  echo Hello, $`path`
}
```

## file

```sh
server {
  file ./www/index.html
}
```

## rewrite

```sh
server {
  rewrite /example  # Default 302
  # or
  rewrite {
    location https://$`header_host`$`path`
    status 301
  }
}
```

## directory

```sh
server {
  directory on | off | [option]
  # Display creation time
  directory {
    time on | off | "%Y-%m-%d %H:%M"
  }
  # Display file size
  directory {
    size on | off
  }
}
```

## compress

```sh
server {
  compress on
  # or
  compress {
    # Optional value: 'auto' 'gzip' 'deflate' 'br'
    mode gzip
    # Optional value: 'default' 'fastest' 'best'
    level default
    # Which files are compressed, default: 'html' 'css' 'js' 'json' 'png'
    extension css js
  }
  # or
  compress {
    # Set compression priority
    mode br gzip auto
  }
}
```

## index

Index file, default: `index.html`.

```sh
server {
  index index.html index.htm
}
```

## header

```sh
server {
  header {
    Access-Control-Allow-Origin *
    Set-Cookie 12345
  }
}
```

## method

Method of allowing requests, default: `GET` `HEAD`.

```sh
server {
  method POST PUT
}
```

## auth

HTTP authentication.

```sh
server {
  auth {
    user 123
    password 456
  }
}
```

## try

```sh
server {
  try $`path`.html index.html
}
```

## error

Custom error page.

> If you use relative path, then you must set the `root` option.

```sh
server {
  error {
    403 ./403.html
    404 ./404.html
    500 ./500.html
    502 ./502.html
    504 ./504.html
  }
}
```

## proxy

```sh
server {
  proxy {
    url http://example.com  # Proxy address.
    method GET              # Change the method of the proxy.
    header {                # Header in proxy request.
      key value
    }
  }
  # or
  proxy {
    url http://example.com$`path`$`query`
  }
}
```

## log

```sh
server {
  log /var/log/www.log
  # or
  log {
    mode stdout  # stdout | file
  }
  # or
  log {
    file /var/log/www.log
    format $`path` $`header_host`
  }
}
```

## ip

Allow and deny clients from specified IP(s).

```sh
server {
  ip {
    allow 127.0.0.1 192.168.0.*
    deny 172.17.*.*
  }
}
```

## Location

### Modifier

- `@` Matching with glob expression.
- `~` Matching using regular expression.
- `^` Matching the start of a location with a string.
- `$` Matching the end of a location with a string.

```sh
server {
  @ /public {
    directory on
  }
  @ /private/** {
    auth {
      user 123
      password 456
    }
  }
  ~ ^/[1-9]{10}$ {
    echo regex
  }
  ^ /start {
    echo start
  }
  $ .png {
    echo end
  }
}
```

## Variable

Built-in variables can be used in `echo`, `rewrite`, `header` and `proxy`.

```
$`path`
$`query`
$`method`
$`version`
$`query_NAME`
$`header_NAME`
```

```sh
server {
  echo Hello $`path`, $`header_host`
}
```
