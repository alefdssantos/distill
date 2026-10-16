#!/usr/bin/env python3
"""Measure isolated Claude and DeepSeek cells against frozen external graders."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import time
import uuid

from .acp_driver import OwnedTree
from .compare_codex import ROOT, LOCAL_TOOLS, distill_accounting, distill_config
from .evaluate import _sha256_file, _sha256_tree
from .runner import _terminate_owned_group

SONNET = 'claude/claude-sonnet-5-5'
DEEPSEEK = 'deepseek-v4.1-flash'


def config(main, *, ablation=False):
    text = distill_config(main_model=main, worker_model=main,
                          worker_effort='low' if ablation else 'auto')
    text = text.replace('session_summary = "chatgpt/gpt-6-luna"', f'session_summary = "{main}"')
    text = text.replace('model = "chatgpt/gpt-6-luna"', f'model = "{main}"')
    text = text.replace('[jev]\n', f'[jev]\nenabled = {str(not ablation).lower()}\n')
    if ablation:
        text = text.replace('effort_auto = true', 'effort_auto = false')
    if main == DEEPSEEK:
        text += '''
[model."deepseek-v4.1-flash"]
model = "~deepseek/deepseek-flash-latest"
base_url = "https://openrouter.ai/api/v1"
api_backend = "chat_completions"
env_key = "OPENROUTER_API_KEY"
context_window = 1000000
max_completion_tokens = 131072
supports_reasoning_effort = true
reasoning_effort = "low"
supports_backend_search = false
stream_tool_calls = false
max_retries = 2
'''
    return text


def native_usage(path):
    try:
        data = json.loads(path.read_text())
        models = data.get('modelUsage', {})
        keys = ('inputTokens', 'outputTokens', 'cacheReadInputTokens', 'cacheCreationInputTokens')
        rows = [{'model': model, **row} for model, row in models.items()]
        complete = bool(rows) and all(all(isinstance(r.get(k), int) and r[k] >= 0 for k in keys) for r in rows)
        cost = data.get('total_cost_usd')
        return {'accounting_complete': complete, 'call_usage': rows,
                'cost_usd': cost if complete and isinstance(cost, (int, float)) and cost >= 0 else None,
                'credit_estimate': None,
                'observed_models': list(models), 'native_is_error': data.get('is_error', True),
                'reason': None if complete else 'native modelUsage missing or incomplete'}
    except (OSError, ValueError, TypeError) as error:
        return {'accounting_complete': False, 'cost_usd': None, 'call_usage': [], 'reason': str(error)}


def provider_usage(home, session):
    result = distill_accounting(home, session)
    rows = result.get('call_usage', [])
    ledgers = list((home / 'sessions').rglob('usage.json'))
    root = next((p for p in ledgers if p.parent.name == session), None)
    ledger = json.loads(root.read_text())['session'] if root else {}
    complete = (bool(rows) and result.get('ledger_totals_match', False)
                and not ledger.get('usageIsIncomplete', False)
                and len({r['attempt'] for r in rows}) == len(rows) == ledger.get('modelCalls')
                and not result.get('unfolded_child_attempts')
                and all(r.get('complete') and r.get('status') in ('completed', 'rejected') and r.get('owner')
                        and r.get('model') in ('claude-sonnet-5-5', 'deepseek/deepseek-v4.1-flash') for r in rows))
    attributions = ledger.get('attributions', [])
    by_attempt = {r['attempt_id']: r for r in attributions}
    for row in rows:
        attribution = by_attempt.get(row['attempt'], {})
        row['cost_usd_ticks'] = attribution.get('cost_usd_ticks')
        row['cost_basis'] = attribution.get('cost_basis')
        row['cache_creation_tokens'] = attribution.get('usage', {}).get('cache_creation_prompt_tokens', 0)
        row['cache_creation_1h_tokens'] = attribution.get('usage', {}).get('cache_creation_1h_prompt_tokens', 0)
    costs = [r.get('cost_usd_ticks') for r in attributions]
    cost_complete = complete and len(costs) == len(rows) and all(isinstance(c, int) and c >= 0 for c in costs)
    result.update(accounting_complete=bool(complete), credit_estimate=None,
                  cost_usd=sum(costs) / 1e10 if cost_complete else None,
                  cache_creation_tokens=ledger.get('cacheCreationTokens'),
                  cost_bases=sorted({r.get('cost_basis', 'unknown') for r in attributions}))
    if complete and not cost_complete and all(r['model'] == 'claude-sonnet-5-5' for r in rows):
        estimates = []
        for row in attributions:
            usage = row['usage']
            prompt, cached = usage['prompt_tokens'], usage.get('cached_prompt_tokens', 0)
            writes = usage.get('cache_creation_prompt_tokens', 0)
            hour_writes = usage.get('cache_creation_1h_prompt_tokens', 0)
            if not 0 <= hour_writes <= writes <= prompt - cached:
                break
            estimates.append(((prompt - cached - writes) * 2 + cached * .1
                              + (writes - hour_writes) * 2.5 + hour_writes * 4
                              + usage['completion_tokens'] * 10) / 1e6)
        if len(estimates) == len(rows) == len(attributions):
            result['cost_usd'] = sum(estimates)
            result['cost_bases'] = ['modeled_anthropic_public_api_rates_20261016']
    result['observed_models'] = sorted({r['model'] for r in rows})
    result['cost_basis'] = 'native per-attempt USD ticks (1e10 per dollar)' if cost_complete else ('modeled Anthropic public API rates, 2026-10-16; not subscription charges' if result['cost_usd'] is not None else 'unavailable; missing native per-attempt cost')
    return result


def run(args):
    case = next(r for r in json.loads((ROOT / 'cohort-v1.json').read_text())['cases'] if r['id'] == args.case)
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    home, work = out / 'isolated-home', out / 'worktree'
    home.mkdir(mode=0o700)
    target = work / 'tools/task_cost_eval' / case['fixture_ref']
    target.parent.mkdir(parents=True)
    shutil.copytree(ROOT / case['fixture_ref'], target)
    subprocess.run(['git', 'init', '-q', str(work)], check=True)
    subprocess.run(['git', 'add', '.'], cwd=work, check=True)
    subprocess.run(['git', '-c', 'user.name=Benchmark', '-c', 'user.email=benchmark@localhost', 'commit', '-qm', 'Frozen task input'], cwd=work, check=True)
    frozen = {'fixture_sha256': _sha256_tree(ROOT / case['fixture_ref']),
              'grader_sha256': _sha256_file(ROOT / case['grader']['script_ref'])}
    prompt = (ROOT / case['prompt_ref']).read_text()
    prompt += '\nWork only in this repository. Finish the code change autonomously. Do not commit.'
    prompt += '\nThe available Python interpreter is python3.'
    (out / 'prompt.txt').write_text(prompt)
    session = str(uuid.uuid4())
    native, ablation = args.variant == 'claude-native', args.variant == 'deepseek-same-agent-ablation'
    model = SONNET if args.variant in ('sonnet-distill', 'claude-native') else DEEPSEEK
    executable = Path(args.claude if native else args.distill).resolve()
    version = subprocess.check_output([str(executable), '--version'], text=True).strip()
    result = {'variant': args.variant, 'case': args.case, 'session': session, 'model': model,
              'version': version, 'executable_sha256': _sha256_file(executable),
              'harness_sha256': _sha256_file(Path(__file__)),
              'prompt_sha256': hashlib.sha256(prompt.encode()).hexdigest(), **frozen,
              'accounting_complete': False, 'cost_usd': None, 'wall_time_s': 0,
              'exit_code': None, 'timed_out': False}
    env = {k: v for k, v in os.environ.items()
           if not k.endswith('_API_KEY') and not k.startswith(('GROK_', 'DISTILL_', 'CLAUDE_', 'ANTHROPIC_'))}
    env.update(HOME=str(home), DISTILL_HOME=str(home), GROK_HOME=str(home),
               CLAUDE_CONFIG_DIR=str(home / 'claude'), GROK_MANAGED_MCPS_ENABLED='false',
               GROK_MANAGED_MCP_GATEWAY_TOOLS_ENABLED='false')
    proc = None
    tree = None
    result['cleanup_complete'] = False
    started = None
    credentials = []
    try:
        if native:
            auth = json.loads(args.claude_auth.read_text())
            env['CLAUDE_CODE_OAUTH_TOKEN'] = auth['access_token']
            command = [str(executable), '-p', '--output-format', 'json', '--model', SONNET.removeprefix('claude/'),
                       '--effort', 'medium', '--dangerously-skip-permissions', '--setting-sources', '',
                       '--strict-mcp-config', '--mcp-config', '{"mcpServers":{}}',
                       '--no-session-persistence', '--tools', 'Bash,Read,Edit,Write,Glob,Grep,Agent', '--', prompt]
        else:
            provider_auth = 'claude-auth.json' if model == SONNET else 'openrouter-auth.json'
            for name in ['auth.json', provider_auth, 'campaigns_state.json']:
                source = args.claude_auth if name == 'claude-auth.json' else args.distill_profile / name
                if source.exists():
                    credentials.append(home / name)
                    shutil.copyfile(source, home / name)
                    (home / name).chmod(0o600)
            text = config(model, ablation=ablation)
            (home / 'config.toml').write_text(text)
            result['config_sha256'] = hashlib.sha256(text.encode()).hexdigest()
            command = [str(executable), '--no-leader', '--always-approve', '--disable-web-search',
                       '--cwd', str(work), '--session-id', session, '--prompt-file', str(out / 'prompt.txt'),
                       '--output-format', 'streaming-json', '--model', model,
                       '--tools', 'run_terminal_cmd' if ablation else LOCAL_TOOLS]
            if ablation:
                command.extend(['--effort', 'low'])
        (out / 'command.json').write_text(json.dumps(command, indent=2) + '\n')
        started = time.monotonic()
        with (out / 'stdout.txt').open('w') as stdout, (out / 'stderr.txt').open('w') as stderr:
            proc = subprocess.Popen(command, cwd=work, env=env, stdout=stdout, stderr=stderr,
                                    stdin=subprocess.DEVNULL, start_new_session=True)
            tree = OwnedTree(proc)
            try:
                while proc.poll() is None:
                    tree.refresh()
                    if time.monotonic() - started >= args.timeout:
                        result['timed_out'] = True
                        break
                    time.sleep(.02)
            finally:
                result['cleanup_complete'] = tree.stop()
            if not result['cleanup_complete']:
                raise RuntimeError('Inference process tree did not terminate')
        result.update(wall_time_s=round(time.monotonic() - started, 6), exit_code=proc.returncode)
        usage = native_usage(out / 'stdout.txt') if native else provider_usage(home, session)
        result.update(usage)
        observed = result.get('observed_models', [])
        expected = SONNET.removeprefix('claude/') if native else ('deepseek/deepseek-v4.1-flash' if model == DEEPSEEK else model.removeprefix('claude/'))
        result['model_verified'] = bool(observed) and all(m == expected for m in observed)
    except Exception as error:
        result['error'] = f'{type(error).__name__}: {error}'
    finally:
        if proc is not None and proc.poll() is None:
            _terminate_owned_group(proc)
        for path in credentials:
            path.unlink(missing_ok=True)
        if started is not None and result['wall_time_s'] == 0:
            result['wall_time_s'] = round(time.monotonic() - started, 6)
    if not result['cleanup_complete']:
        result['passed'] = False
        result['grader_passed'] = False
        (out / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
        return 1
    grade = subprocess.run(['python3', str(ROOT / case['grader']['script_ref']), str(work)], capture_output=True, text=True, timeout=60)
    (out / 'grader.txt').write_text(grade.stdout + grade.stderr)
    (out / 'diff.patch').write_text(subprocess.check_output(['git', 'diff', '--no-ext-diff'], cwd=work, text=True))
    result['grader_passed'] = grade.returncode == 0
    result['frozen_inputs_unchanged'] = frozen == {'fixture_sha256': _sha256_tree(ROOT / case['fixture_ref']), 'grader_sha256': _sha256_file(ROOT / case['grader']['script_ref'])}
    result['passed'] = (result['exit_code'] == 0 and not result['timed_out'] and result['grader_passed']
                        and result.get('model_verified', False) and result['frozen_inputs_unchanged']
                        and not result.get('native_is_error', False))
    (out / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({k:v for k,v in result.items() if k != 'call_usage'}), flush=True)
    return 0 if result['passed'] else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--variant', choices=('sonnet-distill', 'claude-native', 'deepseek-distill', 'deepseek-same-agent-ablation'), required=True)
    parser.add_argument('--case', required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--distill', type=Path, default=Path(shutil.which('distill')))
    parser.add_argument('--claude', type=Path, default=Path(shutil.which('claude')))
    parser.add_argument('--distill-profile', type=Path, default=Path.home() / '.distill')
    parser.add_argument('--claude-auth', type=Path, default=Path.home() / '.distill/claude-auth.json')
    parser.add_argument('--timeout', type=float, default=300)
    return run(parser.parse_args())


if __name__ == '__main__':
    raise SystemExit(main())
