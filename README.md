<!-- Modified for Distill by Samuel Fajreldines, 2026. -->
# Distill

![distill in action](https://raw.githubusercontent.com/samuelfaj/distill/main/screenshot.png)

Distill is a lightweight coding agent harness and TUI, built to **get far more done
with far fewer tokens.**

It works with Grok and Codex subscriptions, and with any model that speaks the
OpenAI-compatible protocol.

**Steps:**

1. Download distill.
2. Login with openrouter.
3. Login with your subscription.
4. Save money.

----

## Benchmarks

Four requested native-versus-Distill cohorts (2026-10-16), each with three
frozen tasks and three repetitions per arm. Every Distill run enabled external
Jev; its cost includes Jev and child calls. Quality requires a passing unchanged
external behavior grader, successful agent exit, completion within the identical
300-second deadline, and frozen inputs. These small fixtures had no human review
and do not establish universal quality equivalence.

| Native baseline | Distill main + worker | Estimated cost (native → Distill) | Δ cost | Quality (native → Distill) | Δ quality | Time (native → Distill) | Δ time |
|---|---|---:|---:|---:|---:|---:|---:|
| Codex GPT-6 Astra max | GPT-6 Astra auto + GPT-6 Luna auto | $4.07658 → $1.00374 | −75.4% | 9/9 → 9/9 | +0.0% | 440.29 s → 284.99 s | −35.3% |
| Codex GPT-6.1 Sol max | GPT-6.1 Sol auto + GPT-6 Luna auto | $0.79826 → $0.24981 | −68.7% | 9/9 → 9/9 | +0.0% | 384.09 s → 296.60 s | −22.8% |
| Claude Code Opus 5.5 max | Opus 5.5 auto + Haiku 5.5 auto | $2.49456 → $0.34875 | −86.0% | 9/9 → 9/9 | +0.0% | 798.16 s → 191.02 s | −76.1% |
| Codex DeepSeek V4.1 Flash max via OpenRouter Responses | DeepSeek auto + free models (OpenRouter free router) auto | ≥$0.09785 → $0.01177 | ≤−87.9%* | 5/9 → 9/9 | +80.0% | 1973.99 s → 490.27 s | −75.2% |

*DeepSeek native full cost is unknown: three timed-out calls have no usage
receipts. The displayed native cost is a lower bound, not a point estimate; the
cost delta is a conservative upper bound rounded toward zero. Native functional
graders passed 8/9, but only 5/9 completed successfully within the deadline.
The success-rate gain is 44.4 percentage points (+80.0% relative).
Codex used fallback model metadata for this custom DeepSeek integration.

Cost uses standardized model-specific token reference rates, not invoices or
monthly subscription charges. GPT credits convert at a reference $0.04 each.
Claude uses API-equivalent list pricing; native Opus cache-write lifetime is not
exposed. DeepSeek uses the same published OpenRouter rates for both arms; native
Codex actual invoice cost is unavailable. These results do not show pure API
savings. Configured workers were not forced and were not necessarily used on
every task.

See [BENCHMARKS.md](BENCHMARKS.md) for definitions, methodology, exact totals,
cost bounds, reproduction commands, and source JSON.

## Install

### Mac / Linux

```sh
curl -fsSL https://raw.githubusercontent.com/samuelfaj/distill/main/install.sh | sh  
export PATH="$HOME/.local/share/distill/bin:$PATH"                                  
distill --version
```

### Windows

```sh
irm https://raw.githubusercontent.com/samuelfaj/distill/main/install.ps1 | iex        
$bin = "$env:LOCALAPPDATA\distill\bin"; [Environment]::SetEnvironmentVariable('Path', "$bin;" + [Environment]::GetEnvironmentVariable('Path', 'User'), 'User') 
distill --version
```

## Upgrade

For an installation made with the release installer:

```sh
distill update
```

## Choose your models

Open **Model tiers** on the home screen, click **change** beside the current
model, or enter `/tiers`. You can edit each tier in that screen.

| Tier | Purpose | Picker |
|---|---|---|
| Main model | Required. Owns every session: it plans, specifies, delegates, and reviews the work. | `/model` or `/tiers main` |
| Worker model | Optional. Runs every delegated assignment the main model hands it as a precise spec. Without it, the main model does all the work. | `/worker-model` or `/tiers worker` |
| Utility model | Handles bounded tasks such as extraction, summaries, and compression. | `/utility-model` or `/tiers utility` |

Pick your strongest model as the main model and a cheaper one as the worker:
the main model gives the worker as much of the work as it can do well. Each
tier takes an effort level or `auto`, which lets Jev pick the effort for every
call; `auto` is the default. In the Model tiers screen, enter a tier as
`model effort` (for example `gpt-6-luna auto`). You can supply each selection
directly:

```text
/model gpt-6-sol auto
/worker-model gpt-6-luna auto
/utility-model openrouter-qwen37 auto
```

## Always-on behavior

Two rule sets are built into every system prompt, for the main model and for
its subagents, and neither has an off switch:

- **Caveman**: the agent writes terse prose and keeps code, paths and error
  strings exact. `/caveman lite|full|ultra` changes how hard it compresses
  (default `full`); a stored `off` is ignored.
- **Ponytail**: the agent works like a lazy senior developer. Before writing
  code it asks whether the code needs to exist, then reuses what the codebase
  has, then the standard library, then an installed dependency, and only then
  writes the minimum that works. Validation, error handling, security and
  anything you asked for are never cut. Adapted from
  [ponytail](https://github.com/DietrichGebert/ponytail) (MIT).

### Read More:

- [Accounts and local models](docs/local-models.md)
- [How Jev routes work](docs/jev-routing.md)
- [UltraCode activation, hierarchy limits and paired comparisons](docs/ultracode.md)
- [Where the token savings come from](docs/token-saver.md)
