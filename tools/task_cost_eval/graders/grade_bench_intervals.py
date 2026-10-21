#!/usr/bin/env python3
import copy
import math
import sys
from _par_common import attempt, finish, fixture_dir, import_fresh


def main(argv):
    failures = []
    try:
        intervals, = import_fresh(fixture_dir(argv, 'bench_intervals'), 'intervals')
    except Exception as error:
        return finish(['import: ' + repr(error)])

    source = [[5, 7], (1, 3), [3, 5], (2, 4), (2, 4), (10, 12)]
    saved = copy.deepcopy(source)
    attempt(failures, 'merge unsorted', lambda: intervals.merge(source), [(1, 7), (10, 12)])
    attempt(failures, 'merge generator', lambda: intervals.merge(iter(source)), [(1, 7), (10, 12)])
    attempt(failures, 'merge empty', lambda: intervals.merge([]), [])
    attempt(failures, 'merge touching', lambda: intervals.merge([(3, 4), (1, 2), (2, 3)]), [(1, 4)])
    attempt(failures, 'merge nested', lambda: intervals.merge([(1, 10), (2, 3), (4, 5)]), [(1, 10)])
    attempt(failures, 'merge floats', lambda: intervals.merge([(1.5, 2.5), (2.5, 3.0)]), [(1.5, 3.0)])
    attempt(failures, 'merge detached', lambda: (lambda result: (result.append((9, 10)), source)[1])(intervals.merge(source)), saved)
    attempt(failures, 'merge input unchanged', lambda: source, saved)

    attempt(failures, 'covers start', lambda: intervals.covers([(1, 3)], 1), True)
    attempt(failures, 'covers end excluded', lambda: intervals.covers([(1, 3)], 3), False)
    attempt(failures, 'covers middle', lambda: intervals.covers([(1, 3)], 2), True)
    attempt(failures, 'covers outside', lambda: intervals.covers([(1, 3)], 0), False)
    attempt(failures, 'covers empty', lambda: intervals.covers([], 0), False)
    attempt(failures, 'covers union touching', lambda: intervals.covers([(1, 2), (2, 3)], 2), True)
    attempt(failures, 'covers negative', lambda: intervals.covers([(-2.5, -1.5)], -2), True)
    attempt(failures, 'covers bool point', lambda: intervals.covers([(1, 2)], True), raises=ValueError)
    attempt(failures, 'covers nan point', lambda: intervals.covers([(1, 2)], math.nan), raises=ValueError)

    attempt(failures, 'length overlap', lambda: intervals.total_length([(1, 3), (2, 5)]), 4)
    attempt(failures, 'length empty', lambda: intervals.total_length([]), 0)
    attempt(failures, 'length touching', lambda: intervals.total_length([(1, 2), (2, 3)]), 2)
    attempt(failures, 'length floats', lambda: intervals.total_length([(1.5, 2.0), (2.0, 4.5)]), 3.0)
    attempt(failures, 'length duplicates', lambda: intervals.total_length([(1, 4), (1, 4), (1, 4)]), 3)

    for invalid in ([(1, 1)], [(2, 1)], [(1, 2, 3)], [(1,)], [1], ['1', 2], [(True, 2)], [(1, False)],
                    [(math.nan, 2)], [(1, math.inf)], [(1, -math.inf)], [(1 + 1j, 2)], [(None, 2)]):
        attempt(failures, 'reject merge ' + repr(invalid), lambda value=invalid: intervals.merge(value), raises=ValueError)
    for invalid in (None, 1, 'abc', 3.5):
        attempt(failures, 'reject container ' + repr(invalid), lambda value=invalid: intervals.merge(value), raises=ValueError)

    return finish(failures)


if __name__ == '__main__':
    raise SystemExit(main(sys.argv))
