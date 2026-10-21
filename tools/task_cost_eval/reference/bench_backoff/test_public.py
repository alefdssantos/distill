import unittest
from backoff import is_retryable, retry_schedule


class BackoffTests(unittest.TestCase):
    def test_zero_delays(self):
        self.assertEqual(retry_schedule(3, 0, 0, 'seed'), [0.0, 0.0, 0.0])

    def test_repeatable(self):
        self.assertEqual(retry_schedule(4, 1, 5, 'seed'), retry_schedule(4, 1, 5, 'seed'))

    def test_classification(self):
        self.assertTrue(is_retryable(TimeoutError()))
        self.assertFalse(is_retryable(ValueError()))
