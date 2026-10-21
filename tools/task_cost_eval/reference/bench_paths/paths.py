def normalize_path(path):
    if not isinstance(path, str):
        raise ValueError('path must be a string')
    if path == '':
        raise ValueError('path must not be empty')
    if '\x00' in path:
        raise ValueError('path must not contain NUL bytes')
    if not path.startswith('/'):
        raise ValueError('path must be absolute')
    trailing = path.endswith('/')
    parts = []
    for part in path.split('/'):
        if part == '' or part == '.':
            continue
        if part == '..':
            if parts:
                parts.pop()
            continue
        parts.append(part)
    if not parts:
        return '/'
    result = '/' + '/'.join(parts)
    return result + '/' if trailing else result
