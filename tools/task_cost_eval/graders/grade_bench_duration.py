#!/usr/bin/env python3
import math
import sys
from _par_common import attempt, finish, fixture_dir, import_fresh


def main(argv):
    failures = []
    try:
        duration, = import_fresh(fixture_dir(argv, 'bench_duration'), 'duration')
    except Exception as error:
        return finish(['import: ' + repr(error)])

    attempt(failures, 'parse compound', lambda: duration.parse_duration('P1DT2H3M4S'), 93784)
    attempt(failures, 'parse weeks', lambda: duration.parse_duration('P2W'), 1209600)
    attempt(failures, 'parse fractional seconds', lambda: duration.parse_duration('PT0.5S'), 0.5)
    attempt(failures, 'parse fractional day', lambda: duration.parse_duration('P1.5D'), 129600)
    attempt(failures, 'parse fractional hours', lambda: duration.parse_duration('PT1.5H'), 5400)
    attempt(failures, 'parse leading zeros', lambda: duration.parse_duration('P01DT02H03M04S'), 93784)
    attempt(failures, 'parse zero day', lambda: duration.parse_duration('P0D'), 0)
    attempt(failures, 'parse zero time', lambda: duration.parse_duration('PT0S'), 0)

    for invalid in ('', 'P', 'PT', 'P1DT', 'P1D2H', 'P1Y', 'P1M', 'P1W2D', 'P1.5DT2H', 'PT1.5H30M',
                    'p1d', 'P1d', ' P1D', 'P1D ', 'P-1D', 'PT+1S', 'P1_D', 'P1,5D', 'P1e3D', 'PT1S1M',
                    'P1D1D', 'P1.S', 'PT.5S', 'P1DT2H3M4.5S6X'):
        attempt(failures, 'reject syntax ' + repr(invalid), lambda value=invalid: duration.parse_duration(value), raises=ValueError)
    for invalid in (None, 1, 1.5, b'P1D', ['P1D'], True):
        attempt(failures, 'reject parse type ' + type(invalid).__name__, lambda value=invalid: duration.parse_duration(value), raises=ValueError)

    attempt(failures, 'format compound', lambda: duration.format_duration(93784), 'P1DT2H3M4S')
    attempt(failures, 'format weeks as days', lambda: duration.format_duration(1209600), 'P14D')
    attempt(failures, 'format zero', lambda: duration.format_duration(0), 'PT0S')
    attempt(failures, 'format day only', lambda: duration.format_duration(86400), 'P1D')
    attempt(failures, 'format hours only', lambda: duration.format_duration(3600), 'PT1H')
    attempt(failures, 'format fraction', lambda: duration.format_duration(0.5), 'PT0.5S')
    attempt(failures, 'format microseconds', lambda: duration.format_duration(1.000001), 'PT1.000001S')
    attempt(failures, 'round trip fraction', lambda: duration.parse_duration(duration.format_duration(123.456789)), 123.456789)
    for invalid in (True, False, '1', None, -1, -0.5, math.inf, -math.inf, math.nan, 0.0000001):
        attempt(failures, 'reject format ' + repr(invalid), lambda value=invalid: duration.format_duration(value), raises=ValueError)

    return finish(failures)


if __name__ == '__main__':
    raise SystemExit(main(sys.argv))
