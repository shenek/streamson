7.1.0 (2020-10-05)
-----------------

* fix memory inefficiencies in streamson-bin
* a new documentation site added (hosted on netlify)
  https://streamson.henek.name
* added a code coverage info (codecov.io)
* `+` operator for Group handler
* manpages for sson


7.0.1 (2020-04-15)
------------------

* fix building release in Github Actions
* fix buffer handler in nested mode
* export path only for extract strategy
* less strict streamson-bin-test stderr checks

7.0.0 (2020-04-03)
------------------

* Remove all matcher (there is all strategy now)
* Limit handlers for strategies in streamson-bin
* Unite PrintLn and File handlers into Output handler
* Use nicer name of main binary `streamson-bin` -> `sson`
* Added terminate function for strategies (can be used to flush handlers)
* Unite handler args for streamson-bin
* Unite matcher args for streamson-bin
* New indenter handler (reformats JSON to be more/less readable)
* New `All` strategy (matches everything, handlers only)
* Data splitting functions for tests
* Handlers can be used for extract strategy
* Handlers can be used for filter strategy
* Added handler group (way to group multiple handlers together)
* Huge refactoring of handlers so they are used in streaming mode now
* Export matched kind for matcher
* Export matched kind for handler
* Merged streamson-extra-matchers with streamson-lib (regex feature)

6.3.1 (2020-12-08)
------------------

* Fix wrong Error types in streamson-extra-matchers

6.3.0 (2020-12-05)
------------------

* Buffer handler can limit its buffer size
* CI tests for macos and windows
* added regex matcher for streamson-bin
* added tests for streamson-bin README.md
* added unstringify handler for convert strategy
* added regex converter to streamson-extra-matchers
* added '--before' and '--after' to streamson-bin extract subcommand

6.2.0 (2020-11-12)
------------------

* docs updates
* added matcher which match all paths
* added analyser handler
* streamson-bin displays JSON structure in trigger strategy
* added shorten handler for convert strategy
* added shell completions for streamson-bin
* added separator to streamson-bin extract subcommand
* added convert benchmark
* benchmarks are not triggered in github actions anymore
* wildcards '?' and '*' can be used inside simple matcher

6.1.0 (2020-10-23)
------------------

* added test for streamson-bin
* fix streamson-bin and insufficient buffer size

6.0.0 (2020-10-12)
------------------

* alter handlers and use it in convert strategy

5.0.2 (2020-10-11)
------------------

* fix convert strategy so it can be safely passed between the threads (second try)

5.0.1 (2020-10-10)
------------------

* fix convert strategy so it can be safely passed between the threads

5.0.0 (2020-10-05)
------------------

* modify streamson-bin to use strategies as subcommands
* export matcher_idx to handlers
* convert strategy added
* extract strategy added
* filter strategy added
* rename existing strategy to trigger
* refactoring to strategies module
* new indexer handler added

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
