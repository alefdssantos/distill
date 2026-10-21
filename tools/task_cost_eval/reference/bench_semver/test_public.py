import unittest
from semver import compare, parse


class SemVerTests(unittest.TestCase):
    def test_parse(self):
        self.assertEqual(parse('1.2.3-alpha.1+build.5'), (1, 2, 3, ('alpha', 1), ('build', '5')))

    def test_compare_release(self):
        self.assertEqual(compare('1.2.3', '1.2.4'), -1)

    def test_prerelease_precedes_release(self):
        self.assertEqual(compare('1.0.0-alpha', '1.0.0'), -1)
