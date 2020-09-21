[![docs.rs](https://docs.rs/streamson-extra-matchers/badge.svg)](https://docs.rs/streamson-extra-matchers)

# Streamson extra matcher

A library which contains extra matchers for streamson-lib.
Note that this library may contain additional dependencies.

## Matchers

### Regex
Matches path based on regex.

#### Example
```rust
use streamson_lib::{handler, strategy};
use streamson_extra_matchers::Regex;

use std::{str::FromStr, sync::{Arc, Mutex}};

let handler = Arc::new(Mutex::new(handler::PrintLn::new()));
let matcher = Regex::from_str(r#"\{"[Uu]ser"\}\[\]"#).unwrap();

let mut trigger = strategy::Trigger::new();

trigger.add_matcher(
    Box::new(matcher),
    &[handler],
);

for input in vec![
    br#"{"Users": [1,2]"#.to_vec(),
    br#", "users": [3, 4]}"#.to_vec(),
] {
    trigger.process(&input).unwrap();
}
```
