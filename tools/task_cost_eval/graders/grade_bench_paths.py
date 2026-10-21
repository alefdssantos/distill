#!/usr/bin/env python3
import sys
from _par_common import attempt, finish, fixture_dir, import_fresh


def main(argv):
    failures = []
    try:
        paths, = import_fresh(fixture_dir(argv, 'bench_paths'), 'paths')
    except Exception as error:
        return finish(['import: ' + repr(error)])
    attempt(failures, 'collapse repeated slashes', lambda: paths.normalize_path('/a//b///c'), '/a/b/c')
    attempt(failures, 'drop dot segments', lambda: paths.normalize_path('/a/./b/./c'), '/a/b/c')
    attempt(failures, 'resolve parent', lambda: paths.normalize_path('/a/b/../c'), '/a/c')
    attempt(failures, 'parent cannot escape root', lambda: paths.normalize_path('/../../a'), '/a')
    attempt(failures, 'root escape stays root', lambda: paths.normalize_path('/../..'), '/')
    attempt(failures, 'trailing slash preserved', lambda: paths.normalize_path('/a/b/'), '/a/b/')
    attempt(failures, 'trailing slash after dot', lambda: paths.normalize_path('/a/./'), '/a/')
    attempt(failures, 'trailing slash after parent', lambda: paths.normalize_path('/a/b/../'), '/a/')
    attempt(failures, 'parent without trailing slash', lambda: paths.normalize_path('/a/b/..'), '/a')
    attempt(failures, 'root', lambda: paths.normalize_path('/'), '/')
    attempt(failures, 'double slash root', lambda: paths.normalize_path('//'), '/')
    attempt(failures, 'multiple trailing slashes', lambda: paths.normalize_path('/a//'), '/a/')
    attempt(failures, 'unicode segments preserved', lambda: paths.normalize_path('/é/雪/../ß/'),
            '/é/ß/')
    attempt(failures, 'literal triple dot', lambda: paths.normalize_path('/a/.../b'), '/a/.../b')
    attempt(failures, 'dotfile preserved', lambda: paths.normalize_path('/a/.hidden'), '/a/.hidden')
    attempt(failures, 'spaces preserved', lambda: paths.normalize_path('/a b/c'), '/a b/c')
    attempt(failures, 'idempotent', lambda: paths.normalize_path(paths.normalize_path('/x//y/../z/')),
            '/x/z/')
    attempt(failures, 'empty path', lambda: paths.normalize_path(''), raises=ValueError)
    attempt(failures, 'NUL byte', lambda: paths.normalize_path('/a\x00b'), raises=ValueError)
    attempt(failures, 'non-string path', lambda: paths.normalize_path(b'/a'), raises=ValueError)
    attempt(failures, 'relative path', lambda: paths.normalize_path('a/b'), raises=ValueError)
    attempt(failures, 'relative dot path', lambda: paths.normalize_path('./a'), raises=ValueError)
    return finish(failures)


if __name__ == '__main__':
    raise SystemExit(main(sys.argv))
