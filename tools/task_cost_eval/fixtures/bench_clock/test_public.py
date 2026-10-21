import unittest
from clock import parse_time, format_time, add_minutes


class ClockTests(unittest.TestCase):
    def test_parse(self):
        self.assertEqual(parse_time('9:05 am'), 545)

    def test_format(self):
        self.assertEqual(format_time(1439), '23:59')

    def test_add(self):
        self.assertEqual(add_minutes('23:59', 2), '00:01')
