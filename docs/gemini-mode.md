# Gemini through the official CLI

`distill-local --gemini` opens the Distill terminal UI against the installed
official Antigravity CLI (`agy acp --stdio --sandbox --disable-telemetry`).
Gemini runs in its own ACP session and keeps its own sign-in and permission
prompts. This mode never joins a Distill leader and never forces an xAI login.

## Sign in

Use either form:

```sh
distill-local login --gemini
```

or, inside Distill, run `/login-gemini`. Both suspend the terminal UI and run
`agy acp --login`. Install the official CLI first; Distill does not collect or
store Gemini credentials.

## Models

The Gemini agent returns its model catalog in the ACP session response. Gemini
mode opens that session at launch, before the first prompt, so `/model` is
available immediately. Run `/model` inside the Gemini session to list the
catalog and switch with the standard ACP `session/set_model` request.

A Gemini session is separate from regular Distill sessions. Its model choice
does not change the models used by regular Distill sessions, and regular
Distill models and routing are unchanged by `--gemini`.

## Usage

Distill shows the per-turn token counts that Gemini reports in its prompt
response. It shows no quota bar or remaining-quota estimate for Gemini:
supported ACP does not expose that quota, so Distill does not invent or fake
one. Check plan limits in the official Gemini CLI or Google account UI.
