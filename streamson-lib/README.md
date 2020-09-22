[![docs.rs](https://docs.rs/streamson-lib/badge.svg)](https://docs.rs/streamson-lib)

# Streamson Lib

Rust library to handle large JSONs.

Note that it doesn't fully validates whether the input JSON is valid.
This means that invalid JSONs might pass without an error.

## Trigger strategy

It doesn't actually perform parses json into data. It just splits JSONs. And triggers handlers on matched paths.


## Filter strategy

It actually alters the JSON. If the path is matched the matched part should be removed from output json.


## Extract strategy

Only extracts matched data, nothing else.


## Examples
### Trigger
```rust
use streamson_lib::{strategy, GenericError, PrintLn, Simple};

let mut trigger = strategy::Trigger::new();
let handler = Arc::new(Mutex::new(PrintLn::new());
let matcher = Simple(r#"{"users"}[]"#).unwrap();
trigger.add_matcher(Box::new(matcher), &[handler]);

let mut buffer = [0; 2048];
while let Ok(size) = input.read(&mut buffer[..]) {
	let (output_data, continue) = filter.process(&buffer[..size])?;
}
```

### Filter
```rust
use streamson_lib::{strategy, error::GenericError, matcher::Simple};

let mut filter = strategy::Filter::new();
let matcher = Simple(r#"{"users"}[]"#).unwrap();
filter.add_matcher(Box::new(matcher), &[handler]);

let mut buffer = [0; 2048];
while let Ok(size) = input.read(&mut buffer[..]) {
	let (output_data, continue) = filter.process(&buffer[..size])?;
}
```

### Extract
```rust
use streamson_lib::{strategy, error::GenericError, matcher::Simple};

let mut extract = strategy::Extract::new();
let matcher = Simple(r#"{"users"}[]"#).unwrap();
extract.add_matcher(Box::new(matcher), &[handler]);

let mut buffer = [0; 2048];
while let Ok(size) = input.read(&mut buffer[..]) {
	let (output_data, continue) = extract.process(&buffer[..size])?;
}
```


## Traits
### Custom Handlers
You can define your custom handler.
```rust
use streamson_lib::handler;

...

struct CustomHandler;

impl handler::Handler for CustomHandler {
	...
}

```

### Custom Matchers
You can define custom matchers as well.
```rust
use streamson_lib::matcher;

struct CustomMatcher;

impl matcher::MatchMaker for CustomMatcher {
	...
}
```
