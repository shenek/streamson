# Streamson Lib

Rust library to split large JSONs.

It doesn't actually perform the parsing. It just splits JSONs. And triggers handlers on matched paths.o

Note that it doesn't fully validates whether the JSON is valid.
This means that invalid JSONs might pass without an error.


## Examples
### Simple
```rust
use streamson_lib::{Collector, GenericError, PrintLn, Simple};

let mut collector = Collector::new();
let handler = Arc::new(Mutex::new(PrintLn::new());
let matcher = Simple(r#"{"users"}[]"#);
collector = collector.add_matcher(Box::new(matcher), &[handler]);

let mut buffer = [0; 2048];
while let Ok(size) = input.read(&mut buffer[..]) {
	collector.process(&buffer[..size]);
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
