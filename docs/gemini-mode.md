# Gemini through the official CLI

Run `/gemini` inside a normal `distill-local` session to open a Gemini session in
the same terminal, against the installed official Antigravity CLI
(`agy acp --stdio --sandbox --disable-telemetry`). The normal session is kept and
resumes when the Gemini session closes. `distill-local --gemini` remains an
explicit direct entry point.
Gemini runs in its own ACP session and keeps its own sign-in and permission
prompts. This mode never joins a Distill leader and never forces an xAI login.

## Sign in

Inside Distill, run `/login-gemini`. It suspends the terminal UI and runs
`agy acp --login`, then returns to the same session. The terminal command
`distill-local login --gemini` remains available. Install the official CLI first; Distill does not collect or
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
