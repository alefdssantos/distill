import copy
import unittest

from .ultrafast_verdict import verdict


def result():
    rows = []
    for case in ('parallel', 'sequential'):
        for variant, mode, wall in (('baseline', 'normal', 100), ('candidate', 'ultrafast', 60)):
            rows.append({'case': case, 'repetition': 1, 'variant': variant, 'passed': True,
                'wall_time_s': wall, 'credits': 1, 'grader': {'wall_time_s': 1},
                'settings': {'execution_mode': mode, 'transport': 'acp', 'timeout_s': 900,
                    'binary_sha256': 'same', 'model': 'same', 'effort': 'auto', 'max_concurrent': 8},
                'acp': {'setup_confirmed': True, 'cleanup_complete': True,
                    'activation_receipt': {'result': {'mode': mode}}},
                'execution_mode_activation_confirmed': True, 'accounting_complete': True,
                'accounting_evidence': {'complete': True},
                'call_usage': [{'complete': True, 'model': 'same', 'effort': 'low'}],
                'frozen_inputs_unchanged': True, 'binary_unchanged': True})
    return {'cases': ['parallel', 'sequential'], 'repetitions': 1, 'runs': rows}


class UltraFastVerdictTest(unittest.TestCase):
    def test_same_quality_and_over_thirty_percent_paired_savings_pass(self):
        report = verdict(result())
        self.assertEqual(report['verdict'], 'PASS')
        self.assertAlmostEqual(report['median_paired_completion_ratio'], 61 / 101)
        self.assertEqual(report['candidate']['accepted'], 2)
        self.assertEqual(report['candidate']['calls_by_model_effort'], {'same/low': 2})

    def test_early_failure_is_charged_and_cannot_manufacture_speed(self):
        data = result()
        data['runs'][1].update(passed=False, wall_time_s=.1)
        report = verdict(data)
        self.assertEqual(report['verdict'], 'FAIL')
        self.assertEqual(report['quality_regressions'], 1)
        self.assertEqual(report['candidate']['charged_completion_s'], 961)
        self.assertEqual(report['candidate']['credits'], 2, 'failed usage remains in totals')

    def test_smaller_improvement_does_not_meet_predeclared_speed_target(self):
        data = result()
        for row in data['runs']:
            if row['variant'] == 'candidate':
                row['wall_time_s'] = 80
        self.assertEqual(verdict(data)['verdict'], 'FAIL')

    def test_incomplete_pairs_or_usage_or_unmatched_binary_are_incomplete(self):
        original = result()
        for failure in ('missing-pair', 'usage', 'binary'):
            with self.subTest(failure=failure):
                data = copy.deepcopy(original)
                if failure == 'missing-pair':
                    data['runs'].pop()
                elif failure == 'usage':
                    data['runs'][1]['accounting_complete'] = False
                else:
                    data['runs'][1]['settings']['binary_sha256'] = 'other'
                self.assertEqual(verdict(data)['verdict'], 'INCOMPLETE')
