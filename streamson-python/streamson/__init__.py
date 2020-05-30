import json
import typing

from array import array

from streamson.streamson import SimpleStreamson as _SimpleStreamson


def extract_iter(
    input_gen: typing.Generator[bytes, None, None],
    simple_matches: typing.List[str],
):
    streamson = _SimpleStreamson(simple_matches)
    for item in input_gen:
        streamson.feed(item)
        res = streamson.pop()
        while res is not None:
            path, data = res
            yield path, json.loads(array('B', data).tobytes())
            res = streamson.pop()
