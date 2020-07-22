

# Variable


Built-in variables can be used in `echo` `rewrite` `header` `proxy`

```
${VAR}
```

```yaml
echo: Hello ${path}, ${header_host}
```

* `${path}`
* `${query}`
* `${url}`
* `${method}`
* `${query_NAME}`
* `${header_NAME}`

---
