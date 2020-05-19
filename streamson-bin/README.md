# Streamson Bin

Splits stdout (in JSON format) according to given options.
It is supposed to be memory efficient and fast to process.


## Examples
Consider following context of `input.json` file:
```json
{
	"books": [
		{"title": "title 1", "authors": ["author1@exmaple.com"]},
		{"title": "title 2", "authors": ["author2@example.com", "author3@example.com"]}
	]
}

```

### Extract to stdout
```
cat input.json | streamson-bin -P '{"books"}[]'
```

Output:
```
{"books"}[0]: {"title": "title 1", "authors": ["author1@exmaple.com"]}
{"books"}[1]: {"title": "title 2", "authors": ["author2@example.com", "author3@example.com"]}
```

### Extract to stdout without header
```
cat input.json | streamson-bin -p '{"books"}[]{"authors"}'
```

Output:
```
["author1@exmaple.com"]
["author2@example.com", "author3@example.com"]
```

### Extract to file
```
cat input.json | streamson-bin -f '{"books"}[0]{}:/tmp/output.out'
cat /tmp/output.out
```

Output:
```
"title 1"
["author1@exmaple.com"]
```

### Several matchers can be used
```
cat input.json | streamson-bin -p '{"books"}[]{"authors"}[]' -p '{"books"}[]{"title"}'
```

Output:
```
"title 1"
"author1@exmaple.com"
"title 2"
"author2@exmaple.com"
"author3@exmaple.com"
```
