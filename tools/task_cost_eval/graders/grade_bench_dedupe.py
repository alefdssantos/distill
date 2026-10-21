#!/usr/bin/env python3
import copy
import sys
from _par_common import attempt, finish, fixture_dir, import_fresh


def main(argv):
    failures = []
    try:
        dedupe, = import_fresh(fixture_dir(argv, 'bench_dedupe'), 'dedupe')
    except Exception as error:
        return finish(['import: ' + repr(error)])
    records = [{'id': 'a', 'n': 1}, {'id': 'b', 'n': 2}, {'id': 'a', 'n': 3}, {'id': 'c', 'n': 4}]
    saved = copy.deepcopy(records)
    attempt(failures, 'keeps first occurrence',
            lambda: dedupe.dedupe_records(records, lambda record: record['id']),
            [{'id': 'a', 'n': 1}, {'id': 'b', 'n': 2}, {'id': 'c', 'n': 4}])
    attempt(failures, 'all unique preserves order',
            lambda: dedupe.dedupe_records([{'k': 'z'}, {'k': 'a'}, {'k': 'm'}],
                                          lambda record: record['k']),
            [{'k': 'z'}, {'k': 'a'}, {'k': 'm'}])
    attempt(failures, 'empty input', lambda: dedupe.dedupe_records([], lambda record: record['id']), [])
    attempt(failures, 'case sensitive by default',
            lambda: dedupe.dedupe_records([{'k': 'A'}, {'k': 'a'}], lambda record: record['k']),
            [{'k': 'A'}, {'k': 'a'}])
    attempt(failures, 'casefold collapses keys',
            lambda: dedupe.dedupe_records([{'k': 'A'}, {'k': 'a'}], lambda record: record['k'],
                                          casefold=True), [{'k': 'A'}])
    attempt(failures, 'unicode casefold',
            lambda: dedupe.dedupe_records([{'k': 'Straße'}, {'k': 'STRASSE'}],
                                          lambda record: record['k'], casefold=True),
            [{'k': 'Straße'}])
    attempt(failures, 'generator input',
            lambda: dedupe.dedupe_records((record for record in records), lambda record: record['id']),
            [{'id': 'a', 'n': 1}, {'id': 'b', 'n': 2}, {'id': 'c', 'n': 4}])
    attempt(failures, 'empty string key',
            lambda: dedupe.dedupe_records([{'v': 1}, {'v': 2}], lambda record: ''),
            [{'v': 1}])
    attempt(failures, 'dict order retained',
            lambda: list(dedupe.dedupe_records([{'b': 1, 'a': 2}], lambda record: 'k')[0]),
            ['b', 'a'])

    def detached():
        result = dedupe.dedupe_records(records, lambda record: record['id'])
        result[0]['id'] = 'changed'
        result[0]['n'] = 99
        result.append({'id': 'extra'})
        return records

    attempt(failures, 'detached shallow copies', detached, saved)

    calls = []

    def counting_key(record):
        calls.append(record['id'])
        return record['id']

    counting_records = [{'id': 'a'}, {'id': 'a'}, {'id': 'b'}]

    def counted():
        calls.clear()
        return dedupe.dedupe_records(counting_records, counting_key), list(calls)

    attempt(failures, 'key called once per record', counted,
            ([{'id': 'a'}, {'id': 'b'}], ['a', 'a', 'b']))
    attempt(failures, 'non-dict record',
            lambda: dedupe.dedupe_records([1], lambda record: 'k'), raises=ValueError)
    attempt(failures, 'non-string key result',
            lambda: dedupe.dedupe_records([{'k': 1}], lambda record: record['k']), raises=ValueError)
    attempt(failures, 'non-callable key',
            lambda: dedupe.dedupe_records([{'k': 'a'}], 'not callable'), raises=ValueError)
    attempt(failures, 'non-bool casefold',
            lambda: dedupe.dedupe_records([{'k': 'a'}], lambda record: record['k'], casefold=1),
            raises=ValueError)
    attempt(failures, 'non-iterable records',
            lambda: dedupe.dedupe_records(42, lambda record: 'k'), raises=ValueError)
    return finish(failures)


if __name__ == '__main__':
    raise SystemExit(main(sys.argv))
