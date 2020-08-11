[![docs.rs](https://docs.rs/streamson-tokio/badge.svg)](https://docs.rs/streamson-tokio)

# Streamson tokio

A library which integrates streamson with tokio.
So that you can easily split jsons using asynchronous rust.

## Examples
### Reading a large file
```rust
 use std::io;
 use streamson_lib::{error, matcher};
 use streamson_tokio::decoder::Extractor;
 use tokio::{fs, stream::StreamExt};
 use tokio_util::codec::FramedRead;

 let mut file = fs::File::open("/tmp/large.json").await?;
 let matcher = matcher::Combinator::new(matcher::Simple::new(r#"{"users"}[]"#).unwrap())
     | matcher::Combinator::new(matcher::Simple::new(r#"{"groups"}[]"#).unwrap());
 let extractor = Extractor::new(matcher);
 let mut output = FramedRead::new(file, extractor);
 while let Some(item) = output.next().await {
     let (path, data) = item?;
     // Do something with extracted data
 }
```
