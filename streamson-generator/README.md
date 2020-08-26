[![docs.rs](https://docs.rs/streamson-generator/badge.svg)](https://docs.rs/streamson-generator)

# Streamson generator

A library which integrates streamson with rust generators.

## Examples
### Use file for input generator
```rust
let mut file = fs::File::open("/tmp/large.json")?;
let mut input_generator = move || {
	loop {
		let mut buffer = vec![0; 2048];
		if file.read(&mut buffer).unwrap() == 0 {
			break;
		}
		yield buffer;
	}
};

let matcher = Box::new(Simple::from_str(r#"{"users"}[]{"name"}"#).unwrap());
let mut output_generator = StreamsonGenerator::new(input_generator, matcher);

for item in output_generator {
	match item {
		Ok((path, data)) => {
			// Do something with the data
		},
		Err(err) => {
			// Deal with error situation
		}
	}
}
```
