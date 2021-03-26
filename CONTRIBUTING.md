# How to contribute to streamson

PATCHES ARE WELCOMED!

To contribute code you need to be sure that it is tested and all other tests are passing.
Also Please try to keep commit message format.

`streamson` packages should be published on [crates.io](https://crates.io).
`streamson-bin` (a.k.a. `sson`) can be installed using
```shell
cargo install . --bin sson --path streamson-bin/`.
```


## Running tests

Make sure that you have latest stable rust installed.
```shel
rustup update
```

To run tests you can simply use cargo
```shell
cargo test --all-features
```

To run tests of streamson binary `sson` you need to install `streamson-bin-test` first.
```shell
cargo install --bin streamson-bin-test --path streamson-bin/
```

Then you can test it.
```shell
streamson-bin-test sson
```

## Commit message format

Preferred way how the commit messages should look like is like this:
```
<kind>[(<component(s))>]: <short description>

[<long description>]
```
Examples:
```
feature(bin): Added new Awesome Handler

Details of Awesome Handler
```

```
fix(lib): Fixing error when input stream is empty
```
