import unittest
from sizes import format_size, parse_size


class SizeTests(unittest.TestCase):
    def test_parse_decimal_units(self):
        self.assertEqual(parse_size('10 B'), 10)
        self.assertEqual(parse_size('2 MB'), 2000000)

    def test_parse_binary_unit(self):
        self.assertEqual(parse_size('1.5 KiB'), 1536)

    def test_format_bytes(self):
        self.assertEqual(format_size(2000000), '2 MB')
