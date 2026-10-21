#!/usr/bin/env python3
import sys
from _par_common import attempt, finish, fixture_dir, import_fresh


def main(argv):
    failures = []
    try:
        wrapping, = import_fresh(fixture_dir(argv, 'bench_wrap'), 'wrapping')
    except Exception as error:
        return finish(['import: ' + repr(error)])
    attempt(failures, 'greedy wrap', lambda: wrapping.wrap_text('the quick brown fox', 10),
            'the quick\nbrown fox')
    attempt(failures, 'exact width fit', lambda: wrapping.wrap_text('abc def', 7), 'abc def')
    attempt(failures, 'one over width', lambda: wrapping.wrap_text('abc def', 6), 'abc\ndef')
    attempt(failures, 'long word hard split', lambda: wrapping.wrap_text('abcdefgh', 3),
            'abc\ndef\ngh')
    attempt(failures, 'long word remainder joins', lambda: wrapping.wrap_text('abcdefg z', 3),
            'abc\ndef\ng z')
    attempt(failures, 'explicit newlines preserved', lambda: wrapping.wrap_text('one two\nthree', 7),
            'one two\nthree')
    attempt(failures, 'blank lines preserved', lambda: wrapping.wrap_text('a\n\nb', 10), 'a\n\nb')
    attempt(failures, 'whitespace-only line', lambda: wrapping.wrap_text('a\n   \nb', 5), 'a\n\nb')
    attempt(failures, 'trailing newline preserved', lambda: wrapping.wrap_text('a\n', 10), 'a\n')
    attempt(failures, 'empty text', lambda: wrapping.wrap_text('', 5), '')
    attempt(failures, 'collapse whitespace', lambda: wrapping.wrap_text('a\t\tb  c', 5), 'a b c')
    attempt(failures, 'width one', lambda: wrapping.wrap_text('ab cd', 1), 'a\nb\nc\nd')
    attempt(failures, 'unicode code points', lambda: wrapping.wrap_text('éé é', 4), 'éé é')
    attempt(failures, 'line lengths bounded',
            lambda: all(len(line) <= 4 for line in wrapping.wrap_text(
                'alpha beta gamma delta epsilon zeta', 4).split('\n')), True)
    attempt(failures, 'zero width', lambda: wrapping.wrap_text('a', 0), raises=ValueError)
    attempt(failures, 'negative width', lambda: wrapping.wrap_text('a', -1), raises=ValueError)
    attempt(failures, 'bool width', lambda: wrapping.wrap_text('a', True), raises=ValueError)
    attempt(failures, 'float width', lambda: wrapping.wrap_text('a', 2.5), raises=ValueError)
    attempt(failures, 'non-string text', lambda: wrapping.wrap_text(b'a', 2), raises=ValueError)
    attempt(failures, 'no trailing spaces',
            lambda: [line for line in wrapping.wrap_text('aa bb cc', 2).split('\n')
                     if line != line.rstrip()], [])
    return finish(failures)


if __name__ == '__main__':
    raise SystemExit(main(sys.argv))
