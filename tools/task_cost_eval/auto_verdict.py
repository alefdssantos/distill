"""Paired Auto gate: best quality at the lowest credit cost in the least time; bounded sample, not a statistical test."""
from __future__ import annotations

import statistics

from .ultrafast_verdict import (MAX_MISSED_ACCEPTANCES, MAX_QUALITY_REGRESSIONS, charged_completion_s,
                                quality_reasons, validated_pairs)
from .ultrasave_verdict import summarize, token_reasons

# Predeclared: Auto must not cost more or take longer than any fixed-mode baseline median,
# and must save at least 15% on one of the two axes.
MAX_RATIO = 1.0
MIN_SAVING_RATIO = 0.85


def verdict(results):
    compare_versions = bool(results.get('compare_versions'))
    baseline_modes = ('normal', 'ultrafast', 'ultrasave', 'ultracode') + (('auto',) if compare_versions else ())
    reasons, pairs = validated_pairs(results, candidate_mode='auto', baseline_modes=baseline_modes,
                                     compare_versions=compare_versions)
    report = {'verdict': 'INCOMPLETE', 'reasons': reasons, 'compare_versions': compare_versions,
              'target': {'maximum_median_paired_credit_ratio': MAX_RATIO,
                         'maximum_median_paired_completion_ratio': MAX_RATIO,
                         'minimum_saving_on_one_axis_ratio': MIN_SAVING_RATIO,
                         'maximum_quality_regressions': MAX_QUALITY_REGRESSIONS,
                         'minimum_candidate_accepted': 'runs - %d' % MAX_MISSED_ACCEPTANCES},
              'credit_basis': results.get('credit_basis'),
              'token_basis': 'sum of provider input_tokens + output_tokens over all physical calls; '
                             'cached input and reasoning are included, not added again',
              'scope': 'bounded observed sample; no statistical equivalence or general superiority claim',
              'pairs': [], 'baseline': None, 'candidate': None}
    if reasons:
        return report
    reasons.extend(token_reasons(results))
    if reasons:
        return report

    for key, pair in sorted(pairs.items()):
        baseline, candidate = pair['baseline'], pair['candidate']
        if baseline['credits'] <= 0:
            reasons.append(f'{key}: nonpositive baseline credits')
            return report
        base_time, cand_time = charged_completion_s(baseline), charged_completion_s(candidate)
        if base_time <= 0:
            reasons.append(f'{key}: nonpositive baseline completion time')
            return report
        raw_credit_ratio = candidate['credits'] / baseline['credits']
        both_passed = baseline['passed'] and candidate['passed']
        report['pairs'].append({'case': key[0], 'repetition': key[1],
            'baseline': summarize([baseline]), 'candidate': summarize([candidate]),
            'baseline_passed': baseline['passed'], 'candidate_passed': candidate['passed'],
            'raw_credit_ratio': raw_credit_ratio,
            'gate_credit_ratio': raw_credit_ratio if both_passed else max(1.0, raw_credit_ratio),
            'baseline_completion_s': base_time, 'candidate_completion_s': cand_time,
            'completion_ratio': cand_time / base_time,
            'quality_regression': baseline['passed'] and not candidate['passed']})
    for variant in ('baseline', 'candidate'):
        rows = [pair[variant] for pair in pairs.values()]
        peaks = [row['acp'].get('observed_peak_live_subagents') for row in rows]
        attempts = [row['acp'].get('observed_subagent_attempts') for row in rows]
        efforts = {}
        for row in rows:
            for call in row['call_usage']:
                key = f"{call.get('model')}/{call.get('effort') or 'unknown'}"
                efforts[key] = efforts.get(key, 0) + 1
        report[variant] = {
            **summarize(rows),
            'charged_completion_s': sum(charged_completion_s(row) for row in rows),
            'median_raw_elapsed_s': statistics.median(row['wall_time_s'] for row in rows),
            'slowest_raw_elapsed_s': max(row['wall_time_s'] for row in rows),
            'calls_by_model_effort': efforts,
            'observed_peak_live_subagents': max(peaks) if all(value is not None for value in peaks) else None,
            'observed_subagent_attempts': sum(attempts) if all(value is not None for value in attempts) else None,
        }
    credit_ratio = statistics.median(pair['gate_credit_ratio'] for pair in report['pairs'])
    completion_ratio = statistics.median(pair['completion_ratio'] for pair in report['pairs'])
    report['median_paired_credit_ratio'] = credit_ratio
    report['median_paired_completion_ratio'] = completion_ratio
    report['quality_regressions'] = sum(pair['quality_regression'] for pair in report['pairs'])
    if not report['baseline']['accepted']:
        reasons.append('baseline accepted no task; no quality comparison basis')
        return report
    if credit_ratio > MAX_RATIO:
        reasons.append(f'median paired credit ratio {credit_ratio:.3f} exceeds {MAX_RATIO}')
    if completion_ratio > MAX_RATIO:
        reasons.append(f'median paired completion ratio {completion_ratio:.3f} exceeds {MAX_RATIO}')
    if min(credit_ratio, completion_ratio) > MIN_SAVING_RATIO:
        reasons.append(f'neither median credit ratio nor completion ratio reached {MIN_SAVING_RATIO}')
    reasons.extend(quality_reasons(report, 'Auto'))
    report['verdict'] = 'FAIL' if reasons else 'PASS'
    return report
