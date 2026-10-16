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

Latest measured results (2026-10-16). Three distinct comparisons; all arms
passed frozen graders in 9/9 small-task runs. Results do not prove universal
quality equivalence.

| Comparison | Agent | Time | Cost | Passes |
|---|---|---:|---:|---:|
| GPT subscription | Distill GPT-6.1 Sol auto + GPT-6 Luna auto | 258.92 s | $0.20¹ | 9/9 |
|  | Codex GPT-6.1 Sol high | 300.60 s | $0.72¹ | 9/9 |
| Claude Sonnet 5.5 | Distill auto | 97.69 s | $0.185 | 9/9 |
|  | Claude Code medium | 99.69 s | $0.207 | 9/9 |
| DeepSeek V4.1 Flash policy ablation | Distill auto/delegation | 323.15 s | $0.00717 | 9/9 |
|  | Distill low/no delegation | 346.03 s | $0.00796 | 9/9 |

Distill used 72.4% lower modeled cost and 13.9% less time than Codex. With
Claude, it used 10.7% lower modeled cost and 2.0% less time. The DeepSeek
auto/delegation policy used 9.9% lower cost and 6.6% less time than fixed low.

¹ GPT cost is modeled subscription credits converted at a reference $0.04 per
credit, not the subscription bill. Claude uses native list-price API-equivalent
cost; DeepSeek uses native reported per-attempt cost. These different cost bases
are not cross-provider invoice comparisons. The DeepSeek comparison tests two
Distill policies on the same model, not an external DeepSeek competitor.

See [BENCHMARKS.md](BENCHMARKS.md) for methodology, limitations, and receipts.

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
