import unittest
from dedupe import dedupe_records


class DedupeTests(unittest.TestCase):
    def test_keeps_first_occurrence(self):
        records = [{'id': 1, 'v': 'a'}, {'id': 1, 'v': 'b'}, {'id': 2, 'v': 'c'}]
        self.assertEqual(dedupe_records(records, lambda record: str(record['id'])),
                         [{'id': 1, 'v': 'a'}, {'id': 2, 'v': 'c'}])

    def test_casefold_keys(self):
        records = [{'email': 'A@X.test'}, {'email': 'a@x.test'}]
        self.assertEqual(dedupe_records(records, lambda record: record['email'], casefold=True),
                         [{'email': 'A@X.test'}])
