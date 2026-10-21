Implement closed-open numeric interval utilities in `intervals.py` inside this slice, using only the Python standard library and Python 3.11+. Keep the existing function signatures.

An interval is a `tuple` or `list` of exactly two numbers `(start, end)` where each endpoint is an `int` or `float`, both are finite, and `start < end`. The interval represents `[start, end)`: `start` is inside and `end` is outside. Reject `bool` endpoints even though `bool` subclasses `int`, reject `NaN`, infinities, strings, complex numbers, malformed pairs and non-iterable containers with `ValueError`. Empty input is valid.

`merge(intervals)` accepts any iterable of intervals, including a generator, and returns a new sorted list of tuples. Sort by `start` ascending then `end` ascending. Merge two intervals when the next `start` is less than or equal to the current merged `end`, so touching intervals such as `(1, 2)` and `(2, 3)` become `(1, 3)`; nested intervals keep the larger end. Duplicate intervals collapse. Preserve the numeric values from the input. Never mutate the input or return references to its inner tuples.

`covers(intervals, point)` returns `True` when `point` lies in `[start, end)` of any interval and `False` otherwise. `point` must be a finite non-`bool` `int` or `float`; validate it with `ValueError`. Intervals are treated as a union, so overlapping and touching intervals behave as their merged form. Empty input returns `False`.

`total_length(intervals)` returns the total length of the union, summing `end - start` after merging. Empty input returns integer `0`. It returns an `int` when every participating computation uses integers and a `float` when any endpoint is a float. Never mutate inputs.

Ordering is deterministic and explicit; do not depend on dictionary or set iteration order. All functions are pure. Unicode is irrelevant because only numeric input is accepted. Run `PYTHONDONTWRITEBYTECODE=1 python -m unittest discover -v` from this slice.
