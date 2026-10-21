#!/usr/bin/env python3
import copy
import sys
from _par_common import attempt, finish, fixture_dir, import_fresh


def main(argv):
    failures = []
    try:
        csvparse, = import_fresh(fixture_dir(argv, 'bench_csv'), 'csvparse')
    except Exception as error:
        return finish(['import: ' + repr(error)])
    canonical = 'name,note\r\nAda,"hello, world"\r\nBob,"say ""hi"""\r\n'
    parsed = [
        {'name': 'Ada', 'note': 'hello, world'},
        {'name': 'Bob', 'note': 'say "hi"'},
    ]
    attempt(failures, 'canonical parse', lambda: csvparse.parse_csv(canonical), parsed)
    attempt(failures, 'embedded CRLF', lambda: csvparse.parse_csv('id,body\r\n1,"a\r\nb"\r\n'),
            [{'id': '1', 'body': 'a\r\nb'}])
    attempt(failures, 'empty text', lambda: csvparse.parse_csv(''), [])
    attempt(failures, 'trailing newline', lambda: csvparse.parse_csv('a\n1\n'), [{'a': '1'}])
    attempt(failures, 'blank lines skipped', lambda: csvparse.parse_csv('a\n\n1\n\n2\n'),
            [{'a': '1'}, {'a': '2'}])
    attempt(failures, 'malformed quoted field', lambda: csvparse.parse_csv('a\n"unterminated\n'),
            raises=ValueError)
    attempt(failures, 'header only', lambda: csvparse.parse_csv('a,b\n'), [])
    attempt(failures, 'field spaces preserved', lambda: csvparse.parse_csv('a,b\n x ," y "\n'),
            [{'a': ' x ', 'b': ' y '}])
    attempt(failures, 'duplicate header', lambda: csvparse.parse_csv('a,a\n1,2\n'), raises=ValueError)
    attempt(failures, 'empty header name', lambda: csvparse.parse_csv('a,\n1,2\n'), raises=ValueError)
    attempt(failures, 'row too many fields', lambda: csvparse.parse_csv('a,b\n1,2,3\n'), raises=ValueError)
    attempt(failures, 'row too few fields', lambda: csvparse.parse_csv('a,b\n1\n'), raises=ValueError)
    attempt(failures, 'non-string text', lambda: csvparse.parse_csv(b'a\n1\n'), raises=ValueError)
    rows = [
        {'id': '1', 'kind': 'a', 'n': 'x'},
        {'id': '2', 'kind': 'b', 'n': 'y'},
        {'id': '3', 'kind': 'A', 'n': 'x'},
    ]
    columns = ['id', 'kind', 'n']
    filters = {'kind': 'a'}
    saved_rows = copy.deepcopy(rows)
    saved_columns = list(columns)
    saved_filters = dict(filters)
    attempt(failures, 'canonical selection', lambda: csvparse.select_rows(rows, columns, filters),
            [{'id': '1', 'kind': 'a', 'n': 'x'}])
    attempt(failures, 'all rows empty filters', lambda: csvparse.select_rows(rows, ['id'], {}),
            [{'id': '1'}, {'id': '2'}, {'id': '3'}])
    attempt(failures, 'exact case-sensitive filter',
            lambda: csvparse.select_rows(rows, ['id'], {'kind': 'A'}), [{'id': '3'}])
    attempt(failures, 'filter unselected column',
            lambda: csvparse.select_rows(rows, ['id'], {'n': 'y'}), [{'id': '2'}])
    attempt(failures, 'no matching row', lambda: csvparse.select_rows(rows, ['id'], {'kind': 'z'}), [])
    attempt(failures, 'empty rows', lambda: csvparse.select_rows([], ['id'], {'kind': 'a'}), [])
    attempt(failures, 'generator rows',
            lambda: csvparse.select_rows(iter(rows), ['id'], {'kind': 'b'}), [{'id': '2'}])
    attempt(failures, 'duplicate columns', lambda: csvparse.select_rows(rows, ['id', 'id'], {}),
            raises=ValueError)
    attempt(failures, 'unknown selected column', lambda: csvparse.select_rows(rows, ['missing'], {}),
            raises=ValueError)
    attempt(failures, 'unknown filter column', lambda: csvparse.select_rows(rows, ['id'], {'missing': 'x'}),
            raises=ValueError)
    attempt(failures, 'invalid filters type', lambda: csvparse.select_rows(rows, ['id'], {'kind': None}),
            raises=ValueError)
    attempt(failures, 'invalid row entry', lambda: csvparse.select_rows([1], ['id'], {}), raises=ValueError)
    attempt(failures, 'non-iterable rows', lambda: csvparse.select_rows(42, ['id'], {}), raises=ValueError)

    def detached():
        result = csvparse.select_rows(rows, ['id'], {})
        result[0]['id'] = 'changed'
        result.append({'id': 'extra'})
        return rows

    attempt(failures, 'detached result', detached, saved_rows)
    attempt(failures, 'inputs unchanged', lambda: (rows, columns, filters),
            (saved_rows, saved_columns, saved_filters))
    return finish(failures)


if __name__ == '__main__':
    raise SystemExit(main(sys.argv))
