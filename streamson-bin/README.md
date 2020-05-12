# Streamson Bin

Processes a stream of bytes (in JSON format) to a stream of subjson bytes.
It is meant to be used for a memory efficient large json file processing.


## Examples
Input:
```json
{
	"books": [
		{"title": "title 1", "authors": ["author1@exmaple.com"]},
		{"title": "title 2", "authors": ["author2@example.com", "author3@example.com"]}
	]
}

```

Cmd
```
cat input.json | streamson-bin -p '{"books"}[]'
```

Output:
```
{"books"}[0]: {"title": "title 1", "authors": ["author1@exmaple.com"]}
{"books"}[1]: {"title": "title 2", "authors": ["author2@example.com", "author3@example.com"]}
```

Path
```
cat input.json | streamson-bin -p '{"books"}[]{"authors"}'
```

Output:
```
{"books"}[0]{"authors"}: ["author1@exmaple.com"]
{"books"}[1]{"authors"}: ["author2@example.com", "author3@example.com"]
```

Path
```
cat input.json | streamson-bin -p '{"books"}[0]{}'
```

Output:
```
{"books"}[0]{"title"}: "title 1"
{"books"}[0]{"authors"}: ["author1@exmaple.com"]
```
