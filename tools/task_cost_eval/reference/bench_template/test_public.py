import unittest
from template import render


class TemplateTests(unittest.TestCase):
    def test_escape(self):
        self.assertEqual(render('Hello {{name}}!', {'name': '<Ada>'}), 'Hello &lt;Ada&gt;!')

    def test_raw(self):
        self.assertEqual(render('{{{raw}}}', {'raw': '<b>yes</b>'}), '<b>yes</b>')

    def test_dotted(self):
        self.assertEqual(render('{{person.name}}', {'person': {'name': 'Ada'}}), 'Ada')
