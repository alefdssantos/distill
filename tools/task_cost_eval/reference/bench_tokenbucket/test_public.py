import unittest
from tokenbucket import TokenBucket


class TokenBucketTests(unittest.TestCase):
    def test_refill(self):
        bucket = TokenBucket(2, 1)
        self.assertTrue(bucket.allow('a', 0, 2))
        self.assertFalse(bucket.allow('a', 0))
        self.assertTrue(bucket.allow('a', 1))

    def test_independent_keys(self):
        bucket = TokenBucket(1, 0)
        self.assertTrue(bucket.allow('a', 3))
        self.assertTrue(bucket.allow('b', 0))
