1.0.1 (2020-06-28)
------------------

* make matchers and handlers to be safely send between the threads

1.0.0 (2020-06-25)
------------------

* added `Depth` matcher (matches paths with given depth)
* added `Combinator` matcher (combines matchers - `and`, `or`, `not`)
* refactor of streamson-tokio so it can use new matchers
* added examples with custom handler, custom matcher and serde

0.2.0 (2020-05-30)
------------------

* added `Buffer` handler
* added library which can be used with tokio
* added a simple benchmark
* fixes regarding `"` escapes
* improved error handling


0.1.0 (2020-05-19)
------------------

* added `Simple` matcher
* added `PrintLn` and `File` handlers
* added the main library
* added cmdline utility for processing large jsons
* initial version
