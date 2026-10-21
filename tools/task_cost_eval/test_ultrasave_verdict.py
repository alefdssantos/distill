import copy
import unittest

from .test_ultrafast_verdict import result as paired_result
from .ultrasave_verdict import TOKEN_FIELDS, verdict


def result():
    data = paired_result()
    for row in data['runs']:
        candidate = row['variant'] == 'candidate'
        mode = 'ultrasave' if candidate else 'normal'
        row['settings']['execution_mode'] = mode
        row['acp']['activation_receipt']['result'] = {'mode': mode}
        row['call_usage'] = [
            {'complete': True, 'input_tokens': 60 if candidate else 100,
             'cached_input_tokens': 50 if candidate else 80, 'output_tokens': 20, 'reasoning_tokens': 10},
            {'complete': True, 'input_tokens': 10, 'cached_input_tokens': 5,
             'output_tokens': 5, 'reasoning_tokens': 0},
        ]
        row.update({field: sum(call[field] for call in row['call_usage']) for field in TOKEN_FIELDS})
    return data


class UltraSaveVerdictTest(unittest.TestCase):
    def test_all_calls_count_once_without_adding_cached_or_reasoning_tokens_again(self):
        report = verdict(result())
        self.assertEqual(report['verdict'], 'PASS')
        self.assertAlmostEqual(report['median_paired_token_ratio'], 95 / 135)
        self.assertEqual(report['baseline']['total_tokens'], 270)
        self.assertEqual(report['candidate']['total_tokens'], 190)
        self.assertEqual(report['candidate']['input_tokens'], 140)
        self.assertEqual(report['candidate']['cached_input_tokens'], 110)
        self.assertEqual(report['candidate']['uncached_input_tokens'], 30)
        self.assertEqual(report['candidate']['output_tokens'], 50)
        self.assertEqual(report['candidate']['reasoning_tokens'], 20)
        self.assertEqual(report['candidate']['model_calls'], 4)
        self.assertEqual(report['pairs'][0]['candidate']['total_tokens'], 95)

    def test_failed_candidate_cannot_earn_token_savings_and_retains_raw_usage(self):
        data = result()
        data['runs'][1]['passed'] = False
        report = verdict(data)
        self.assertEqual(report['verdict'], 'FAIL')
        self.assertEqual(report['quality_regressions'], 1)
        failed = report['pairs'][0]
        self.assertFalse(failed['token_savings_eligible'])
        self.assertEqual(failed['gate_token_ratio'], 1)
        self.assertAlmostEqual(failed['raw_token_ratio'], 95 / 135)
        self.assertEqual(report['candidate']['total_tokens'], 190)
        self.assertEqual(report['candidate']['credits'], 2)
        self.assertEqual(report['candidate']['raw_elapsed_s'], 120)

    def test_smaller_reduction_misses_the_predeclared_target(self):
        data = result()
        for row in data['runs']:
            if row['variant'] == 'candidate':
                row['call_usage'][0]['input_tokens'] += 25
                row['input_tokens'] += 25
        self.assertEqual(verdict(data)['verdict'], 'FAIL')

    def test_missing_or_unreconciled_usage_is_incomplete_not_zero(self):
        for failure in ('missing', 'unreconciled', 'subset', 'audit'):
            with self.subTest(failure=failure):
                data = copy.deepcopy(result())
                row = data['runs'][1]
                if failure == 'missing':
                    row['call_usage'][0].pop('output_tokens')
                elif failure == 'unreconciled':
                    row['input_tokens'] += 1
                elif failure == 'subset':
                    row['call_usage'][0]['reasoning_tokens'] = 21
                    row['reasoning_tokens'] = 21
                else:
                    row['accounting_complete'] = False
                report = verdict(data)
                self.assertEqual(report['verdict'], 'INCOMPLETE')
                self.assertIsNone(report['candidate'])
