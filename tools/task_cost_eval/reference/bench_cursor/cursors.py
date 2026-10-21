import base64
import json
import re


def _validate_pair(created_at, id):
    if isinstance(created_at, bool) or not isinstance(created_at, int):
        raise ValueError('created_at must be an integer')
    if not isinstance(id, str):
        raise ValueError('id must be a string')


def encode_cursor(created_at, id):
    _validate_pair(created_at, id)
    payload = json.dumps([created_at, id], ensure_ascii=False, separators=(',', ':'))
    return base64.urlsafe_b64encode(payload.encode('utf-8')).decode('ascii').rstrip('=')


def decode_cursor(token):
    if not isinstance(token, str):
        raise ValueError('cursor must be a string')
    if token == '':
        raise ValueError('cursor must not be empty')
    if '=' in token or re.fullmatch(r'[A-Za-z0-9_-]+', token) is None:
        raise ValueError('cursor must be unpadded base64url')
    if len(token) % 4 == 1:
        raise ValueError('cursor has an invalid base64url length')
    try:
        raw = base64.urlsafe_b64decode((token + '=' * (-len(token) % 4)).encode('ascii'))
    except Exception:
        raise ValueError('cursor is not valid base64url')
    try:
        payload = json.loads(raw.decode('utf-8'))
    except (UnicodeDecodeError, json.JSONDecodeError):
        raise ValueError('cursor payload must be UTF-8 JSON')
    if not isinstance(payload, list) or len(payload) != 2:
        raise ValueError('cursor payload must be a two-element array')
    created_at, id = payload
    _validate_pair(created_at, id)
    if encode_cursor(created_at, id) != token:
        raise ValueError('cursor is not canonical')
    return created_at, id


def paginate(records, limit, cursor=None):
    if isinstance(limit, bool) or not isinstance(limit, int) or limit < 1:
        raise ValueError('limit must be a positive integer')
    try:
        records = list(records)
    except TypeError:
        raise ValueError('records must be iterable')
    seen = set()
    for record in records:
        if not isinstance(record, dict):
            raise ValueError('records must be dictionaries')
        if 'created_at' not in record or 'id' not in record:
            raise ValueError('records must contain created_at and id')
        _validate_pair(record['created_at'], record['id'])
        pair = (record['created_at'], record['id'])
        if pair in seen:
            raise ValueError('records must have unique (created_at, id) pairs')
        seen.add(pair)
    if cursor is not None:
        if not isinstance(cursor, str):
            raise ValueError('cursor must be a string or None')
        position = decode_cursor(cursor)
    else:
        position = None
    ordered = sorted(records, key=lambda record: (record['created_at'], record['id']), reverse=True)
    if position is not None:
        ordered = [record for record in ordered
                   if (record['created_at'], record['id']) < position]
    page = ordered[:limit]
    next_cursor = None
    if len(ordered) > len(page):
        last = page[-1]
        next_cursor = encode_cursor(last['created_at'], last['id'])
    return [dict(record) for record in page], next_cursor
