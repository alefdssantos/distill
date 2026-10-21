import re
from decimal import Decimal, ROUND_HALF_UP
from fractions import Fraction

_UNITS = {
    'B': 1,
    'KB': 1000,
    'MB': 1000 ** 2,
    'GB': 1000 ** 3,
    'TB': 1000 ** 4,
    'KiB': 1024,
    'MiB': 1024 ** 2,
    'GiB': 1024 ** 3,
    'TiB': 1024 ** 4,
}
_PATTERN = re.compile(r'(?P<number>[0-9]+(?:\.[0-9]+)?) (?P<unit>B|KB|MB|GB|TB|KiB|MiB|GiB|TiB)')
_DISPLAY = (('B', 1), ('KB', 1000), ('MB', 1000 ** 2), ('GB', 1000 ** 3), ('TB', 1000 ** 4))


def parse_size(text):
    if not isinstance(text, str):
        raise ValueError('size must be a string')
    match = _PATTERN.fullmatch(text.strip())
    if match is None:
        raise ValueError('invalid size')
    amount = Fraction(match.group('number')) * _UNITS[match.group('unit')]
    if amount.denominator != 1:
        raise ValueError('size must be a whole number of bytes')
    return int(amount)


def format_size(num_bytes):
    if isinstance(num_bytes, bool) or not isinstance(num_bytes, int) or num_bytes < 0:
        raise ValueError('size must be a non-negative int')
    if num_bytes < 1000:
        return f'{num_bytes} B'
    index = max(i for i, (_, factor) in enumerate(_DISPLAY) if num_bytes >= factor)
    while True:
        unit, factor = _DISPLAY[index]
        if unit == 'B':
            return f'{num_bytes} B'
        value = (Decimal(num_bytes) / Decimal(factor)).quantize(Decimal('0.01'), rounding=ROUND_HALF_UP)
        if value >= 1000 and index + 1 < len(_DISPLAY):
            index += 1
            continue
        text = format(value, 'f').rstrip('0').rstrip('.')
        return f'{text} {unit}'
