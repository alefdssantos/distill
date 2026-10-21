import unittest
from duration import format_duration, parse_duration


class DurationTests(unittest.TestCase):
    def test_parse_compound(self):
        self.assertEqual(parse_duration('P1DT2H3M4S'), 93784)

    def test_parse_fractional_seconds(self):
        self.assertEqual(parse_duration('PT0.5S'), 0.5)

    def test_format_compound(self):
        self.assertEqual(format_duration(93784), 'P1DT2H3M4S')
