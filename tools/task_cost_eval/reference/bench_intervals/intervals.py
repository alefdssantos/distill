import math


def _number(value):
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise ValueError('interval values must be numbers')
    if isinstance(value, float) and not math.isfinite(value):
        raise ValueError('interval values must be finite')
    return value


def _pairs(intervals):
    try:
        items = iter(intervals)
    except TypeError:
        raise ValueError('intervals must be iterable') from None
    result = []
    for item in items:
        if not isinstance(item, (tuple, list)) or len(item) != 2:
            raise ValueError('interval must be a pair')
        start = _number(item[0])
        end = _number(item[1])
        if not start < end:
            raise ValueError('interval start must be less than end')
        result.append((start, end))
    return result


def merge(intervals):
    ordered = sorted(_pairs(intervals))
    merged = []
    for start, end in ordered:
        if merged and start <= merged[-1][1]:
            if end > merged[-1][1]:
                merged[-1] = (merged[-1][0], end)
        else:
            merged.append((start, end))
    return merged


def covers(intervals, point):
    point = _number(point)
    return any(start <= point < end for start, end in merge(intervals))


def total_length(intervals):
    total = 0
    for start, end in merge(intervals):
        total += end - start
    return total
