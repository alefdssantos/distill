"""Guard the requested model pairs and honest published benchmark arithmetic."""
import json
import math
from pathlib import Path
import unittest

from .compare_additional import comparison_cost

RESULTS = Path(__file__).parent / 'results'
EXPECTED = {
    'astra': ('gpt-6-astra', 'gpt-6-astra', 'gpt-6-luna'),
    'sol': ('gpt-6.1-sol', 'gpt-6.1-sol', 'gpt-6-luna'),
    'opus': ('claude/claude-opus-5-5', 'claude/claude-opus-5-5', 'claude/claude-haiku-5-5'),
    'deepseek': ('deepseek/deepseek-v4.1-flash', 'deepseek-v4.1-flash', 'openrouter-free'),
}


class RequestedBenchmarkTests(unittest.TestCase):
    def test_exact_native_max_and_distill_auto_worker_pairs(self):
        for family, (native, main, worker) in EXPECTED.items():
            data = json.loads((RESULTS / f'requested-20261016-{family}.json').read_text())
            self.assertEqual(len(data['runs']), 18)
            for row in data['runs']:
                result = row['result']
                actual = result.get('main_model', result.get('model'))
                self.assertEqual(actual, main if row['arm'] == 'distill' else native)
                self.assertTrue(result['model_verified'] and result.get('worker_verified', True))
                self.assertTrue(row['credentials_cleaned'])
                if row['arm'] == 'distill':
                    self.assertEqual(result['worker_model'], worker)
                    self.assertEqual(result['requested_effort'], 'auto')
                    self.assertTrue(result['jev_enabled'])
                else:
                    self.assertEqual(result['requested_effort'], 'max')
                    command = row['command.json']
                    if family == 'opus':
                        self.assertEqual(command[command.index('--effort') + 1], 'max')
                    else:
                        self.assertIn('model_reasoning_effort="max"', command)
            for i in range(0, 18, 2):
                pair = data['runs'][i:i + 2]
                self.assertEqual({r['arm'] for r in pair}, {'native', 'distill'})
                self.assertEqual(pair[0]['case'], pair[1]['case'])
                self.assertEqual(pair[0]['repetition'], pair[1]['repetition'])

    def test_all_failures_and_unpriced_calls_stay_in_quality_and_cost_bounds(self):
        summary = json.loads((RESULTS / 'requested-20261016-summary.json').read_text())
        for family in EXPECTED:
            data = json.loads((RESULTS / f'requested-20261016-{family}.json').read_text())
            pair = summary['pairs'][family]
            for arm in ('native', 'distill'):
                rows = [r['result'] for r in data['runs'] if r['arm'] == arm]
                successes = sum(r['grader_passed'] and r['exit_code'] == 0 and not r['timed_out']
                                and r['frozen_inputs_unchanged'] for r in rows)
                self.assertEqual(pair[arm]['quality_passes'], successes)
                prices = [comparison_cost([c]) for r in rows for c in r['call_usage']]
                self.assertEqual(pair[arm]['unpriced_calls'], sum(p is None for p in prices))
                known = sum(p for p in prices if p is not None)
                self.assertTrue(math.isclose(pair[arm]['reference_cost_lower_bound_usd'], known))
            native, distill = pair['native'], pair['distill']
            if native['reference_cost_usd'] is None:
                self.assertEqual(family, 'deepseek')
                self.assertEqual(native['unpriced_calls'], 3)
                bound = 100 * (distill['reference_cost_usd'] / native['reference_cost_lower_bound_usd'] - 1)
                self.assertAlmostEqual(pair['cost_delta_percent_upper_bound'], bound)
                self.assertGreaterEqual(pair['cost_delta_display_upper_bound'], bound)
            else:
                self.assertAlmostEqual(pair['cost_delta_percent'],
                                       100 * (distill['reference_cost_usd'] / native['reference_cost_usd'] - 1))
            self.assertAlmostEqual(pair['quality_delta_percent'],
                                   100 * (distill['quality_passes'] / native['quality_passes'] - 1))

    def test_actual_free_worker_generations_have_zero_cost_receipts(self):
        data = json.loads((RESULTS / 'requested-20261016-deepseek.json').read_text())
        worker_calls = []
        for row in data['runs']:
            if row['arm'] == 'distill':
                worker_calls.extend(c for c in row['result']['call_usage']
                                    if c.get('agent') == 'worker' and c.get('role') != 'jev')
        self.assertTrue(worker_calls)
        for call in worker_calls:
            self.assertTrue(call['model'].endswith(':free'))
            self.assertEqual(call['cost_usd_ticks'], 0)


if __name__ == '__main__':
    unittest.main()
