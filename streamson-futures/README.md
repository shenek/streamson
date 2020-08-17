[![docs.rs](https://docs.rs/streamson-futures/badge.svg)](https://docs.rs/streamson-futures)

# Streamson futures

A library which integrates streamson with futures.
It enables to use streamson with async runs

## Examples
### Wrapping a stream
```rust
use bytes::Bytes;
use futures::stream::{self, StreamExt};
use streamson_lib::matcher;
use streamson_futures::stream::CollectorStream;

let stream = stream::iter(
    vec![r#"{"users": ["#, r#"{"name": "carl", "id": 1}"#, r#"]}"#]
        .drain(..)
        .map(Bytes::from)
        .collect::<Vec<Bytes>>()
);
let matcher = matcher::Simple::new(r#"{"users"}[]{"name"}"#).unwrap();
let wrapped_stream = CollectorStream::new(stream, Box::new(matcher));
```
