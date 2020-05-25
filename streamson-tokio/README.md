# Streamson tokio

A library which integrates streamson with tokio.
So that you can easily split jsons using asynchronous rust.

## Examples
### Reading a large file
```rust
 use std::io;
 use streamson_lib::error;
 use streamson_tokio::decoder::SimpleExtractor;
 use tokio::{fs, stream::StreamExt};
 use tokio_util::codec::FramedRead;

 let mut file = fs::File::open("/tmp/large.json").await?;
 let extractor = SimpleExtractor::new(vec![r#"{"users"}[]"#, r#"{"groups"}[]"#]);
 let mut output = FramedRead::new(file, extractor);
 while let Some(item) = output.next().await {
	 let (path, data) = item?;
	 // Do something with extracted data
 }
```
