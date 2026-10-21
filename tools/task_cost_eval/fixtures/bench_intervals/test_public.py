import unittest
from intervals import covers, merge, total_length


class IntervalTests(unittest.TestCase):
    def test_merge(self):
        self.assertEqual(merge([(5, 7), (1, 3), (3, 5)]), [(1, 7)])

    def test_covers(self):
        self.assertTrue(covers([(1, 3)], 1))
        self.assertFalse(covers([(1, 3)], 3))

    def test_total_length(self):
        self.assertEqual(total_length([(1, 3), (2, 5)]), 4)
