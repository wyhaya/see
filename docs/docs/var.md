

# Var


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
