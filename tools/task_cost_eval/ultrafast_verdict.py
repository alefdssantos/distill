"""Bounded paired UltraFast speed/quality gate; not a statistical equivalence test."""
from __future__ import annotations

import math
import statistics


def validated_pairs(results, *, candidate_mode, baseline_modes):
    """Check the shared frozen-input, ACP and accounting evidence for paired modes."""
    reasons, pairs = [], {}
    expected = {(case, rep) for case in results['cases']
                for rep in range(1, results['repetitions'] + 1)}
    profiles = []
    for row in results['runs']:
        variant = row['variant']
        key = (row['case'], row['repetition'])
        label = f'{variant} {key[0]}#{key[1]}'
        if variant not in ('baseline', 'candidate') or key not in expected or variant in pairs.get(key, {}):
            reasons.append(f'{label}: unexpected or duplicate run')
            continue
        pairs.setdefault(key, {})[variant] = row
        settings = row['settings']
        profiles.append({key: settings.get(key) for key in (
            'binary_sha256', 'model', 'effort', 'configured_models', 'configured_utility',
            'configured_main_effort_auto', 'max_concurrent', 'capacity_env', 'max_depth',
            'depth_env', 'timeout_s', 'order_seed', 'tools', 'permissions')})
        mode = settings.get('execution_mode')
        expected_mode = (candidate_mode,) if variant == 'candidate' else baseline_modes
        acp = row.get('acp') or {}
        activation = (acp.get('activation_receipt') or {}).get('result') or {}
        if (settings.get('transport') != 'acp' or mode not in expected_mode or
                activation.get('mode') != mode or not acp.get('setup_confirmed') or
                not acp.get('cleanup_complete') or not row.get('execution_mode_activation_confirmed')):
            reasons.append(f'{label}: missing mode/setup/cleanup confirmation')
        if (not row.get('accounting_complete') or not (row.get('accounting_evidence') or {}).get('complete') or
                not row.get('call_usage') or any(not call.get('complete') for call in row['call_usage'])):
            reasons.append(f'{label}: incomplete all-call accounting')
        if not row.get('frozen_inputs_unchanged') or not row.get('binary_unchanged'):
            reasons.append(f'{label}: frozen input/binary evidence missing')
        for field in ('wall_time_s', 'credits'):
            value = row.get(field)
            if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value) or value < 0:
                reasons.append(f'{label}: invalid {field}')
        if not isinstance(settings.get('timeout_s'), (float, int)) or settings['timeout_s'] <= 0:
            reasons.append(f'{label}: missing positive timeout')
    if not expected or set(pairs) != expected or any(set(pair) != {'baseline', 'candidate'} for pair in pairs.values()):
        reasons.append('missing declared case/repetition pairs')
    if not profiles or any(profile != profiles[0] for profile in profiles):
        reasons.append('binary/model/effort/capacity/timeout settings differ between runs')
    return reasons, pairs


def verdict(results):
    reasons, pairs = validated_pairs(results, candidate_mode='ultrafast', baseline_modes=('normal', 'ultracode'))
    report = {'verdict': 'INCOMPLETE', 'reasons': reasons,
              'target': {'maximum_median_paired_completion_ratio': 0.70,
                         'maximum_quality_regressions': 0},
              'scope': 'bounded observed sample; no statistical equivalence or general superiority claim',
              'pairs': [], 'baseline': None, 'candidate': None}
    if reasons:
        return report

    def completion(row):
        # Include verification and charge failures the full time budget: an early
        # failure must not count as delivering the requested result more quickly.
        elapsed = row['wall_time_s'] + row['grader']['wall_time_s']
        return elapsed if row['passed'] else max(elapsed, row['settings']['timeout_s'])

    for key, pair in sorted(pairs.items()):
        baseline, candidate = pair['baseline'], pair['candidate']
        base_time, cand_time = completion(baseline), completion(candidate)
        if base_time <= 0:
            report['reasons'].append(f'{key}: nonpositive baseline completion time')
            return report
        report['pairs'].append({'case': key[0], 'repetition': key[1],
            'baseline_passed': baseline['passed'], 'candidate_passed': candidate['passed'],
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
            'runs': len(rows), 'accepted': sum(row['passed'] for row in rows),
            'raw_elapsed_s': sum(row['wall_time_s'] for row in rows),
            'verification_s': sum(row['grader']['wall_time_s'] for row in rows),
            'charged_completion_s': sum(completion(row) for row in rows),
            'median_raw_elapsed_s': statistics.median(row['wall_time_s'] for row in rows),
            'slowest_raw_elapsed_s': max(row['wall_time_s'] for row in rows),
            'credits': sum(row['credits'] for row in rows),
            'model_calls': sum(len(row['call_usage']) for row in rows),
            'calls_by_model_effort': efforts,
            'observed_peak_live_subagents': max(peaks) if all(value is not None for value in peaks) else None,
            'observed_subagent_attempts': sum(attempts) if all(value is not None for value in attempts) else None,
        }
    ratio = statistics.median(pair['completion_ratio'] for pair in report['pairs'])
    report['median_paired_completion_ratio'] = ratio
    report['median_paired_time_saved_fraction'] = 1 - ratio
    report['quality_regressions'] = sum(pair['quality_regression'] for pair in report['pairs'])
    if not report['baseline']['accepted']:
        report['reasons'].append('baseline accepted no task; no quality comparison basis')
        return report
    if ratio > 0.70:
        report['reasons'].append('median paired completion time did not improve by at least 30%')
    if report['quality_regressions']:
        report['reasons'].append('one or more baseline-passing pairs failed in UltraFast')
    report['verdict'] = 'FAIL' if report['reasons'] else 'PASS'
    return report
