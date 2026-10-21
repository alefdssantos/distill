import unittest
from config_merge import deep_merge, get_path


class ConfigMergeTests(unittest.TestCase):
    def test_nested_merge(self):
        self.assertEqual(deep_merge({'a': {'x': 1}}, {'a': {'y': 2}}), {'a': {'x': 1, 'y': 2}})

    def test_explicit_override(self):
        self.assertEqual(deep_merge({'x': [1], 'y': 2}, {'x': [3], 'y': None}), {'x': [3], 'y': None})

    def test_dotted_getter(self):
        self.assertEqual(get_path({'a': {'b': 4}}, 'a.b'), 4)
