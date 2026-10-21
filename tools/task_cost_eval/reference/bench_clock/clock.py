import re


def parse_time(text):
    if not isinstance(text, str):
        raise ValueError('time must be a string')
    match = re.fullmatch(r'([0-9]{1,2}):([0-9]{2})(?: ([AaPp][Mm]))?', text)
    if match is None:
        raise ValueError('invalid clock syntax')
    hour, minute = int(match[1]), int(match[2])
    suffix = match[3]
    if minute > 59 or (suffix is None and hour > 23) or (suffix is not None and not 1 <= hour <= 12):
        raise ValueError('invalid clock range')
    if suffix is not None:
        hour = hour % 12 + (12 if suffix.lower() == 'pm' else 0)
    return hour * 60 + minute


def format_time(minutes):
    if not isinstance(minutes, int) or isinstance(minutes, bool) or not 0 <= minutes < 1440:
        raise ValueError('invalid minutes')
    return f'{minutes // 60:02d}:{minutes % 60:02d}'


def add_minutes(text, delta):
    if not isinstance(delta, int) or isinstance(delta, bool):
        raise ValueError('delta must be an integer')
    return format_time((parse_time(text) + delta) % 1440)
