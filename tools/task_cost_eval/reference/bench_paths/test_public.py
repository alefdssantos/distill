import unittest
from paths import normalize_path


class PathTests(unittest.TestCase):
    def test_collapse_and_dots(self):
        self.assertEqual(normalize_path('/a//b/./c'), '/a/b/c')

    def test_parent_segments(self):
        self.assertEqual(normalize_path('/a/../b/'), '/b/')

    def test_cannot_escape_root(self):
        self.assertEqual(normalize_path('/../../a'), '/a')
