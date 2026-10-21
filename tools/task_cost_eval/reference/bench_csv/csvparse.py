import csv
import io


def parse_csv(text):
    if not isinstance(text, str):
        raise ValueError('CSV text must be a string')
    if text == '':
        return []
    reader = csv.reader(io.StringIO(text, newline=''), strict=True)
    try:
        records = [record for record in reader if record]
    except csv.Error as error:
        raise ValueError('malformed CSV') from error
    if not records:
        return []
    header = records[0]
    if any(not isinstance(name, str) or name == '' for name in header):
        raise ValueError('header names must be non-empty strings')
    if len(set(header)) != len(header):
        raise ValueError('header names must be unique')
    rows = []
    for record in records[1:]:
        if len(record) != len(header):
            raise ValueError('each row must have the same number of fields as the header')
        rows.append(dict(zip(header, record)))
    return rows


def select_rows(rows, columns, filters):
    if not isinstance(columns, list) or any(not isinstance(column, str) for column in columns):
        raise ValueError('columns must be a list of strings')
    if len(set(columns)) != len(columns):
        raise ValueError('columns must be unique')
    if not isinstance(filters, dict) or any(
            not isinstance(key, str) or not isinstance(value, str) for key, value in filters.items()):
        raise ValueError('filters must be a mapping of strings to strings')
    try:
        iterator = iter(rows)
    except TypeError:
        raise ValueError('rows must be iterable')
    selected = []
    for row in iterator:
        if not isinstance(row, dict):
            raise ValueError('rows must be dictionaries')
        if any(key not in row for key in filters):
            raise ValueError('filter column not present in row')
        if any(row[key] != value for key, value in filters.items()):
            continue
        if any(column not in row for column in columns):
            raise ValueError('selected column not present in row')
        selected.append({column: row[column] for column in columns})
    return selected
