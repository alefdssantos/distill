import unittest
from toposort import topological_sort


class ToposortTests(unittest.TestCase):
    def test_dependencies(self):
        self.assertEqual(topological_sort({'app': ['lib'], 'lib': []}), ['lib', 'app'])

    def test_available_ties(self):
        self.assertEqual(topological_sort({'z': [], 'a': [], 'b': ['a']}), ['a', 'b', 'z'])

    def test_duplicates(self):
        self.assertEqual(topological_sort({'b': ['a', 'a'], 'a': []}), ['a', 'b'])
