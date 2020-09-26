![Security audit](https://github.com/shenek/streamson/workflows/Security%20audit/badge.svg)
![Code Quality](https://github.com/shenek/streamson/workflows/Code%20Quality/badge.svg)
![Release](https://github.com/shenek/streamson/workflows/Release/badge.svg)

# Streamson

A memory efficient set of tools to process large JSONs data.

* [streamson-lib](streamson-lib/README.md) - Core Rust library
* [streamson-bin](streamson-bin/README.md) - Binary to process JSONs from stdout.
* [streamson-tokio](streamson-tokio/README.md) - Helpers to integrates streamson with tokio

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
