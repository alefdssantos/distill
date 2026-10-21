def encode_cursor(created_at, id):
    raise NotImplementedError('cursor encoding pending')


def decode_cursor(token):
    raise NotImplementedError('cursor decoding pending')


def paginate(records, limit, cursor=None):
    raise NotImplementedError('cursor pagination pending')
