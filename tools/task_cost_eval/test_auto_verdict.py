import unittest

from .auto_verdict import verdict
from .test_ultrafast_verdict import FOUR
from .test_ultrasave_verdict import result as paired_result


def result(cases=('parallel', 'sequential')):
    """Candidate runs Auto: activation reports the mode Auto chose (normal) plus choice 'auto'."""
    data = paired_result(cases)
    for row in data['runs']:
        if row['variant'] == 'candidate':
            row['settings']['execution_mode'] = 'auto'
            row['acp']['activation_receipt']['result'] = {'mode': 'normal', 'choice': 'auto'}
    return data


class AutoVerdictTest(unittest.TestCase):
    def test_cheaper_and_not_slower_passes_with_auto_choice_receipt(self):
        report = verdict(result())
        self.assertEqual(report['verdict'], 'PASS', report['reasons'])
        self.assertAlmostEqual(report['median_paired_credit_ratio'], .4)
        self.assertAlmostEqual(report['median_paired_completion_ratio'], 56 / 101)

    def test_slower_and_pricier_fails_both_ratios(self):
        data = result()
        for row in data['runs']:
            if row['variant'] == 'candidate':
                row['wall_time_s'] = 150
                row['credits'] = 12
        report = verdict(data)
        self.assertEqual(report['verdict'], 'FAIL')
        self.assertTrue(any('credit ratio' in reason for reason in report['reasons']), report['reasons'])
        self.assertTrue(any('completion ratio' in reason for reason in report['reasons']), report['reasons'])

    def test_even_one_unaccepted_auto_task_fails(self):
        data = result(FOUR)
        data['runs'][1]['passed'] = False
        report = verdict(data)
        self.assertEqual((report['quality_regressions'], report['verdict']), (1, 'FAIL'))
        self.assertTrue(any('Auto' in reason for reason in report['reasons']), report['reasons'])
        # A baseline failure must not excuse a candidate failure either.
        data['runs'][0]['passed'] = False
        report = verdict(data)
        self.assertEqual((report['quality_regressions'], report['verdict']), (0, 'FAIL'))

    def test_mode_only_activation_for_auto_candidate_is_incomplete(self):
        data = result()
        for row in data['runs']:
            if row['variant'] == 'candidate':
                row['acp']['activation_receipt']['result'] = {'mode': 'normal'}
        report = verdict(data)
        self.assertEqual(report['verdict'], 'INCOMPLETE')
        self.assertIsNone(report['candidate'])


if __name__ == '__main__':
    unittest.main()
