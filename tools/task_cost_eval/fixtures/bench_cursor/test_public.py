import unittest
from cursors import decode_cursor, encode_cursor, paginate


class CursorTests(unittest.TestCase):
    def test_round_trip(self):
        token = encode_cursor(1700000000, 'abc')
        self.assertNotIn('=', token)
        self.assertEqual(decode_cursor(token), (1700000000, 'abc'))

    def test_first_page_order(self):
        records = [
            {'created_at': 3, 'id': 'a'},
            {'created_at': 3, 'id': 'b'},
            {'created_at': 2, 'id': 'c'},
        ]
        page, cursor = paginate(records, 2)
        self.assertEqual(page, [{'created_at': 3, 'id': 'b'}, {'created_at': 3, 'id': 'a'}])
        self.assertEqual(decode_cursor(cursor), (3, 'a'))

    def test_second_page(self):
        records = [
            {'created_at': 3, 'id': 'a'},
            {'created_at': 3, 'id': 'b'},
            {'created_at': 2, 'id': 'c'},
        ]
        page, cursor = paginate(records, 2, encode_cursor(3, 'a'))
        self.assertEqual(page, [{'created_at': 2, 'id': 'c'}])
        self.assertIsNone(cursor)
