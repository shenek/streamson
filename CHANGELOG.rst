4.1.0 (2020-08-29)
------------------
* new streamson-generato subcrate (unstable)
* new streamson-futures subcrate
* new streamson-extra-matchers subcrate
* remove Root path element
* rename some classes
* add_matcher no longer uses builder pattern
* parse depth matcher from str
* various performance improvements
* extend benchmarks
* make path optional in Buffer handler

4.0.0 (2020-07-29)
------------------

* matcher refactoring
* other speed optimizations

3.0.0 (2020-07-21)
------------------

* fix benchmarks
* add Depth/Buffer and Combinator/Buffer benchmarks
* depth matcher optimizations
* use VecDeque or Vec instead of Bytes and BytesMut (speed optimizations)
* remove Bytes dependecy

2.0.0 (2020-07-13)
------------------

* speed optimizations
* stops to check whether data are in utf8

1.0.2 (2020-07-02)
------------------

* make matchers clonable

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
