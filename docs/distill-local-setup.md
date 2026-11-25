# Run Distill with the bundled local model

This branch adds `scripts/distill-local-launch.py`, a supervisor that starts the
local MLX model server, runs Distill with it, and stops only the model process it
started.

## Requirements

- Mac with Apple Silicon (M1 or newer)
- macOS 26 or newer (the `mlx` wheel is tagged `macosx_26_0_arm64`)
- About 3 GB free disk space
- Rust toolchain and `uv`

## 1. Clone and build

```sh
git clone -b distill-local https://github.com/alefdssantos/distill.git
cd distill
cargo build --release --bin distill
```

## 2. Create the runtime directory

The launcher expects this layout:

```text
runtime/
  lib/distill
  lib/distill-local-launch.py
  mlx-env/
  models/Qwen3.5-2B-4bit/
  hf-cache/hub/
  serve-model
  profile/config.toml
```

Create it and copy the built files:

```sh
ROOT="$HOME/distill-local/runtime"
mkdir -p "$ROOT/lib" "$ROOT/models" "$ROOT/hf-cache/hub" "$ROOT/profile"
cp target/release/distill "$ROOT/lib/distill"
cp scripts/distill-local-launch.py "$ROOT/lib/distill-local-launch.py"
```

## 3. Install MLX and download the model

```sh
uv venv --python 3.14 "$ROOT/mlx-env"
uv pip install --python "$ROOT/mlx-env/bin/python" mlx-lm==0.32.2
"$ROOT/mlx-env/bin/hf" download mlx-community/Qwen3.5-2B-4bit \
  --local-dir "$ROOT/models/Qwen3.5-2B-4bit"
```

## 4. Create the model server

Write `$ROOT/serve-model`:

```sh
#!/bin/sh
set -eu
root=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
export HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1 HF_HOME="$root/hf-cache"
exec "$root/mlx-env/bin/mlx_lm.server" \
  --model "$root/models/Qwen3.5-2B-4bit" \
  --host 127.0.0.1 --port 8081 \
  --chat-template-args '{"enable_thinking":false}'
```

```sh
chmod +x "$ROOT/serve-model"
```

## 5. Create the profile

Write `$ROOT/profile/config.toml`:

```toml
[models]
default = "qwen-local"
worker = ""

[jev]
enabled = true
provider = "local_chat"
base_url = "http://127.0.0.1:8081/v1"
model = "qwen-local"
effort_auto = false

[model.qwen-local]
name = "Qwen3.5-2B"
model = "default_model"
base_url = "http://127.0.0.1:8081/v1"
api_backend = "chat_completions"
```

## 6. Create the launcher command

```sh
mkdir -p "$HOME/.local/bin"
cat > "$HOME/.local/bin/distill-local" <<WRAPPER
#!/bin/sh
export GROK_DISABLE_AUTOUPDATER=1
unset GROK_LEADER_SOCKET
exec "$ROOT/mlx-env/bin/python" "$ROOT/lib/distill-local-launch.py" "$ROOT" "\$@"
WRAPPER
chmod +x "$HOME/.local/bin/distill-local"
```

Run:

```sh
~/.local/bin/distill-local
```

Without arguments the launcher starts the local model and opens the interface.
With `-p`, `login`, or another CLI argument it skips supervision and runs the
CLI directly, so the local model must already be running.
