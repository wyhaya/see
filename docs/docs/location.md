

# Location

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


