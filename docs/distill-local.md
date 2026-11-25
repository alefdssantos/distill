# Local model support in this fork

This document describes the changes on the `distill-local` branch and the
measurements taken on the local runtime. Setup steps are in
[distill-local-setup.md](distill-local-setup.md).

## What this branch adds

- Optional selection of a local OpenAI-compatible model for the decision and
  utility layers. The picker accepts only loopback `chat_completions` entries
  with no credential.
- `scripts/distill-local-launch.py`, a supervisor that starts only the local
  endpoints selected by the profile, runs the Distill CLI, forwards signals, and
  stops only the model processes it started.
- `scripts/test_distill_local_launch.py`, covering endpoint selection, startup
  failure, signal handling, port conflicts, and process cleanup.
- Local replacement token metering in the session header, so local usage is
  visible instead of being counted as remote savings.
- Reported usage from rejected local decisions is counted.
- The local meter is labelled as workload, not savings.
- `docs/local-models.md` documents local decision configuration.
- `docs/distill-local-setup.md` documents clone, build, MLX, model download, and
  launcher setup.

Relevant commits, oldest first:

```text
987b8213 Add optional local decision and utility model selection
4229c351 Show local decision model name in terminal status
b5fac9f2 Add local runtime model supervisor launcher
c091050d Show local replacement token meter in session header
137d0b14 Combine local model lifecycle and token meter
3a9795a3 Count reported usage for rejected local decisions
b524566e Label the local token meter as workload, not savings
```

The branch is based on `4da0b4ed` and is 20 commits behind upstream `main` as
of 2026-11-25. A rebase onto the current upstream was attempted but not
published: the external SSD holding the worktree disconnected during the build.

## Verification

- Launcher tests at the branch tip: `20 passed` (`python3 -m pytest -q`).
- The packaged binary reports `distill 2.0.83 (b524566ebda1)`.
- The local Qwen server answered a chat completion with `content: "ok"`.
- The source at the branch tip was not rebuilt after the SSD failure.

## Local inference benchmark

Hardware and software:

- MacBook Air, Apple M4, 10 cores (4 performance, 6 efficiency), 16 GB RAM
- macOS 27.2
- `mlx-lm` 0.32.2
- Model: `mlx-community/Qwen3.5-2B-4bit`
- Server: `mlx_lm.server` on `127.0.0.1:8081`
- `temperature = 0`, `max_tokens = 64`
- Prompt: `Explain in one sentence why local inference matters.` (22 prompt tokens)
- One warmup request excluded

| Run | Wall time | Completion tokens | Output tokens/s |
|---|---:|---:|---:|
| 1 | 0.552 s | 31 | 56.2 |
| 2 | 0.725 s | 31 | 42.8 |
| 3 | 0.526 s | 31 | 58.9 |
| Mean | 0.601 s | 31 | 52.6 |

Warm model load, measured after the model had already run on this machine, was
0.70 s from process start to `/v1/models` responding. Cold load was not measured.

Interpretation limits:

- This measures local inference speed only. It does not measure task quality,
  agent success, or remote API cost.
- The fork has no new task-cost campaign. The existing harness benchmarks in
  [BENCHMARKS.md](../BENCHMARKS.md) were produced upstream and are not changed by
  this branch.
- Results are from one machine and three runs. They are not a general
  performance claim for other Apple Silicon models.
