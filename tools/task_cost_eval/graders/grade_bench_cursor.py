#!/usr/bin/env python3
import copy
import sys
from _par_common import attempt, finish, fixture_dir, import_fresh


def main(argv):
    failures = []
    try:
        cursors, = import_fresh(fixture_dir(argv, 'bench_cursor'), 'cursors')
    except Exception as error:
        return finish(['import: ' + repr(error)])
    attempt(failures, 'canonical token', lambda: cursors.encode_cursor(1700000000, 'abc'),
            'WzE3MDAwMDAwMDAsImFiYyJd')
    attempt(failures, 'unicode token', lambda: cursors.encode_cursor(1, 'é雪'), 'WzEsIsOp6ZuqIl0')
    attempt(failures, 'empty id token', lambda: cursors.encode_cursor(5, ''), 'WzUsIiJd')
    attempt(failures, 'negative timestamp token', lambda: cursors.encode_cursor(-5, 'a'),
            'Wy01LCJhIl0')
    attempt(failures, 'no padding', lambda: '=' in cursors.encode_cursor(1700000000, 'abc'), False)
    attempt(failures, 'decode round trip', lambda: cursors.decode_cursor('WzE3MDAwMDAwMDAsImFiYyJd'),
            (1700000000, 'abc'))
    attempt(failures, 'decode unicode', lambda: cursors.decode_cursor('WzEsIsOp6ZuqIl0'), (1, 'é雪'))
    attempt(failures, 'decode negative and empty id', lambda: (
        cursors.decode_cursor('Wy01LCJhIl0'), cursors.decode_cursor('WzUsIiJd')),
        ((-5, 'a'), (5, '')))
    attempt(failures, 'reject padded token',
            lambda: cursors.decode_cursor('WzE3MDAwMDAwMDAsImFiYyJd='), raises=ValueError)
    attempt(failures, 'reject invalid alphabet', lambda: cursors.decode_cursor('!!!!'), raises=ValueError)
    attempt(failures, 'reject invalid length', lambda: cursors.decode_cursor('A'), raises=ValueError)
    attempt(failures, 'reject empty token', lambda: cursors.decode_cursor(''), raises=ValueError)
    attempt(failures, 'reject non-string token', lambda: cursors.decode_cursor(b'abc'), raises=ValueError)
    attempt(failures, 'reject object payload', lambda: cursors.decode_cursor('eyJhIjoxfQ'),
            raises=ValueError)
    attempt(failures, 'reject short array', lambda: cursors.decode_cursor('WzFd'), raises=ValueError)
    attempt(failures, 'reject long array', lambda: cursors.decode_cursor('WzEsImEiLDJd'),
            raises=ValueError)
    attempt(failures, 'reject string timestamp', lambda: cursors.decode_cursor('WyIxIiwiYSJd'),
            raises=ValueError)
    attempt(failures, 'reject bool timestamp', lambda: cursors.decode_cursor('W3RydWUsImEiXQ'),
            raises=ValueError)
    attempt(failures, 'reject float timestamp', lambda: cursors.decode_cursor('WzEuNSwiYSJd'),
            raises=ValueError)
    attempt(failures, 'reject null timestamp', lambda: cursors.decode_cursor('W251bGwsImEiXQ'),
            raises=ValueError)
    attempt(failures, 'reject non-canonical json', lambda: cursors.decode_cursor('WzEsICJhIl0'),
            raises=ValueError)
    attempt(failures, 'reject invalid utf8', lambda: cursors.decode_cursor('_w'), raises=ValueError)
    attempt(failures, 'encode rejects bool timestamp', lambda: cursors.encode_cursor(True, 'a'),
            raises=ValueError)
    attempt(failures, 'encode rejects string timestamp', lambda: cursors.encode_cursor('1', 'a'),
            raises=ValueError)
    attempt(failures, 'encode rejects float timestamp', lambda: cursors.encode_cursor(1.5, 'a'),
            raises=ValueError)
    attempt(failures, 'encode rejects non-string id', lambda: cursors.encode_cursor(1, 2),
            raises=ValueError)

    records = [
        {'created_at': 3, 'id': 'a', 'n': 1},
        {'created_at': 2, 'id': 'c', 'n': 2},
        {'created_at': 3, 'id': 'b', 'n': 3},
        {'created_at': 2, 'id': 'c2', 'n': 4},
    ]
    saved = copy.deepcopy(records)
    first_three = [
        {'created_at': 3, 'id': 'b', 'n': 3},
        {'created_at': 3, 'id': 'a', 'n': 1},
        {'created_at': 2, 'id': 'c2', 'n': 4},
    ]
    attempt(failures, 'first page ordering', lambda: cursors.paginate(records, 3),
            (first_three, 'WzIsImMyIl0'))
    attempt(failures, 'second page', lambda: cursors.paginate(records, 3, 'WzIsImMyIl0'),
            ([{'created_at': 2, 'id': 'c', 'n': 2}], None))
    attempt(failures, 'page limit', lambda: cursors.paginate(records, 2),
            (first_three[:2], 'WzMsImEiXQ'))
    attempt(failures, 'limit larger than remaining', lambda: cursors.paginate(records, 10, 'WzMsImEiXQ'),
            ([{'created_at': 2, 'id': 'c2', 'n': 4}, {'created_at': 2, 'id': 'c', 'n': 2}], None))
    attempt(failures, 'empty records', lambda: cursors.paginate([], 5), ([], None))
    attempt(failures, 'cursor beyond end', lambda: cursors.paginate(records, 5, 'WzEsImEiXQ'),
            ([], None))
    attempt(failures, 'generator records', lambda: cursors.paginate(
        (record for record in records), 3), (first_three, 'WzIsImMyIl0'))

    def detached():
        page, _ = cursors.paginate(records, 10)
        page[0]['id'] = 'changed'
        page.append({'created_at': 0, 'id': 'extra'})
        return records

    attempt(failures, 'detached page records', detached, saved)
    attempt(failures, 'zero limit', lambda: cursors.paginate(records, 0), raises=ValueError)
    attempt(failures, 'negative limit', lambda: cursors.paginate(records, -1), raises=ValueError)
    attempt(failures, 'bool limit', lambda: cursors.paginate(records, True), raises=ValueError)
    attempt(failures, 'float limit', lambda: cursors.paginate(records, 1.5), raises=ValueError)
    attempt(failures, 'string limit', lambda: cursors.paginate(records, '2'), raises=ValueError)
    attempt(failures, 'non-iterable records', lambda: cursors.paginate(42, 2), raises=ValueError)
    attempt(failures, 'non-dict record', lambda: cursors.paginate([1], 2), raises=ValueError)
    attempt(failures, 'missing id', lambda: cursors.paginate([{'created_at': 1}], 2),
            raises=ValueError)
    attempt(failures, 'bool record timestamp',
            lambda: cursors.paginate([{'created_at': True, 'id': 'a'}], 2), raises=ValueError)
    attempt(failures, 'duplicate pair', lambda: cursors.paginate(
        [{'created_at': 1, 'id': 'a'}, {'created_at': 1, 'id': 'a'}], 2), raises=ValueError)
    attempt(failures, 'invalid cursor type', lambda: cursors.paginate(records, 2, b'abc'),
            raises=ValueError)
    return finish(failures)


if __name__ == '__main__':
    raise SystemExit(main(sys.argv))
