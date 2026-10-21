#!/usr/bin/env python3
import sys
from _par_common import attempt, finish, fixture_dir, import_fresh


def main(argv):
    failures = []
    try:
        semver, = import_fresh(fixture_dir(argv, 'bench_semver'), 'semver')
    except Exception as error:
        return finish(['import: ' + repr(error)])

    attempt(failures, 'parse plain', lambda: semver.parse('1.2.3'), (1, 2, 3, (), ()))
    attempt(failures, 'parse prerelease', lambda: semver.parse('1.2.3-alpha.1.0A'), (1, 2, 3, ('alpha', 1, '0A'), ()))
    attempt(failures, 'parse build', lambda: semver.parse('1.2.3+build.001'), (1, 2, 3, (), ('build', '001')))
    attempt(failures, 'parse both', lambda: semver.parse('10.20.30-rc.2+exp.sha.5114f85'), (10, 20, 30, ('rc', 2), ('exp', 'sha', '5114f85')))
    attempt(failures, 'parse zeros', lambda: semver.parse('0.0.0-0'), (0, 0, 0, (0,), ()))
    attempt(failures, 'parse hyphen identifier', lambda: semver.parse('1.0.0-x-y-z.--'), (1, 0, 0, ('x-y-z', '--'), ()))

    for invalid in ('', '1', '1.2', '1.2.3.4', '01.2.3', '1.02.3', '1.2.03', '1.2.3-', '1.2.3+',
                    '1.2.3-a..b', '1.2.3-01', '1.2.3-a_b', '1.2.3+a..b', ' 1.2.3', '1.2.3 ',
                    'v1.2.3', '1.2.3-α', '1.2.3-a+b+c', '+1.2.3', '-1.2.3'):
        attempt(failures, 'reject syntax ' + repr(invalid), lambda value=invalid: semver.parse(value), raises=ValueError)
    for invalid in (None, 123, 1.2, b'1.2.3', ['1.2.3'], True):
        attempt(failures, 'reject parse type ' + type(invalid).__name__, lambda value=invalid: semver.parse(value), raises=ValueError)

    attempt(failures, 'compare equal', lambda: semver.compare('1.2.3', '1.2.3'), 0)
    attempt(failures, 'compare build ignored', lambda: semver.compare('1.2.3+one', '1.2.3+two'), 0)
    attempt(failures, 'compare core order', lambda: semver.compare('1.10.0', '1.9.9'), 1)
    attempt(failures, 'compare release over prerelease', lambda: semver.compare('1.0.0', '1.0.0-rc.1'), 1)
    attempt(failures, 'compare numeric lower than alpha', lambda: semver.compare('1.0.0-1', '1.0.0-alpha'), -1)
    attempt(failures, 'compare numeric ordering', lambda: semver.compare('1.0.0-2', '1.0.0-10'), -1)
    attempt(failures, 'compare alpha ascii', lambda: semver.compare('1.0.0-alpha', '1.0.0-beta'), -1)
    attempt(failures, 'compare shorter prerelease', lambda: semver.compare('1.0.0-alpha', '1.0.0-alpha.1'), -1)
    attempt(failures, 'compare long chain', lambda: semver.compare('1.0.0-alpha.beta', '1.0.0-alpha.1'), 1)
    attempt(failures, 'compare invalid left', lambda: semver.compare('1.0', '1.0.0'), raises=ValueError)
    attempt(failures, 'compare invalid right', lambda: semver.compare('1.0.0', None), raises=ValueError)

    return finish(failures)


if __name__ == '__main__':
    raise SystemExit(main(sys.argv))
