"""Fixture proofs for the ultrafast-v2 cohort: seeded fails, reference passes."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parent
COHORT = ROOT / 'cohort-ultrafast-v2.json'


def cases():
    return json.loads(COHORT.read_text())['cases']


def run_grader(case, source):
    with tempfile.TemporaryDirectory(prefix='benchmark-v2-proof-') as tmp:
        workspace = Path(tmp)
        target = workspace / 'tools/task_cost_eval' / case['fixture_ref']
        shutil.copytree(ROOT / source, target)
        if (workspace / 'tools/task_cost_eval/reference').exists() or (workspace / 'tools/task_cost_eval/graders').exists():
            raise AssertionError('private artifacts leaked into workspace')
        env = {**os.environ, 'PYTHONDONTWRITEBYTECODE': '1'}
        return subprocess.run([sys.executable, str(ROOT / case['grader']['script_ref']), str(workspace)],
                              cwd=workspace, env=env, capture_output=True, text=True, timeout=30)


class BenchmarkCaseTests(unittest.TestCase):
    def test_cohort_shape(self):
        seen = set()
        for case in cases():
            for key in ('id', 'kind', 'prompt_ref', 'fixture_ref', 'grader', 'reference_ref'):
                self.assertIn(key, case, case.get('id'))
            self.assertTrue((ROOT / case['fixture_ref']).is_dir(), case['id'])
            self.assertTrue((ROOT / case['reference_ref']).is_dir(), case['id'])
            self.assertTrue((ROOT / case['prompt_ref']).is_file(), case['id'])
            self.assertTrue((ROOT / case['grader']['script_ref']).is_file(), case['id'])
            self.assertNotIn(case['id'], seen)
            seen.add(case['id'])
        self.assertEqual(len(seen), 20)

    def test_seeded_fails_and_reference_passes(self):
        for case in cases():
            with self.subTest(case=case['id']):
                seeded = run_grader(case, case['fixture_ref'])
                self.assertEqual(seeded.returncode, 1, case['id'] + ': seeded grader did not fail')
                self.assertNotIn('Traceback', seeded.stderr, case['id'] + ': grader crashed on seed')
                self.assertNotIn('import:', seeded.stderr, case['id'] + ': seed did not import')
                reference = run_grader(case, case['reference_ref'])
                self.assertEqual(reference.returncode, 0, case['id'] + ': reference failed\n' + reference.stderr)
                self.assertEqual(reference.stderr, '', case['id'] + ': reference emitted diagnostics')

    def test_reference_public_tests(self):
        for case in cases():
            with self.subTest(case=case['id']):
                env = {**os.environ, 'PYTHONDONTWRITEBYTECODE': '1'}
                result = subprocess.run([sys.executable, '-m', 'unittest', 'discover', '-v'],
                                        cwd=ROOT / case['reference_ref'], env=env, capture_output=True, text=True, timeout=30)
                self.assertEqual(result.returncode, 0, case['id'] + ': public tests failed\n' + result.stderr)


if __name__ == '__main__':
    unittest.main()
