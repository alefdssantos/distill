def build_tree(paths):
    if isinstance(paths, (str, bytes)):
        raise ValueError('paths must be an iterable of strings')
    try:
        paths = iter(paths)
    except TypeError:
        raise ValueError('paths must be iterable') from None
    result, seen = {}, set()
    for path in paths:
        if not isinstance(path, str) or not path:
            raise ValueError('path must be a nonempty string')
        parts = path.split('/')
        if any(not part or part in ('.', '..') or '\n' in part or '\r' in part for part in parts):
            raise ValueError('invalid path component')
        if path in seen:
            raise ValueError('duplicate path: ' + path)
        seen.add(path)
        node = result
        for part in parts:
            node = node.setdefault(part, {})
    return result


def render_tree(tree, prefix=''):
    if not isinstance(prefix, str) or '\n' in prefix or '\r' in prefix:
        raise ValueError('prefix must be a single-line string')
    lines, active = [], set()
    def visit(node, depth):
        if not isinstance(node, dict) or id(node) in active:
            raise ValueError('tree must be an acyclic nested dict')
        active.add(id(node))
        for name in node:
            if not isinstance(name, str) or not name or name in ('.', '..') or any(c in name for c in '/\n\r'):
                raise ValueError('invalid node name')
        for name in sorted(node):
            lines.append(prefix + '  ' * depth + name)
            visit(node[name], depth + 1)
        active.remove(id(node))
    visit(tree, 0)
    return '\n'.join(lines)
