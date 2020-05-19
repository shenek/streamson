# Streamson

A memory efficient set of tools to split large JSONs into a smaller parts.

* [streamson-lib](streamson-lib/README.md) - Core Rust library
* [streamson-bin](streamson-bin/README.md) - Binary to split JSONs

## Motivation
Imagine a situation when you get a very large JSON input.
And you're not able to parse it because it can't fit into the memory.

E.g. someone thought that it might be a good idea to dump entire database into
a single json file.

```
{
	"access_logs": [...],  # millions of records
	"users": [{"name": "Example User", ...}, ...],
	"groups": [...],
	...
}
```

Tools present in this repository are able to help you e.g.
to split it into a several files according to it's path machter.

For example:
* `{"access_logs"}[]` will extract array from `"access_logs"`.
* `{"users"}[]{"name"}` will extract all user names.
