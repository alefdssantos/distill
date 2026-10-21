import unittest
from matrix import build, transpose, multiply


class MatrixTests(unittest.TestCase):
    def test_build(self):
        self.assertEqual(build([(1, 0, 2), (0, 1, 3), (1, 0, 4)]), [(0, 1, 3), (1, 0, 6)])

    def test_transpose(self):
        self.assertEqual(transpose([(0, 2, 5)]), [(2, 0, 5)])

    def test_multiply(self):
        self.assertEqual(multiply([(0, 1, 3)], [(1, 2, 4)]), [(0, 2, 12)])
