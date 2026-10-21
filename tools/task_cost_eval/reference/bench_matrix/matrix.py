def build(triples):
    cells = {}
    try:
        for row, col, value in triples:
            if any(not isinstance(x, int) or isinstance(x, bool) for x in (row, col, value)) or row < 0 or col < 0:
                raise ValueError('invalid matrix entry')
            cells[row, col] = cells.get((row, col), 0) + value
    except (TypeError, ValueError) as error:
        raise ValueError('invalid matrix triples') from error
    return [(row, col, value) for (row, col), value in sorted(cells.items()) if value != 0]


def transpose(triples):
    return build((col, row, value) for row, col, value in build(triples))


def multiply(left, right):
    a, b = build(left), build(right)
    rows = {}
    for row, col, value in b:
        rows.setdefault(row, []).append((col, value))
    return build((row, col, value * other) for row, inner, value in a for col, other in rows.get(inner, []))
