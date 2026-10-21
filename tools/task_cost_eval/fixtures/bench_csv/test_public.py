import unittest
from csvparse import parse_csv, select_rows


class CsvTests(unittest.TestCase):
    def test_parse_quoted_values(self):
        text = 'name,note\r\nAda,"hello, world"\r\nBob,"say ""hi"""\r\n'
        self.assertEqual(parse_csv(text), [
            {'name': 'Ada', 'note': 'hello, world'},
            {'name': 'Bob', 'note': 'say "hi"'},
        ])

    def test_parse_embedded_newline(self):
        self.assertEqual(parse_csv('id,body\n1,"line one\nline two"\n'),
                         [{'id': '1', 'body': 'line one\nline two'}])

    def test_select_and_filter(self):
        rows = [{'id': '1', 'kind': 'a', 'n': 'x'}, {'id': '2', 'kind': 'b', 'n': 'y'}]
        self.assertEqual(select_rows(rows, ['id', 'kind'], {'kind': 'b'}),
                         [{'id': '2', 'kind': 'b'}])
