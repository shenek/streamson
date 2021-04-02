[![docs.rs](https://docs.rs/streamson-lib/badge.svg)](https://docs.rs/streamson-lib)

# Streamson Lib

Rust library to handle large JSONs. It aims to be memory efficient as well as fast.

Note that it doesn't fully validates whether the input JSON is valid.
This means that invalid JSONs might pass without an error.

## Strategies

| Strategy | Converts data | Buffers matched data | Nested matches | Uses handlers | Uses matchers |
| -------- | ------------- | -------------------- | -------------- | ------------- | ------------- |
| Trigger  | No            | No                   | Yes            | Yes           | Yes           |
| Filter   | Yes           | No                   | No             | Yes           | Yes           |
| Extract  | Yes           | No                   | No             | Yes           | Yes           |
| Convert  | Yes           | No                   | No             | Yes           | Yes           |
| All      | Yes/No        | No                   | No             | Yes           | No            |


### Trigger strategy

It triggers handlers on matched JSON parts. It doesn't return data as output.


### Filter strategy

It actually alters the JSON. If the path is matched the matched part should be removed from output JSON.
Handlers can be used here to e.g. store removed parts into a file.


### Extract strategy

Alters the JSON as well. It returns only the matched parts as output.
Handlers can be used to e.g. convert extracted parts.


### Convert strategy

Alters the JSON by calling convert handlers to matched parts.


### All strategy

Matches all data. Handlers can be used to convert the content of entire JSON or to perform
some kind of analysis.


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


### Regex
Matches path based on regex.

#### Example
```rust
use streamson_lib::{handler, strategy::{self, Strategy}, matcher};

use std::{io, str::FromStr, sync::{Arc, Mutex}};

let handler = Arc::new(Mutex::new(handler::Output::new(io::stdout())));
let matcher = matcher::Regex::from_str(r#"\{"[Uu]ser"\}\[\]"#).unwrap();

let mut trigger = strategy::Trigger::new();

trigger.add_matcher(
    Box::new(matcher),
    handler,
);

for input in vec![
    br#"{"Users": [1,2]"#.to_vec(),
    br#", "users": [3, 4]}"#.to_vec(),
] {
    trigger.process(&input).unwrap();
}
```


### Combinator

Wraps one or two matchers. It implements basic logic operators (`NOT`, `OR` and `AND`).

## Handlers

### Analyser
Stores matched paths to analyze JSON structure.

### Buffer
Buffers matched data which can be manually extracted later.

### Output
Writes matched data into given output (e.g. file or stdout).

### Indenter
Converts indentation of the matched data.

### Indexer
Store indexes of the matched data.

### Regex
Converts data based on regex.

#### Example
```rust
use streamson_lib::{matcher, strategy::{self, Strategy}, handler};
use std::sync::{Arc, Mutex};
use regex;

let converter =
Arc::new(Mutex::new(handler::Regex::new().add_regex("s/User/user/".to_string())));
let matcher = matcher::Simple::new(r#"{"users"}[]{"name"}"#).unwrap();
let mut convert = strategy::Convert::new();
// Set the matcher for convert strategy
convert.add_matcher(Box::new(matcher), converter);
for input in vec![
    br#"{"users": [{"password": "1234", "name": "User1"}, {"#.to_vec(),
    br#""password": "0000", "name": "user2}]}"#.to_vec(),
] {
    for converted_data in convert.process(&input).unwrap() {
        println!("{:?}", converted_data);
    }
}
```

### Replace
Replaces matched output by fixed data.

### Shorten
Shortens matched data

### Unstringify
Unstringifies matched data.

## Examples
### Trigger
```rust
use streamson_lib::{strategy::{self, Strategy}, error::General, handler::Output, matcher::Simple};
use std::sync::{Arc, Mutex};
use std::{io::prelude::*, io};

let mut trigger = strategy::Trigger::new();
let handler = Arc::new(Mutex::new(Output::new(io::stdout())));
let matcher = Simple::new(r#"{"users"}[]"#).unwrap();
trigger.add_matcher(Box::new(matcher), handler);

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
use streamson_lib::{strategy::{self, Strategy}, error::General, matcher::Simple};
use std::io::prelude::*;

let mut filter = strategy::Filter::new();
let matcher = Simple::new(r#"{"users"}[]"#).unwrap();
filter.add_matcher(Box::new(matcher), None);

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
use streamson_lib::{strategy::{self, Strategy}, error::General, matcher::Simple};
use std::io::prelude::*;

let mut extract = strategy::Extract::new();
let matcher = Simple::new(r#"{"users"}[]"#).unwrap();
extract.add_matcher(Box::new(matcher), None);

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
use streamson_lib::{strategy::{self, Strategy}, matcher, handler};
use std::sync::{Arc, Mutex};
use std::io::prelude::*;

let mut convert = strategy::Convert::new();
let matcher = matcher::Simple::new(r#"{"list"}[]"#).unwrap();

convert.add_matcher(
	Box::new(matcher),
	Arc::new(Mutex::new(handler::Unstringify::new())),
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

### All
```rust
use streamson_lib::{strategy::{self, Strategy}, matcher, handler};
use std::sync::{Arc, Mutex};
use std::io::prelude::*;

let mut all = strategy::All::new();

let analyser = Arc::new(Mutex::new(handler::Analyser::new()));

all.add_handler(analyser.clone());

let mut buffer = [0; 2048];
let mut input = "<input data>".as_bytes();
while let Ok(size) = input.read(&mut buffer[..]) {
	if !size > 0 {
		break;
	}
	all.process(&buffer[..size]);
}

println!("{:?}", analyser.lock().unwrap().results())
```


## Traits
### Custom Handlers
You can define your custom handler.
```rust
use std::any::Any;

use streamson_lib::{handler, Path, error, streamer::Token};

#[derive(Debug)]
struct CustomHandler;

impl handler::Handler for CustomHandler {
	fn start(
		&mut self, _: &Path, _: usize, _: Token 
	) -> Result<Option<std::vec::Vec<u8>>, error::Handler> { 
		todo!()
	}

	fn feed(
		&mut self, _: &[u8], _: usize,
	) -> Result<Option<std::vec::Vec<u8>>, error::Handler> { 
		todo!()
	}

	fn end(
		&mut self, _: &Path, _: usize, _: Token
	) -> Result<Option<std::vec::Vec<u8>>, error::Handler> { 
		todo!()
	}
	
	fn as_any(&self) -> &dyn Any {
		self
	}

	fn is_converter(&self) -> bool {
		todo!()
	}
}

```

### Custom Matchers
You can define custom matchers as well.
```rust
use streamson_lib::{matcher, Path, streamer::ParsedKind};

#[derive(Debug)]
struct CustomMatcher;

impl matcher::Matcher for CustomMatcher {
	fn match_path(&self, _: &Path, _: ParsedKind) -> bool { todo!() }
}
```
