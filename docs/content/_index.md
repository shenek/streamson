+++
title = "Streamson"
sort_by = "weight"
+++


# What it is
Streamson is a tool to process large JSON inputs (the input can be a stream of JSONs as well).
It tries to be memory efficient and it assumes that entire JSON won't fit into memory.


# What it is not
Streamson is not a JSON parser which tries to convert data to some kind of internal representation.
It simly expects UTF-8 encoded input and it is able to convert it to another UTF-8 encoded output.
Note that the output doesn't really need to be a valid JSON.

# Motivation

![Intentation screencast](screencast.gif)
