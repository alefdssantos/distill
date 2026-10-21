#!/usr/bin/env python3
import sys
from _par_common import attempt, finish, fixture_dir, import_fresh


def main(argv):
    failures = []
    try:
        money, = import_fresh(fixture_dir(argv, 'bench_money'), 'money')
    except Exception as error:
        return finish(['import: ' + repr(error)])

    attempt(failures, 'even split', lambda: money.allocate(100, [1, 1]), [50, 50])
    attempt(failures, 'equal triple tie', lambda: money.allocate(100, [1, 1, 1]), [34, 33, 33])
    attempt(failures, 'weighted exact', lambda: money.allocate(1000, [5, 3, 2]), [500, 300, 200])
    attempt(failures, 'small tie lower index', lambda: money.allocate(5, [1, 1, 1]), [2, 2, 1])
    attempt(failures, 'remainder priority', lambda: money.allocate(7, [2, 2, 1]), [3, 3, 1])
    attempt(failures, 'zero weight', lambda: money.allocate(1, [0, 0, 1]), [0, 0, 1])
    attempt(failures, 'zero cents', lambda: money.allocate(0, [0, 1]), [0, 0])
    attempt(failures, 'single weight', lambda: money.allocate(123, [9]), [123])
    attempt(failures, 'generator weights', lambda: money.allocate(11, iter([1, 2, 3, 4])), [1, 2, 3, 5])
    attempt(failures, 'mixed remainders', lambda: money.allocate(1000003, [3, 5, 7, 11]), [115385, 192308, 269232, 423078])
    attempt(failures, 'large integers', lambda: money.allocate(10 ** 18 + 7, [10 ** 9, 2 * 10 ** 9, 7 * 10 ** 9]), [100000000000000001, 200000000000000001, 700000000000000005])
    attempt(failures, 'zero in many', lambda: money.allocate(999, [7, 7, 7, 0]), [333, 333, 333, 0])
    attempt(failures, 'returns list', lambda: isinstance(money.allocate(3, [1, 1]), list), True)

    weights = [1, 2, 4]
    saved = list(weights)
    attempt(failures, 'does not mutate weights', lambda: (money.allocate(10, weights), weights)[1], saved)
    attempt(failures, 'detached result', lambda: (lambda result: (result.append(99), weights)[1])(money.allocate(10, weights)), saved)

    for invalid in (True, False, -1, 1.5, '1', None, [1], (1, 2)):
        attempt(failures, 'reject cents ' + repr(invalid), lambda value=invalid: money.allocate(value, [1]), raises=ValueError)
    for invalid in ([], [0, 0], [1, True], [1, False], [1, -1], [1, 1.5], [1.0], ['1'], [1, None], 5, None, 'abc'):
        attempt(failures, 'reject weights ' + repr(invalid), lambda value=invalid: money.allocate(10, value), raises=ValueError)

    return finish(failures)


if __name__ == '__main__':
    raise SystemExit(main(sys.argv))
