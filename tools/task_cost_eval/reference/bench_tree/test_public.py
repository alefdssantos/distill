import unittest
from tree import build_tree, render_tree


class TreeTests(unittest.TestCase):
    def test_nested(self):
        self.assertEqual(build_tree(['a/b', 'a/c']), {'a': {'b': {}, 'c': {}}})

    def test_sorted_prefix(self):
        self.assertEqual(render_tree(build_tree(['z', 'a/b']), '> '), '> a\n>   b\n> z')

    def test_explicit_parent(self):
        self.assertEqual(build_tree(['a/b', 'a']), {'a': {'b': {}}})
