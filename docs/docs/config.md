# Configuration

## listen

Bind to socket address

```yaml
listen: 80  # 0.0.0.0
# or
listen: 80 8080
# or
listen: 127.0.0.1:1234
```

## https

HTTPS option

```yaml
https:
  name: domain.com
  key: /root/ssl.key
  cert: /root/ssl.pem
```

## host

Match http header host

```yaml
host: exmaple.com    # Exact match
# or
host: ~/*.go+gle.com  # Regular match => photo.goooogle.com
# or
host:                # Multiple
  - example.*
  - *.exmaple.com
```

## root

Directory that requires service

```yaml
root: /root/www 
```

## echo

Output plain text

```yaml
echo: Hello wrold
# or
echo: Hello, ${request_path}
```

## file

Output specified file

```yaml
file: ./www/index.html
```

## rewrite
  
HTTP rewrite config

```yaml
rewrite: https://example.com     # Default 302
# or
rewrite: /example 301
# or
rewrite: /example 302
```

## directory

File list options

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

## compress

File compression options

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

## header

Header in response

```yaml
header:    
  Access-Control-Allow-Origin: "*"
  Set-Cookie: "12345"
```

## method

Method of allowing requests, default: GET HEAD

```yaml
method:
  - POST
  - PUT
```

## auth
  
HTTP user and password verification

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

> If you use a relative path, you must set root

```yaml
error:  
  403: 403.html
  404: 404.html
  500: 500.html
  502: 502.html
  504: 504.html
```

## proxy

Proxy options

```yaml
proxy:
  uri: http://example.com  # Proxy address
  method: GET              # Change the method of the proxy
  header:                  # Header in proxy request
    key: value
# or
proxy:                     # Rand proxy
  uri:
    - http://example1.com
    - http://example2.com${request_uri}

```

## log

Access log

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

