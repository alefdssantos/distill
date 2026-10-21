import math
import re
from fractions import Fraction

_PATTERN = re.compile(
    r'^P(?:'
    r'(?P<weeks>[0-9]+(?:\.[0-9]+)?)W'
    r'|'
    r'(?:(?P<days>[0-9]+(?:\.[0-9]+)?)D)?'
    r'(?:T'
    r'(?:(?P<hours>[0-9]+(?:\.[0-9]+)?)H)?'
    r'(?:(?P<minutes>[0-9]+(?:\.[0-9]+)?)M)?'
    r'(?:(?P<seconds>[0-9]+(?:\.[0-9]+)?)S)?'
    r')?'
    r')$'
)
_FACTORS = (('weeks', 604800), ('days', 86400), ('hours', 3600), ('minutes', 60), ('seconds', 1))


def parse_duration(text):
    if not isinstance(text, str):
        raise ValueError('duration must be a string')
    match = _PATTERN.fullmatch(text)
    if match is None:
        raise ValueError('invalid ISO-8601 duration')
    present = [(name, match.group(name)) for name, _ in _FACTORS if match.group(name) is not None]
    if not present:
        raise ValueError('duration has no components')
    if 'T' in text and all(match.group(name) is None for name in ('hours', 'minutes', 'seconds')):
        raise ValueError('T must be followed by a time component')
    fractional = [name for name, value in present if '.' in value]
    if fractional and fractional != [present[-1][0]]:
        raise ValueError('fraction allowed only in the last component')
    total = Fraction(0)
    for name, value in present:
        factor = dict(_FACTORS)[name]
        total += Fraction(value) * factor
    if total.denominator == 1:
        return int(total)
    return float(total)


def format_duration(seconds):
    if isinstance(seconds, bool) or not isinstance(seconds, (int, float)):
        raise ValueError('seconds must be an int or float')
    if isinstance(seconds, float) and not math.isfinite(seconds):
        raise ValueError('seconds must be finite')
    if seconds < 0:
        raise ValueError('seconds must not be negative')
    value = Fraction(seconds) if isinstance(seconds, int) else Fraction(str(seconds))
    scaled = value * 1_000_000
    if scaled.denominator != 1:
        raise ValueError('seconds have more than six fractional digits')
    micros = int(scaled)
    days, micros = divmod(micros, 86_400_000_000)
    hours, micros = divmod(micros, 3_600_000_000)
    minutes, micros = divmod(micros, 60_000_000)
    whole, micros = divmod(micros, 1_000_000)
    parts = ['P']
    if days:
        parts.append(f'{days}D')
    time_parts = []
    if hours:
        time_parts.append(f'{hours}H')
    if minutes:
        time_parts.append(f'{minutes}M')
    if whole or micros:
        if micros:
            time_parts.append(f'{whole}.{micros:06d}'.rstrip('0') + 'S')
        else:
            time_parts.append(f'{whole}S')
    if time_parts:
        parts.append('T' + ''.join(time_parts))
    elif not days:
        return 'PT0S'
    return ''.join(parts)
