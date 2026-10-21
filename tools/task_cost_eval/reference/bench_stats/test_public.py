import unittest
from fractions import Fraction
from stats import summarize


class StatsTests(unittest.TestCase):
    def test_summary(self):
        self.assertEqual(summarize([1, 2, 3]), dict(count=3, mean=2, min=1, max=3, variance=Fraction(2, 3)))

    def test_generator(self):
        self.assertEqual(summarize(x for x in [4, 4])['variance'], 0)

    def test_empty(self):
        self.assertEqual(summarize([]), dict(count=0, mean=None, min=None, max=None, variance=None))
