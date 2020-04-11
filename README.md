# Streamson

**NOTE that this project is still work in progress and currently does noting usefull**

Processes a stream of bytes (in json format) to a stream of subjson bytes.
It is meant to be used for a memory efficient large json file processing.


## Example
Input:
```json
{
	books: [
		{"title": "title 1", "authors": ['author1@exmaple.com']},
		{"title": "title 2", "authors": ['author2@example.com', 'author3@example.com']}
	]
}

```

Path
```
{"books"}[]
```

Output:
```
{"books"}[0] {"title": "title 1", "authors": ['author1@exmaple.com']}
{"books"}[1] {"title": "title 2", "authors": ['author2@example.com', 'author3@example.com']}
```

Path
```
{"books"}[]{"authors"}
```

Output:
```
{"books"}[0]{"authors"} ['author1@exmaple.com']
{"books"}[1]{"authors"} ['author2@example.com', 'author3@example.com']
```

Path
```
{"books"}[0]{}
```

Output:
```
{"books"}[0]{"title"} "title 1"
{"books"}[0]{"authors"} ["author1@exmaple.com"]
```
