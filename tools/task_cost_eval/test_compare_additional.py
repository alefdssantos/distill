import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from .compare_additional import config, native_usage, provider_usage


class AdditionalBenchmarkTests(unittest.TestCase):
    def test_deepseek_config_parses_quoted_model_id_and_ablations(self):
        import tomllib

        text = config('deepseek-v4.1-flash', ablation=True)
        parsed = tomllib.loads(text)
        self.assertEqual(parsed['models']['default'], 'deepseek-v4.1-flash')
        self.assertEqual(parsed['models']['worker'], 'deepseek-v4.1-flash')
        self.assertFalse(parsed['jev']['enabled'])
        self.assertFalse(parsed['jev']['effort_auto'])
        self.assertEqual(parsed['models']['worker_effort'], 'low')
        self.assertEqual(parsed['model']['deepseek-v4.1-flash']['model'], '~deepseek/deepseek-flash-latest')

    def test_normal_config_enables_jev_and_auto_effort(self):
        import tomllib

        parsed = tomllib.loads(config('claude/claude-sonnet-5-5'))
        self.assertTrue(parsed['jev']['enabled'])
        self.assertTrue(parsed['jev']['effort_auto'])
        self.assertEqual(parsed['models']['worker_effort'], 'auto')

    def test_native_usage_reads_camel_case_cache_counts_and_cost(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'native.json'
            path.write_text(json.dumps({'modelUsage': {'claude-sonnet-5-5': {
                'inputTokens': 10, 'outputTokens': 2, 'cacheReadInputTokens': 1,
                'cacheCreationInputTokens': 3}}, 'total_cost_usd': 0.25, 'is_error': False}))
            result = native_usage(path)
        self.assertTrue(result['accounting_complete'])
        self.assertEqual(result['call_usage'][0]['cacheReadInputTokens'], 1)
        self.assertEqual(result['call_usage'][0]['cacheCreationInputTokens'], 3)
        self.assertEqual(result['cost_usd'], 0.25)
        self.assertFalse(result['native_is_error'])

    def test_native_usage_missing_unknown_cost_null_and_invalid_input(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'native.json'
            path.write_text(json.dumps({'modelUsage': {'m': {
                'inputTokens': 1, 'outputTokens': 0, 'cacheReadInputTokens': 0,
                'cacheCreationInputTokens': 0}}}))
            result = native_usage(path)
            self.assertTrue(result['accounting_complete'])
            self.assertIsNone(result['cost_usd'])
            path.write_text('{}')
            self.assertFalse(native_usage(path)['accounting_complete'])
            path.write_text('{')
            self.assertFalse(native_usage(path)['accounting_complete'])
            self.assertFalse(native_usage(Path(directory) / 'missing.json')['accounting_complete'])

    def test_native_usage_rejects_incomplete_cache_counts_and_negative_cost(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'native.json'
            path.write_text(json.dumps({'modelUsage': {'m': {
                'inputTokens': 1, 'outputTokens': 0, 'cacheReadInputTokens': 0}}}))
            self.assertFalse(native_usage(path)['accounting_complete'])
            path.write_text(json.dumps({'modelUsage': {'m': {
                'inputTokens': 1, 'outputTokens': 0, 'cacheReadInputTokens': 0,
                'cacheCreationInputTokens': 0}}, 'total_cost_usd': -1}))
            self.assertIsNone(native_usage(path)['cost_usd'])

    def test_provider_usage_requires_complete_unique_matching_successful_owned_rows(self):
        result = {'call_usage': [{'attempt': 'a', 'complete': True, 'status': 'completed', 'owner': 's', 'model': 'deepseek/deepseek-v4.1-flash'}],
                  'ledger_totals_match': True, 'unfolded_child_attempts': []}
        with tempfile.TemporaryDirectory() as directory:
            home = Path(directory)
            usage = home / 'sessions' / 's' / 'usage.json'
            usage.parent.mkdir(parents=True)
            usage.write_text(json.dumps({'session': {'usageIsIncomplete': False, 'modelCalls': 1}}))
            with patch('tools.task_cost_eval.compare_additional.distill_accounting', return_value=result.copy()):
                self.assertTrue(provider_usage(home, 's')['accounting_complete'])
            bad_rows = [
                [],
                [dict(result['call_usage'][0], attempt='missing', complete=False)],
                [result['call_usage'][0], dict(result['call_usage'][0])],
                [dict(result['call_usage'][0], status='failed')],
                [dict(result['call_usage'][0], owner=None)],
                [dict(result['call_usage'][0], model='unknown')],
            ]
            for rows in bad_rows:
                with self.subTest(rows=rows):
                    broken = dict(result, call_usage=rows)
                    with patch('tools.task_cost_eval.compare_additional.distill_accounting', return_value=broken):
                        self.assertFalse(provider_usage(home, 's')['accounting_complete'])
            incomplete = dict(result)
            incomplete['ledger_totals_match'] = False
            with patch('tools.task_cost_eval.compare_additional.distill_accounting', return_value=incomplete):
                self.assertFalse(provider_usage(home, 's')['accounting_complete'])
            usage.write_text(json.dumps({'session': {'usageIsIncomplete': True, 'modelCalls': 1}}))
            with patch('tools.task_cost_eval.compare_additional.distill_accounting', return_value=result.copy()):
                self.assertFalse(provider_usage(home, 's')['accounting_complete'])


    def test_rejected_answer_keeps_its_reported_bill_and_missing_cost_stays_unknown(self):
        row = {'attempt': 'a', 'complete': True, 'status': 'rejected',
               'owner': 's', 'model': 'deepseek/deepseek-v4.1-flash'}
        accounted = {'call_usage': [row], 'ledger_totals_match': True, 'unfolded_child_attempts': []}
        with tempfile.TemporaryDirectory() as directory:
            home = Path(directory)
            usage = home / 'sessions/s/usage.json'
            usage.parent.mkdir(parents=True)
            for cost, expected in [(12000000, .0012), (None, None)]:
                usage.write_text(json.dumps({'session': {'modelCalls': 1, 'attributions': [
                    {'attempt_id': 'a', 'cost_usd_ticks': cost, 'cost_basis': 'reported'}]}}))
                with patch('tools.task_cost_eval.compare_additional.distill_accounting', return_value=accounted):
                    result = provider_usage(home, 's')
                self.assertTrue(result['accounting_complete'])
                self.assertEqual(result['cost_usd'], expected)

    def test_claude_prices_uncached_input_once_and_splits_cache_write_durations(self):
        row = {'attempt': 'a', 'complete': True, 'status': 'completed',
               'owner': 's', 'model': 'claude-sonnet-5-5'}
        accounted = {'call_usage': [row], 'ledger_totals_match': True, 'unfolded_child_attempts': []}
        with tempfile.TemporaryDirectory() as directory:
            home = Path(directory)
            usage = home / 'sessions/s/usage.json'
            usage.parent.mkdir(parents=True)
            usage.write_text(json.dumps({'session': {'modelCalls': 1, 'attributions': [
                {'attempt_id': 'a', 'usage': {'prompt_tokens': 100, 'cached_prompt_tokens': 20,
                 'cache_creation_prompt_tokens': 30, 'cache_creation_1h_prompt_tokens': 10,
                 'completion_tokens': 10}}]}}))
            with patch('tools.task_cost_eval.compare_additional.distill_accounting', return_value=accounted):
                result = provider_usage(home, 's')
            self.assertTrue(result['accounting_complete'])
            self.assertAlmostEqual(result['cost_usd'], (50 * 2 + 20 * .1 + 20 * 2.5 + 10 * 4 + 10 * 10) / 1e6)


if __name__ == '__main__':
    unittest.main()
