+++
title = "Design"
description = "How does it work"
weight = 1
+++

Streamson reads parts of input JSON and performs processes input and optionally generates an output. It expects input to be valid  UTF-8 encoded JSON. 


The JSON processing itself can be done using several [**strategies**](#strategies). You can choose the strategy based on that what you want to do with the JSON. You may want to e.g remove some JSON part or split JSON into smaller parts, etc.

To match a part of the data you need to define a [**matcher**](#matchers). The matcher examines the [**path**](#path) + data type (*obejct*, *array*, *string*, *number*, *boolean*, *null*) and based on the decides whether the data should be matched.

If you have some matched data you may want to do something with it. So you need to define some [**handlers**](#handlers). Handlers accept streams of data and may produce some data output.


# Strategies
## All
Matches all data (no need to set matchers). Handlers can be used to convert the content of entire JSON or to perform some kind of analysis.

## Convert
Alters the JSON by calling convert handlers to matched parts.

## Extract
Alters the JSON as well. It returns only the matched parts as output. Handlers are trigger over the matched stream, but output is not converted.

## Filter
It alters the JSON. If the path is matched the matched part should be removed from output JSON. Handlers can be used here to e.g. store removed parts into a file.

## Trigger
It triggers handlers on matched JSON parts. It doesn't return data as output. So it doesn't meant to convert the output.

# Path
The path is some kind of structure description of currently processed JSON part.
```json
{  // root path starts here
  "users": [  // {"users"} path starts here
    {  // {"users"}[0] path starts here
      "name": "first", // {"users"}[0]{"name"} 
      "id": 1,  // {"users"}[0]{"id"}
    }, // {"users"}[0] path ends here
  ]  // {"users"} path ends here
}  // root path ends here
```

# Matchers
## Simple
Its definition is very similar to [**path**](#path). But it contains a few additions.
* `[]` will match all items in array
* `[1,3-5]` will match second, fourth, fifth and sixth item in array
* `{}` will match any key in object
* `?` will match all items in dict or array
* `*` will match all items in dict or array 0 and times

##### Examples
* `{"a"}[]` matches paths `{"a"}[0]`, `{"a"}[1]`, ...
* `{}[1]` matches paths `{"a"}[1]` , `{"b"}[1]`, ...
* `?[1]` matches same as `{}[1]`, `[][1]`
* `*[1]` matches `[1]` and same as `?[1]`, `??[1]`, `???[1]`, ...

## Depth
You can match based on the [**path**](#path) depth.

## Combinator
It is also possible to combine two matchers together or to negate the matcher.
These matchers need to be wrapped by Combinator matcher.

Combinator itself supports following operations:
* negate (e.g. `~<matcher>`)
* or (e.g. `<matcher1> || <matcher2>`)
* and (e.g. `<matcher1> && <matcher2>`)

##### Examples 
* `2` would match `{"a"}[1]`, `{"a"}[1]{"b"}`, but would not match `{"a"}`
* `2-2` would match `{"a"}[1]`, but would not match `{"a"}[1]{"b"}` nor `{"a"}`

## Regex
You can match [**path**](#path) base on regular expression as well.

##### Examples 
* `^\{"[Uu][Ss][Ee][Rr][Ss]"\}$` would match `{"user"}`, `{"User"}`, `{"USER"}`, ...


# Handlers
## Analyser
It collect informations about the JSON which is being processed.
Basically it count different [**paths**](#path) with squashed arrays.

This handler is useful only with [**all**](#all) strategy, because it needs to
see the data of entire JSON.

<details><summary>Example</summary>
<p>

Input JSON:
```
{
    "users": [
       {"name": "user1", "id": 1},
       {"name": "user2", "id": 2, "is_admin": true},
       {"name": "user3", "id": 3},
    ]
}
```
Collected data would look like this:
```
"": 1  (root elemet)
{"users"}: 1
{"users"}[]: 3
{"users"}[]{"id"}: 3
{"users"}[]{"is_admin"}: 1
{"users"}[]{"name"}: 3
```

</p>
</details>

## Buffer
Collect the data that are being matched.
Note that it can process nested matches.
The buffer itself can be poped when after some
data were fed to the input. This way this handler can
be used to process huge amount of relatively small JSONs.

This handlers is not present in binary, because it wouldn't make much sense,
but still can be useful in [**rust**](/rust) or [**python**](/python) bindings.

<details><summary>Example</summary>
<p>

Matcher ([**combinator**](#combinator) of two [**simple**](#simple) matchers):
```
{"users"}[] || {"users"}[]{"name"}
```
Input JSON:
```
{
    "users": [
       {"name": "user1", "id": 1},
       {"name": "user2", "id": 2, "is_admin": true},
       {"name": "user3", "id": 3},
    ]
}
```
After consuming the entire input the buffer should contain:
```
{"users"}[0]{"name"} "user1"
{"users"}[0] {"name": "user1", "id": 1},
{"users"}[1]{"name"} "user2"
{"users"}[1] {"name": "user2", "id": 2},
{"users"}[2]{"name"} "user3"
{"users"}[2] {"name": "user3", "id": 3},
```

</p>
</details>

## Indenter
Converts the input data to so it can be more human readable or compressed.
This handler is useful only with [**all**](#all) strategy.


<details><summary>Example</summary>
<p>

Input JSON:
```
{
    "users": [
       {"name": "user1", "id": 1},
       {"name": "user2", "id": 2, "is_admin": true},
       {"name": "user3", "id": 3},
    ]
}
```
Output JSON with indent=2:
```
{
  "users": [
    {
      "name": "user1",
      "id": 1
    },
    {
      "name": "user2",
      "id": 2,
      "is_admin": true
    },
    {
      "name": "user3",
      "id": 3
    }
  ]
}
```
Output JSON with undefined indent (compressed):
```
{"users":[{"name":"user1","id":1},{"name":"user2","id":2,"is_admin":true},{"name":"user3","id":3}]}
```

</p>
</details>


## Indexer
Collect info about indexes of JSON parts (start / end).

<details><summary>Example</summary>
<p>

[**Simple**](#simple) matcher:
```
{"users"}[]{"name"}
```

Input JSON:
```
{
    "users": [
       {"name": "user1", "id": 1},
       {"name": "user2", "id": 2, "is_admin": true},
       {"name": "user3", "id": 3},
    ]
}
```

Collected indexes would look like this:
```
{"users"}[0]{"name"} Start(24)
{"users"}[0]{"name"} End(30)
{"users"}[1]{"name"} Start(56)
{"users"}[1]{"name"} End(62)
{"users"}[2]{"name"} Start(108)
{"users"}[2]{"name"} End(114)
```

</p>
</details>

## Output
Writes matched data to given output (could be a file, stdout, ...).

<details><summary>Example</summary>
<p>

[**Simple**](#simple) matcher:
```
{"users"}[]
```

Input JSON:
```
{
    "users": [
       {"name": "user1", "id": 1},
       {"name": "user2", "id": 2, "is_admin": true},
       {"name": "user3", "id": 3},
    ]
}
```
Output defined as a file (e.g. `/tmp/out.json`). And the content
should look like this:
```
{"name": "user1", "id": 1}{"name": "user2", "id": 2, "is_admin": true}{"name": "user3", "id": 3},
```

</p>
</details>


## Regex
Uses sed regex expression convert data (e.g. `s/user/User/`).

<details><summary>Example</summary>
<p>

[**Simple**](#simple) matcher:
```
{"users"}[]{"id"}
```

Input JSON:
```
{
    "users": [
       {"name": "user1", "id": 1},
       {"name": "user2", "id": 2, "is_admin": true},
       {"name": "user3", "id": 3},
    ]
}
```

And with Regex handler `s/user/User/` the output would look like this:
```
{
    "users": [
       {"name": "User1", "id": 1},
       {"name": "User2", "id": 2, "is_admin": true},
       {"name": "User3", "id": 3},
    ]
}
```

</p>
</details>

## Replace
Replaces entire matched data with another fixed data.

<details><summary>Example</summary>
<p>

Simple matcher:
```
{"users"}[]{"is_admin"}
```
Input JSON:
```
{
    "users": [
       {"name": "user1", "id": 1},
       {"name": "user2", "id": 2, "is_admin": true},
       {"name": "user3", "id": 3},
    ]
}
```
Output with `false`:
```
{
    "users": [
       {"name": "user1", "id": 1},
       {"name": "user2", "id": 2, "is_admin": false},
       {"name": "user3", "id": 3},
    ]
}
```

</p>
</details>

## Shorten
Make matched data shorter. Note that this handler should be applied to strings only.

<details><summary>Example</summary>
<p>

Simple matcher:

```
{"users"}[]{"name"}
```

Input JSON:

```
{
    "users": [
       {"name": "user1", "id": 1},
       {"name": "user2", "id": 2, "is_admin": true},
       {"name": "user3", "id": 3},
    ]
}
```

Shorten with 2 max size and `..."` and terminator:

```
{
    "users": [
       {"name": "us...", "id": 1},
       {"name": "us...", "id": 2, "is_admin": true},
       {"name": "us...", "id": 3},
    ]
}
```

</p>
</details>

## Unstringify
Unstringifies matched data.
e.g. (`"{\"a\":5}"` will be converted to `{"a":5}`.

<details><summary>Example</summary>
<p>

Simple matcher:
```
{"users"}[]{"is_admin"}
```

Input JSON
```
{
    "users": [
       {"name": "user1", "id": 1},
       {"name": "user2", "id": 2, "is_admin": "true"},
       {"name": "user3", "id": 3},
    ]
}
```

After Unstringify was used the output should look like this:
```
{
    "users": [
       {"name": user1, "id": 1},
       {"name": user1, "id": 2, "is_admin": true},
       {"name": user1, "id": 3},
    ]
}
```
</p>
</details>


## Group handler
Handlers can be also grouped together. To determine the grouping behaviour it is important to
determine whether its subhandlers are converting data. You can imagine the grouped handlers as a
list. And is processed in following way. If handler is not converting data the input data are passed to
the handler itself and to the next handler. However if handler converts data, the data are passed to
handler itself and handlers output data are passed to the next handler.

```
Data1 -> handlerA(converts=false) -> Data1 -> handlerB(converts=true) -> Data2
```

# Gotchas
Although streamson is memory efficient there can still be situations when it consumes quite huge amount of memory.
A several situations may occure which cause it's memory inefficiency.
Therefore it is not wise to run streamson on a JSON input which you don't trust. 

## Too long keys
Imagine that key in JSON object is just too big.
```json
{
	"3.14159...<thousands of numbers>": "pi"
}
```

However streamson should handle 
```json
{
	"pi": "3.14159...<thousands of numbers>"
}
```
fine for non-buffering handlers.

## Expanding
Now lets say we read a JSON file which is only build one level on the top of another.

```json
[
	1,
	[
		2,
		[
			3,
			[
				4,
				...
			]
		]
	]
]
```

At least streamson has to store the [**path**](#path) in some kind of stack.
And this stack could become quite huge in such situation.
