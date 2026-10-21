import re

_CORE = re.compile(r'[0-9]+')
_IDENTIFIER = re.compile(r'[0-9A-Za-z-]+')


def _core(text):
    parts = text.split('.')
    if len(parts) != 3:
        raise ValueError('version must have major.minor.patch')
    numbers = []
    for part in parts:
        if not _CORE.fullmatch(part):
            raise ValueError('core parts must be ASCII digits')
        if len(part) > 1 and part[0] == '0':
            raise ValueError('core parts must not have leading zeros')
        numbers.append(int(part))
    return tuple(numbers)


def _identifiers(text, numeric):
    if not text:
        raise ValueError('empty identifier list')
    result = []
    for part in text.split('.'):
        if not part or not _IDENTIFIER.fullmatch(part):
            raise ValueError('invalid identifier')
        if numeric and part.isascii() and part.isdigit():
            if len(part) > 1 and part[0] == '0':
                raise ValueError('numeric prerelease identifiers must not have leading zeros')
            result.append(int(part))
        else:
            result.append(part)
    return tuple(result)


def parse(text):
    if not isinstance(text, str):
        raise ValueError('version must be a string')
    core = text
    build = ()
    prerelease = ()
    if '+' in core:
        core, build_text = core.split('+', 1)
        build = _identifiers(build_text, False)
    if '-' in core:
        core, prerelease_text = core.split('-', 1)
        prerelease = _identifiers(prerelease_text, True)
    major, minor, patch = _core(core)
    return (major, minor, patch, prerelease, build)


def compare(a, b):
    left = parse(a)
    right = parse(b)
    for lpart, rpart in zip(left[:3], right[:3]):
        if lpart != rpart:
            return -1 if lpart < rpart else 1
    lpre, rpre = left[3], right[3]
    if not lpre and not rpre:
        return 0
    if not lpre:
        return 1
    if not rpre:
        return -1
    for lpart, rpart in zip(lpre, rpre):
        if lpart == rpart:
            continue
        if isinstance(lpart, int) and isinstance(rpart, int):
            return -1 if lpart < rpart else 1
        if isinstance(lpart, int):
            return -1
        if isinstance(rpart, int):
            return 1
        return -1 if lpart < rpart else 1
    if len(lpre) == len(rpre):
        return 0
    return -1 if len(lpre) < len(rpre) else 1
