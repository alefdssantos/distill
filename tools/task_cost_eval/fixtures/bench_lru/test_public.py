import unittest
from lru import LRUCache


class LruTests(unittest.TestCase):
    def test_hit(self):
        cache = LRUCache(2, lambda: 0)
        cache.set('name', 'Ada', 10)
        self.assertEqual(cache.get('name'), 'Ada')

    def test_eviction(self):
        cache = LRUCache(1, lambda: 0)
        cache.set('a', 1, 10)
        cache.set('b', 2, 10)
        self.assertIsNone(cache.peek('a'))

    def test_expiry(self):
        now = [0]
        cache = LRUCache(1, lambda: now[0])
        cache.set('a', 1, 2)
        now[0] = 2
        self.assertEqual(cache.get('a', 'missing'), 'missing')
