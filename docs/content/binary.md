+++
title = "Binary"
description = "How to use Binary"
weight = 2
+++

# Installation
There are actually two versions of binary which can be installed.
One is provided as a rust binary directly.
Second is a python script written on top of rust bindings.
Both binaries should accept more or less same arguments.

## Python
##### From **pypi.org**
> Might require rust to be installed, for configurations for which there are no .whl files.
```
pip install streamson-python
```

##### From source
> You need to have [**rust installed**](https://rustup.rs/).

```bash
git clone https://github.com/shenek/python-streamson.git
cd python-streamson
pip install .
```

## Rust
> You need to have [**rust installed**](https://rustup.rs/).

##### From **crates.io**
```shell
cargo install streamson-bin
```

##### From source
```shell
git clone https://github.com/shenek/streamson.git
cd streamson
cargo install --path streamson-bin/
```

# Recepies
To make it simpler `sson` binary (rust) is used in following examples.
Pyhton script/binary `streamson` should work here as well.

## Extract to separate files
Imagine a situation where we have following JSON structure in e.g. `input.json`:
```json
{
	"users": [
		{"name": "carl", "uid": 1},
		...
	],
	"groups": [
		{"name": "admins", "gid": 1},
		...
	]
}
```
##### And we want to store both users and groups to separate files (`users.json`, `groups.json`).
```shell
cat input.json | sson trigger \
	-m 'simple.1:{"users"}'\
	-h file.1:users.json\
	-m 'simple.2:{"groups"}'\
	-h file.2:groups.json\
	> /dev/null
```

## Analyse JSON structure
Image we have following JSON stored in e.g. `input.json`.
```json
{
	"a": 10,
	"b": 50
}
[
	{"c": 3},
	{"c": 4},
	{"c": 5, "d": false},
	{"c": 6, "e": [1, 2, 3, 4, 5, 6]}
]
```
And we want to display some info regarding the JSON structure.
##### Solution
```shell
cat input.json | sson all -h a > /dev/null
```
And after the entire JSON is processed following lines should be printed to stderr.
```
JSON structure:
  <root>: 2
  []: 4
  []{"c"}: 4
  []{"d"}: 1
  []{"e"}: 1
  []{"e"}[]: 6
  {"a"}: 1
  {"b"}: 1
```

## Remove/Add indentation
Imagine we have a JSON e.g. `input.json`.
```
{
		"uu": 4, "bb": [
1, 2, 3
			]
}

```

##### And we want to convert it so it would be as small as possible
```shell
cat input.json | sson all -h d
{"uu":4,"bb":[1,2,3]}
```

##### Or we want to have it more readable
```shell
cat input.json | sson all -h d:2
{
  "uu": 4,
  "bb": [
    1,
    2,
    3
  ]
}
```

## Extract users to a stream
Imagine input JSON e.g. `input.json`
```json
{
	"users": [
		{"id": 1, "name": "carl"},
		{"id": 2, "name": "bob"},
		{"id": 3, "name": "alice"}
	]
}
```
##### And we want to extract stream of `users` to the stdout
```shell
cat input.json | sson extract -m 'simple:{"users"}[]'
{"id":1,"name":"carl"}{"id":2,"name":"bob"}{"id":3,"name":"alice"}
```

## Mask secrets
Imagine input JSON e.g. `input.json`
```json
{
	"users": [
		{"id": 1, "name": "carl", "password": "1234"},
		{"id": 2, "name": "bob", "password": "admin"},
		{"id": 3, "name": "alice", "password": "root"}
	]
}
```
##### And we want to mask all passwords.
```shell
cat input.json | sson convert -m 'simple:{"users"}[]{"password"}' -h 'r:"***"'
{
	"users": [
		{"id": 1, "name": "carl", "password": "***"},
		{"id": 2, "name": "bob", "password": "***"},
		{"id": 3, "name": "alice", "password": "***"}
	]
}
```

## Shorten description
Imagine input JSON e.g. `input.json`
```json
{
	"users": [
		{"name": "carl", "bio": "too long text to read"},
		{"name": "bob", "bio": "another long text to read"},
		{"name": "alice", "bio": "even longer text to read"}
	]
}
```
##### And we want have a shorter bio
```shell
cat input.json | sson convert -m 'simple:{"users"}[]{"bio"}' -h 's:5,..."'
{
    "users": [
        {"name": "carl", "bio": "too l..."},
        {"name": "bob", "bio": "anoth..."},
        {"name": "alice", "bio": "even ..."}
    ]
}
```

## Remove fields to separate files
Imagine input JSON e.g. `input.json`
```json
{
	"users": [
		{"id": 1, "name": "carl", "email": "carl@example.com"},
		{"id": 2, "name": "bob", "email": "bob@example.com"},
		{"id": 3, "name": "alice", "email": "alice@example.com"}
	]
}
```

##### And we want to remove `id` and `email` and store it to separate files
```shell
cat input.json | sson filter \
	-m 'simple.1:{"users"}[]{"id"}'\
	-h file.1:ids.txt\
	-m 'simple.2:{"users"}[]{"email"}'\
	-h unstringify.2\
	-h file.2:emails.txt
{
	"users": [
		{ "name": "carl"},
		{ "name": "bob"},
		{ "name": "alice"}
	]
}
```

`ids.txt` should contain:
```
1
2
3
```
`emails.txt` should contain:
```
carl@example.com
bob@example.com
alice@example.com
```
