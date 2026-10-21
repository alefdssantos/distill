"""Paired UltraSave token/quality gate over complete provider usage, including failures."""
from __future__ import annotations

import statistics

from .ultrafast_verdict import validated_pairs

TOKEN_FIELDS = ('input_tokens', 'cached_input_tokens', 'output_tokens', 'reasoning_tokens')


def verdict(results):
    reasons, pairs = validated_pairs(results, candidate_mode='ultrasave', baseline_modes=('normal',))
    report = {'verdict': 'INCOMPLETE', 'reasons': reasons,
              'target': {'maximum_median_paired_token_ratio': 0.80, 'maximum_quality_regressions': 0},
              'token_basis': 'sum of provider input_tokens + output_tokens over all physical calls; '
                             'cached input and reasoning are included, not added again',
              'scope': 'bounded observed sample; no statistical equivalence or general superiority claim',
              'pairs': [], 'baseline': None, 'candidate': None}
    if reasons:
        return report
    for row in results['runs']:
        label = f'{row["variant"]} {row["case"]}#{row["repetition"]}'
        entries = [row, *row['call_usage']]
        if any(type(entry.get(field)) is not int or entry[field] < 0
               for entry in entries for field in TOKEN_FIELDS):
            reasons.append(f'{label}: missing or invalid token usage')
            continue
        if any(entry['cached_input_tokens'] > entry['input_tokens'] or
               entry['reasoning_tokens'] > entry['output_tokens'] for entry in entries):
            reasons.append(f'{label}: token subsets exceed their provider totals')
        if any(row[field] != sum(call[field] for call in row['call_usage']) for field in TOKEN_FIELDS):
            reasons.append(f'{label}: per-call token usage does not reconcile with run totals')
        if row['input_tokens'] + row['output_tokens'] == 0:
            reasons.append(f'{label}: no provider tokens recorded')
    if reasons:
        return report

    def summarize(rows):
        tokens = {field: sum(row[field] for row in rows) for field in TOKEN_FIELDS}
        return {**tokens, 'total_tokens': tokens['input_tokens'] + tokens['output_tokens'],
                'uncached_input_tokens': tokens['input_tokens'] - tokens['cached_input_tokens'],
                'runs': len(rows), 'accepted': sum(row['passed'] for row in rows),
                'credits': sum(row['credits'] for row in rows),
                'raw_elapsed_s': sum(row['wall_time_s'] for row in rows),
                'verification_s': sum(row['grader']['wall_time_s'] for row in rows),
                'model_calls': sum(len(row['call_usage']) for row in rows)}

    for key, pair in sorted(pairs.items()):
        baseline, candidate = pair['baseline'], pair['candidate']
        base_usage, cand_usage = summarize([baseline]), summarize([candidate])
        raw_ratio = cand_usage['total_tokens'] / base_usage['total_tokens']
        both_passed = baseline['passed'] and candidate['passed']
        report['pairs'].append({'case': key[0], 'repetition': key[1],
            'baseline': base_usage, 'candidate': cand_usage,
            'baseline_passed': baseline['passed'], 'candidate_passed': candidate['passed'],
            'raw_token_ratio': raw_ratio, 'token_savings_eligible': both_passed,
            # A failed attempt keeps its real usage but cannot earn savings credit.
            'gate_token_ratio': raw_ratio if both_passed else max(1.0, raw_ratio),
            'quality_regression': baseline['passed'] and not candidate['passed']})
    for variant in ('baseline', 'candidate'):
        report[variant] = summarize([pair[variant] for pair in pairs.values()])
    ratio = statistics.median(pair['gate_token_ratio'] for pair in report['pairs'])
    report['median_paired_token_ratio'] = ratio
    report['median_paired_token_saved_fraction'] = 1 - ratio
    report['median_raw_paired_token_ratio'] = statistics.median(pair['raw_token_ratio'] for pair in report['pairs'])
    report['quality_regressions'] = sum(pair['quality_regression'] for pair in report['pairs'])
    if not report['baseline']['accepted']:
        reasons.append('baseline accepted no task; no quality comparison basis')
        return report
    if ratio > 0.80:
        reasons.append('median paired total tokens did not improve by at least 20%')
    if report['quality_regressions']:
        reasons.append('one or more baseline-passing pairs failed in UltraSave')
    report['verdict'] = 'FAIL' if reasons else 'PASS'
    return report
