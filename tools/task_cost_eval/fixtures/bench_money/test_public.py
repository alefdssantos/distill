import unittest
from money import allocate


class MoneyTests(unittest.TestCase):
    def test_even_split(self):
        self.assertEqual(allocate(100, [1, 1]), [50, 50])

    def test_remainder_goes_to_lower_index(self):
        self.assertEqual(allocate(100, [1, 1, 1]), [34, 33, 33])

    def test_weighted_split(self):
        self.assertEqual(allocate(1000, [5, 3, 2]), [500, 300, 200])
