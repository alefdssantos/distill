Implement proportional integer allocation in `money.py` inside this slice, using only the Python standard library and Python 3.11+. Keep the existing function signature.

`allocate(cents, weights)` returns a new list of non-negative integers whose length equals the number of weights and whose sum is exactly `cents`. `cents` must be a non-negative `int`; reject `bool`, floats, strings and negative values with `ValueError`. `weights` must be a non-empty iterable of non-negative `int` values; reject `bool`, floats, strings, negative values, an empty iterable, a non-iterable object and a weight list whose total is zero with `ValueError`. Zero individual weights are allowed as long as at least one weight is positive. Accept generators and do not mutate the input.

Use largest-remainder allocation with exact integer arithmetic. For each index compute `product = cents * weight`, `base = product // total`, and `remainder = product % total`. Start each share at `base`, then distribute the leftover `cents - sum(base)` units one at a time to weights with the largest remainder. Break remainder ties by lower original index, so ordering is fully deterministic and independent of dictionary or set iteration order. Never use floating point for the division. A zero weight always receives zero. Every result element is an `int` and the result list is detached from the input.

Examples: `allocate(100, [1, 1])` is `[50, 50]`; `allocate(100, [1, 1, 1])` is `[34, 33, 33]`; `allocate(1000, [5, 3, 2])` is `[500, 300, 200]`; `allocate(5, [1, 1, 1])` is `[2, 2, 1]`.

The function is pure, deterministic and case-unrelated because only integers are accepted. Run `PYTHONDONTWRITEBYTECODE=1 python -m unittest discover -v` from this slice.
