# Accounts and local models

The home menu has separate login and logout actions for Grok, ChatGPT, Claude,
and OpenRouter. Signing out of one does not sign out of the others. ChatGPT uses
OAuth and the model catalog available to your account. Claude uses your Pro/Max
subscription through OAuth (`distill login --claude`): the account's models appear in `/model` as
`claude/<id>`, and any `[model.*]` entry whose `base_url` is
`https://api.anthropic.com/v1` with no API key uses the same sign-in.
OpenRouter supports
browser login or an `OPENROUTER_API_KEY` environment variable.

For a local model, start an OpenAI-compatible server and add an entry to your
profile's `config.toml`. Replace the model ID and URL with your server's values:

```toml
[models]
default = "local"

[model.local]
name = "Local model"
model = "your-local-model-id"
base_url = "http://127.0.0.1:8000/v1"
api_backend = "chat_completions"
```

New profiles use `~/.distill`. Set `DISTILL_HOME` to use another directory. An
existing `~/.grok` profile remains supported when no new profile exists, so
previous credentials and sessions remain accessible. An explicit
`DISTILL_HOME` takes priority.

## Local decision model

The decision layer can talk to a local OpenAI-compatible chat server instead of
the remote Jev service. The server must be on an explicit loopback address and
serve `POST /v1/chat/completions`; `localhost`, remote hosts, redirects, proxies,
embedded credentials, queries, and fragments are rejected. A local decision
model needs no API key.

There are two ways to select it.

In the tier editor (`/tiers`, or the footer `change` link), choose
**Decision model** and pick a local catalog entry. The editor lists only
`[model.<id>]` entries that use `api_backend = "chat_completions"`, declare no
`api_key`, `env_key`, or `auth_provider`, and have a validated loopback
`base_url`. The pick is saved to the user profile as:

```toml
[jev.local_decision]
base_url = "http://127.0.0.1:8080/v1"
model = "Qwen3.5-2B-4bit"
```

Choosing **Jev remoto** removes that table, so the previous `[jev]` remote
settings apply again. If the user profile also sets `[jev] provider =
"local_chat"`, the picker removes `provider`, `base_url`, and `model` together;
other profile settings, including the ladder, are preserved. Managed files are
read-only, so a managed `local_chat` transport still wins over the picker.

An explicit configuration works without the picker:

```toml
[jev]
provider = "local_chat"
base_url = "http://127.0.0.1:8080/v1"
model = "Qwen3.5-2B-4bit"
```

Both fields are required. The picker table, when present, wins over the explicit
`[jev]` transport. Provider, URL, model, and activation are read only from the
trusted base layers (managed, user, requirements) and never from project
configuration; restart the session to apply a change.

Local decision calls send no reasoning parameter, no credential, and no proxy.
The local model serves every enabled lever; `[jev.models]` aliases and the
shipped per-lever remote defaults do not move it. The active model name appears
in the prompt footer. The value is configuration, not a reachability probe: a
configured server that is down still reads as selected.

Local chat answers are text-only. Images are refused with
`jev_images_unsupported`. Confidence and probabilities reported by a local chat
model are self-attributed, not calibrated, so the harness clears them; levers
that need that signal defer to their existing conservative path. Local answers
never authorize tool dispatch. A network failure, timeout, truncated answer, or
unsupported request keeps the conservative path and does not call the remote Jev
service. To go back to remote Jev, remove `[jev.local_decision]` (or pick **Jev
remoto**) and restart; `[jev] enabled = false` remains the kill switch.
