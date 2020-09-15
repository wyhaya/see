# Configuration

## listen

```yaml
listen: 80  # 0.0.0.0
# or
listen: 80 8080
# or
listen: 127.0.0.1:1234
```

## host

```yaml
host: exmaple.com     # Domain name
# or
host: *.exmaple.com   # Wildcard
# or
host: ~/*.go+gle.com  # Regex
```

## root

```yaml
root: /root/www
# or
root: ./www
```

## https

```yaml
host: example.com
https:
  key: /root/ssl.key
  cert: /root/ssl.pem
```

## echo

```yaml
echo: Hello wrold
# or
echo: Hello, ${request_path}
```

## file

```yaml
file: ./www/index.html
```

## rewrite

```yaml
rewrite: https://example.com     # Default 302
# or
rewrite: /example 301
# or
rewrite: /example 302
```

## directory

```yaml
directory: true | false | [option]

# Display creation time
directory:
  time: true | false | "%Y-%m-%d %H:%M"

# Display file size
directory:
  size: true | false
```

## compress

```yaml
compress: true
# or
compress:         
  mode: gzip     # optional value 'auto' 'gzip' 'deflate' 'br'
  level: default # optional value 'default' 'fastest' 'best'
  extension:     # Which files are compressed, default: html css js json png
    - css
    - js
    - ''
# or
compress:         
  mode: br gzip auto     # Set compression priority
```

## index

Index file, default: `index.html` `index.htm`
 
```yaml
index: index.html
# or
index: []
# or
index:
  - index.html
  - index.htm
```

## header

```yaml
header:    
  Access-Control-Allow-Origin: "*"
  Set-Cookie: "12345"
```

## method

Method of allowing requests, default: `GET` `HEAD`

```yaml
method:
  - POST
  - PUT
```

## auth
  
HTTP authentication

```yaml
auth:  
  user: username
  password: password
```

## extension

Sets file extension fallbacks

```yaml
extension:  
  - html
  - htm
```

## error

Custom error page

> If you use relative path, then you must set the `root` option

```yaml
error:  
  403: 403.html
  404: 404.html
  500: 500.html
  502: 502.html
  504: 504.html
```

## proxy

```yaml
proxy:
  url: http://example.com  # Proxy address
  method: GET              # Change the method of the proxy
  header:                  # Header in proxy request
    key: value
# or
proxy:                     # Rand proxy
  url:
    - http://example1.com
    - http://example2.com${path}
```

## log

```yaml
log: /var/log/www.log
# or
log:
  mode: stdout  # stdout | file
# or
log:
  file: /var/log/www.log
  format: ${request_path} ${header_host}
```

## ip

Allow and deny clients from specified IP

```yaml
ip:
  allow:
    - 127.0.0.1
    - 192.168.0.*
  deny:
    - 172.17.*.*
```


## Location

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
  /private/**:
    auth:
      user: username
      password: password
  ~[1-9]{10}:
    echo: Match regex
  ^start:
    echo: Match start
  $.png:
    echo: Match end
```

## Variable

Built-in variables can be used in `echo` `rewrite` `header` `proxy`

* `${path}`
* `${query}`
* `${method}`
* `${version}`
* `${query_NAME}`
* `${header_NAME}`

```yaml
echo: Hello ${path}, ${header_host}
```

---