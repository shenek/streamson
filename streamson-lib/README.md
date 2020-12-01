[![docs.rs](https://docs.rs/streamson-lib/badge.svg)](https://docs.rs/streamson-lib)

# Streamson Lib

Rust library to handle large JSONs.

Note that it doesn't fully validates whether the input JSON is valid.
This means that invalid JSONs might pass without an error.

## Strategies

| Strategy | Processes data | Buffers matched data | Nested matches | Uses handlers |
| -------- | -------------- | -------------------- | -------------- | ------------- |
| Trigger  | No             | Yes                  | Yes            | Yes           |
| Filter   | Yes            | No                   | No             | No            |
| Extract  | Yes            | Yes                  | No             | No            |
| Convert  | Yes            | Yes                  | No             | Yes           |


### Trigger strategy

It doesn't actually perform parses json into data. It just takes JSON parts and triggers handlers when a path is matched.


### Filter strategy

It actually alters the JSON. If the path is matched the matched part should be removed from output json.


### Extract strategy

Only extracts matched data, nothing else.


### Convert strategy

Alters the JSON by calling convert functions to matched parts.


## Matchers

Structures which are used to match a part of JSON.

### Simple

It matches path in JSON. For example:
```json
{
	"users": [
		{"name": "carl"},
		{"name": "bob"}
	],
	"groups": [
		{"name": "admins"},
		{"name": "staff"}
	]
}
```
Simple path `{"users"}[0]{"name"}` would match `"carl"`.

Simple path `{"users"}[]{"name"}` would match `"carl"` and `"bob"`.

Simple path `{}[0]{"name"}` would match `"carl"` and `"admins"`.

Simple path `??{"name"}` would match `"carl"`, `"bob"`, `"admins"` and `"staff"`.

Simple path `*{"name"}` would match `"carl"`, `"bob"`, `"admins"` and `"staff"`.


### Depth

Matches depth in JSON path. It has min length and max length ranges (max is optional).


### All

It matches any JSON element.
It is used only for some specific purpuses (such as JSON analysis).


### Combinator

Wraps one or two matchers. It implements basic logic operators (`NOT`, `OR` and `AND`).


## Examples
### Trigger
```rust
use streamson_lib::{strategy, error::General, handler::PrintLn, matcher::Simple};
use std::sync::{Arc, Mutex};
use std::io::prelude::*;

let mut trigger = strategy::Trigger::new();
let handler = Arc::new(Mutex::new(PrintLn::new()));
let matcher = Simple::new(r#"{"users"}[]"#).unwrap();
trigger.add_matcher(Box::new(matcher), &[handler]);

let mut buffer = [0; 2048];
let mut input = "<input data>".as_bytes();
while let Ok(size) = input.read(&mut buffer[..]) {
	if !size > 0 {
		break;
	}
	trigger.process(&buffer[..size]);
}
```

### Filter
```rust
use streamson_lib::{strategy, error::General, matcher::Simple};
use std::io::prelude::*;

let mut filter = strategy::Filter::new();
let matcher = Simple::new(r#"{"users"}[]"#).unwrap();
filter.add_matcher(Box::new(matcher));

let mut buffer = [0; 2048];
let mut input = "<input data>".as_bytes();
while let Ok(size) = input.read(&mut buffer[..]) {
	if !size > 0 {
		break;
	}
	let output_data = filter.process(&buffer[..size]);
}
```

### Extract
```rust
use streamson_lib::{strategy, error::General, matcher::Simple};
use std::io::prelude::*;

let mut extract = strategy::Extract::new();
let matcher = Simple::new(r#"{"users"}[]"#).unwrap();
extract.add_matcher(Box::new(matcher));

let mut buffer = [0; 2048];
let mut input = "<input data>".as_bytes();
while let Ok(size) = input.read(&mut buffer[..]) {
	if !size > 0 {
		break;
	}
	let output_data = extract.process(&buffer[..size]);
}
```

### Convert
```rust
use streamson_lib::{strategy, matcher, handler};
use std::sync::{Arc, Mutex};
use std::io::prelude::*;

let mut convert = strategy::Convert::new();
let matcher = matcher::Simple::new(r#"{"list"}[]"#).unwrap();

convert.add_matcher(
	Box::new(matcher),
	vec![Arc::new(Mutex::new(handler::Unstringify::new()))]
);

let mut buffer = [0; 2048];
let mut input = "<input data>".as_bytes();
while let Ok(size) = input.read(&mut buffer[..]) {
	if !size > 0 {
		break;
	}
	let output_data = convert.process(&buffer[..size]);
}
```


## Traits
### Custom Handlers
You can define your custom handler.
```rust
use streamson_lib::{handler, Path, error};

#[derive(Debug)]
struct CustomHandler;

impl handler::Handler for CustomHandler {
	fn handle(&mut self, _: &Path, _: usize, _: Option<&[u8]>) -> Result<Option<std::vec::Vec<u8>>, error::Handler> { todo!() }
}

```

### Custom Matchers
You can define custom matchers as well.
```rust
use streamson_lib::matcher;

#[derive(Debug)]
struct CustomMatcher;

impl matcher::MatchMaker for CustomMatcher {
	fn match_path(&self, _: &streamson_lib::Path) -> bool { todo!() }
}
```
