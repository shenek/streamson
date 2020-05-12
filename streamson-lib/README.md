# Streamson Lib

Rust library to split large JSONs.


## Examples
### Simple
```rust
use streamson_lib::{Collector, GenericError, PrintLn, Simple};

let mut collector = Collector::new();
let handler = Arc::new(Mutex::new(PrintLn));
...

let mut buffer = [0; 2048];
while let Ok(size) = input.read(&mut buffer[..]) {
	collector.process(&buffer[..size]);
}
```


## Traits
TBD
