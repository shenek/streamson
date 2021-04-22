+++
title = "Streamson"
sort_by = "weight"
+++


# What it is
Streamson is a tool to process large JSON inputs.
It tries to be memory efficient and it assumes that entire JSON won't fit into memory.


# What it is not
Streamson is not a JSON parser which tries to convert data to some kind of internal representation.
It simly expects UTF-8 encoded input and it is able to convert it to another UTF-8 encoded output.
Note that the output doesn't really need to be a valid JSON.

# Motivation
Imagine a situation when you get a very large JSON input. And you're not able to parse it because it can't fit into the memory.

E.g. someone thought that it might be a good idea to dump entire database into a single json file.
```json
{
	"logs": [...],  # millions of records
	"users": [{"name": "Example User", ...}, ...],
	"groups": [...],
	...

}
```

And you want to store the `users` into a separate file.

This can be done quite easily using streamson binary.
```bash
cat input.json | sson extract -m simple:'{"users"}[]' -b '{' -a '}' -S , > users.json

```

The output here is a valid JSON array which contains objects (users).
