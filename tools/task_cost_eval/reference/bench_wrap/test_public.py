import unittest
from wrapping import wrap_text


class WrapTests(unittest.TestCase):
    def test_greedy_wrap(self):
        self.assertEqual(wrap_text('the quick brown fox', 10), 'the quick\nbrown fox')

    def test_preserves_explicit_newlines(self):
        self.assertEqual(wrap_text('a\n\nb', 10), 'a\n\nb')

    def test_hard_splits_long_word(self):
        self.assertEqual(wrap_text('abcdefgh', 3), 'abc\ndef\ngh')
