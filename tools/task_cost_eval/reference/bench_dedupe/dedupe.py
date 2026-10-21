def dedupe_records(records, key, *, casefold=False):
    if not callable(key):
        raise ValueError('key must be callable')
    if not isinstance(casefold, bool):
        raise ValueError('casefold must be a bool')
    try:
        iterator = iter(records)
    except TypeError:
        raise ValueError('records must be iterable')
    seen = set()
    result = []
    for record in iterator:
        if not isinstance(record, dict):
            raise ValueError('records must be dictionaries')
        value = key(record)
        if not isinstance(value, str):
            raise ValueError('key must return a string')
        marker = value.casefold() if casefold else value
        if marker in seen:
            continue
        seen.add(marker)
        result.append(dict(record))
    return result
