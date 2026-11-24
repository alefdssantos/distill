#!/usr/bin/env python3
"""Run Distill with the local MLX models its profile selects.

Usage: distill-local-launch.py RUNTIME_ROOT [DISTILL_ARG ...]

RUNTIME_ROOT is the absolute runtime directory that contains ``lib/distill``,
``serve-model``, ``mlx-env`` and ``models``. The launcher owns ``DISTILL_HOME``;
this script leaves the working directory and environment unchanged.

With no DISTILL_ARG the supervisor starts only the known endpoints selected by
the profile, runs ``RUNTIME_ROOT/lib/distill`` with inherited stdio, and stops
only the model processes it started. Any DISTILL_ARG (``login``, ``version``,
``-p`` ...) skips supervision and execs the CLI directly. ``login --gemini``
and ``models --gemini`` instead exec the installed ``agy`` CLI.

The runtime contract is one ``serve-model`` process that execs one
``mlx_lm.server``; the supervisor signals only that direct child.
"""

from __future__ import annotations

import contextlib
import errno
import fcntl
import http.client
import os
import shutil
import signal
import socket
import subprocess
import sys
import time
import tomllib
from dataclasses import dataclass
from pathlib import Path
from urllib.error import HTTPError, URLError
from urllib.parse import urlsplit
from urllib.request import HTTPRedirectHandler, ProxyHandler, Request, build_opener


STARTUP_TIMEOUT_SECONDS = 120.0
READY_TIMEOUT_SECONDS = 1.0
POLL_INTERVAL_SECONDS = 0.2
MODEL_STOP_GRACE_SECONDS = 5.0
APP_SIGNAL_GRACE_SECONDS = 10.0


@dataclass(frozen=True)
class Endpoint:
    name: str
    port: int
    base_url: str
    model_dir: str


QWEN = Endpoint("qwen", 8081, "http://127.0.0.1:8081/v1", "Qwen3.5-2B-4bit")
LFM = Endpoint("lfm", 8082, "http://127.0.0.1:8082/v1", "LFM2.5-1.2B-Instruct-MLX-4bit")
ENDPOINTS = (QWEN, LFM)

# Exact Gemini commands routed to the official CLI; other arguments stay untouched.
GEMINI_ROUTES = {
    ("login", "--gemini"): ("acp", "--login"),
    ("models", "--gemini"): ("models",),
}


class LaunchError(Exception):
    """A startup failure whose message is safe to show the user."""


class _SignalExit(BaseException):
    def __init__(self, signum: int):
        super().__init__(signum)
        self.signum = signum


class _State:
    app: subprocess.Popen[bytes] | None = None
    signum: int | None = None


class _NoRedirect(HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


def _normalize_base_url(value: object) -> str | None:
    if not isinstance(value, str):
        return None
    raw = value.strip()
    if not raw:
        return None
    try:
        parts = urlsplit(raw)
        port = parts.port
    except ValueError:
        return None
    if parts.scheme.lower() not in ("http", "https"):
        return None
    if parts.username or parts.password or parts.query or parts.fragment:
        return None
    if parts.hostname is None or port is None:
        return None
    path = parts.path.rstrip("/")
    return f"{parts.scheme.lower()}://{parts.hostname.lower()}:{port}{path}"


def _endpoint_for_url(value: object, role: str) -> Endpoint | None:
    normalized = _normalize_base_url(value)
    if normalized is None:
        return None
    for endpoint in ENDPOINTS:
        if endpoint.name == role and normalized == endpoint.base_url:
            return endpoint
    return None


def _local_chat_requested(jev: dict) -> bool:
    provider = jev.get("provider")
    if not isinstance(provider, str):
        return False
    return provider.strip().lower().replace("-", "_") in ("local_chat", "local")


def selected_endpoints(config: dict) -> list[Endpoint]:
    """The known local endpoints selected by the trusted profile settings."""
    jev = config.get("jev")
    if not isinstance(jev, dict) or jev.get("enabled") is False:
        return []

    selected: list[Endpoint] = []

    decision = jev.get("local_decision")
    if isinstance(decision, dict):
        endpoint = _endpoint_for_url(decision.get("base_url"), "qwen")
    elif _local_chat_requested(jev):
        endpoint = _endpoint_for_url(jev.get("base_url"), "qwen")
    else:
        endpoint = None
    if endpoint is not None:
        selected.append(endpoint)

    local = jev.get("local")
    models = config.get("model")
    if isinstance(local, dict) and isinstance(models, dict):
        model_id = local.get("model")
        entry = models.get(model_id) if isinstance(model_id, str) else None
        if isinstance(entry, dict):
            endpoint = _endpoint_for_url(entry.get("base_url"), "lfm")
            if endpoint is not None and endpoint not in selected:
                selected.append(endpoint)

    return selected


def _config_path(runtime: Path) -> Path:
    home = os.environ.get("DISTILL_HOME")
    profile = Path(home) if home else runtime / "profile"
    return profile / "config.toml"


def _load_config(runtime: Path) -> dict:
    path = _config_path(runtime)
    try:
        with path.open("rb") as handle:
            return tomllib.load(handle)
    except FileNotFoundError:
        return {}
    except OSError as error:
        raise LaunchError(f"cannot read profile config {path}: {error.strerror}") from error
    except tomllib.TOMLDecodeError as error:
        raise LaunchError(f"cannot parse profile config {path}: {error}") from error


def _require_executable(path: Path, label: str) -> None:
    if not path.is_file():
        raise LaunchError(f"missing {label}: {path}; reinstall the runtime")
    if not os.access(path, os.X_OK):
        raise LaunchError(f"{label} is not executable: {path}")


def _validate_runtime(runtime: Path, endpoints: tuple[Endpoint, ...] | list[Endpoint]) -> None:
    _require_executable(runtime / "lib" / "distill", "runtime CLI")
    if not endpoints:
        return
    _require_executable(runtime / "serve-model", "runtime model launcher")
    _require_executable(runtime / "mlx-env" / "bin" / "python", "runtime MLX Python")
    _require_executable(runtime / "mlx-env" / "bin" / "mlx_lm.server", "runtime MLX server")
    for endpoint in endpoints:
        model_path = runtime / "models" / endpoint.model_dir
        if not model_path.is_dir():
            raise LaunchError(f"missing model weights: {model_path}; reinstall the runtime")


def _ensure_ports_free(endpoints: tuple[Endpoint, ...] | list[Endpoint]) -> None:
    for endpoint in endpoints:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        try:
            sock.bind(("127.0.0.1", endpoint.port))
        except OSError as error:
            raise LaunchError(
                f"port {endpoint.port} ({endpoint.name}) is already in use: {error.strerror}. "
                "Stop that server or close the other distill-local session, then retry. "
                "No models were started."
            ) from error
        finally:
            sock.close()


def _listener_owned_by(proc: subprocess.Popen, port: int) -> bool:
    try:
        result = subprocess.run(
            [
                "lsof",
                "-a",
                "-p",
                str(proc.pid),
                "-nP",
                f"-iTCP:{port}",
                "-sTCP:LISTEN",
                "-Fp",
            ],
            capture_output=True,
            text=True,
            timeout=2,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired):
        return False
    return result.returncode == 0 and f"p{proc.pid}" in result.stdout.splitlines()


def _model_ready(endpoint: Endpoint, proc: subprocess.Popen) -> bool:
    expected_pid = proc.pid
    if proc.poll() is not None:
        return False
    opener = build_opener(ProxyHandler({}), _NoRedirect())
    request = Request(f"{endpoint.base_url}/models", headers={"Accept": "application/json"})
    try:
        with opener.open(request, timeout=READY_TIMEOUT_SECONDS) as response:
            if not 200 <= response.status < 300:
                return False
    except (OSError, URLError, HTTPError, http.client.HTTPException):
        return False
    return (
        proc.poll() is None
        and proc.pid == expected_pid
        and _listener_owned_by(proc, endpoint.port)
    )


def _spawn_models(runtime: Path, endpoints: list[Endpoint]) -> dict[Endpoint, subprocess.Popen]:
    procs: dict[Endpoint, subprocess.Popen] = {}
    try:
        for endpoint in endpoints:
            procs[endpoint] = subprocess.Popen(
                [str(runtime / "serve-model"), endpoint.name],
                stdin=subprocess.DEVNULL,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                start_new_session=True,
                close_fds=True,
            )
    except OSError as error:
        _stop_models(list(procs.values()))
        raise LaunchError(f"cannot start local model server: {error}") from error
    return procs


def _stop_models(procs: list[subprocess.Popen]) -> None:
    # serve-model execs one mlx_lm.server process with no worker subprocesses.
    # Signal only that known direct child, never a process group.
    owned = [proc for proc in procs if proc.poll() is None]
    for proc in owned:
        _signal_model(proc, signal.SIGTERM)

    deadline = time.monotonic() + MODEL_STOP_GRACE_SECONDS
    while owned:
        owned = [proc for proc in owned if proc.poll() is None]
        if not owned or time.monotonic() >= deadline:
            break
        time.sleep(0.05)

    for proc in owned:
        _signal_model(proc, signal.SIGKILL)
    for proc in procs:
        try:
            proc.wait(timeout=1.0)
        except subprocess.TimeoutExpired:
            pass


def _signal_model(proc: subprocess.Popen, signum: int) -> None:
    try:
        proc.send_signal(signum)
    except (ProcessLookupError, PermissionError):
        pass


def _start_models(
    runtime: Path,
    endpoints: list[Endpoint],
    state: _State,
    startup_timeout: float,
) -> dict[Endpoint, subprocess.Popen]:
    if state.signum is not None:
        raise _SignalExit(state.signum)
    procs = _spawn_models(runtime, endpoints)
    try:
        pending = list(endpoints)
        deadline = time.monotonic() + startup_timeout
        while pending:
            if state.signum is not None:
                raise _SignalExit(state.signum)
            for endpoint in list(pending):
                proc = procs[endpoint]
                status = proc.poll()
                if status is not None:
                    raise LaunchError(
                        f"{endpoint.name} model server exited during startup "
                        f"(status {_child_status(status)}); "
                        f"command: {runtime / 'serve-model'} {endpoint.name}"
                    )
                if _model_ready(endpoint, proc):
                    pending.remove(endpoint)
            if not pending:
                break
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                waiting = ", ".join(f"{ep.name} (127.0.0.1:{ep.port})" for ep in pending)
                raise LaunchError(
                    f"timed out after {startup_timeout:g}s waiting for {waiting}; "
                    f"check {runtime / 'serve-model'} <qwen|lfm>"
                )
            time.sleep(min(POLL_INTERVAL_SECONDS, remaining))
        return procs
    except BaseException:
        _stop_models(list(procs.values()))
        raise


def _child_status(status: int | None) -> int:
    if status is None:
        return 1
    return 128 - status if status < 0 else status


def _wait_for_app(app: subprocess.Popen, state: _State) -> int:
    forwarded = False
    signal_deadline: float | None = None
    while True:
        if state.signum is not None and not forwarded:
            # The app handle is registered before this normal-flow check.
            try:
                if app.poll() is None:
                    app.send_signal(state.signum)
            except (OSError, ProcessLookupError):
                pass
            forwarded = True
            signal_deadline = time.monotonic() + APP_SIGNAL_GRACE_SECONDS
        try:
            return app.wait(timeout=POLL_INTERVAL_SECONDS)
        except subprocess.TimeoutExpired:
            if (
                state.signum is not None
                and forwarded
                and signal_deadline is not None
                and time.monotonic() >= signal_deadline
            ):
                app.kill()


@contextlib.contextmanager
def _signal_handlers(state: _State):
    previous: dict[int, object] = {}

    def handler(signum, frame):
        # Only record here; forwarding happens in normal flow with handles known.
        state.signum = signum

    for signum in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        previous[signum] = signal.getsignal(signum)
        signal.signal(signum, handler)
    try:
        yield
    finally:
        for signum, old_handler in previous.items():
            signal.signal(signum, old_handler)


def run_managed(
    runtime: Path,
    app_args: list[str],
    endpoints: list[Endpoint],
    startup_timeout: float = STARTUP_TIMEOUT_SECONDS,
) -> int:
    """Start the selected models, run the app, and return its exit status."""
    _validate_runtime(runtime, endpoints)
    _ensure_ports_free(endpoints)
    state = _State()
    procs: dict[Endpoint, subprocess.Popen] = {}
    with _signal_handlers(state):
        try:
            if endpoints:
                labels = ", ".join(f"{ep.name} on 127.0.0.1:{ep.port}" for ep in endpoints)
                print(f"distill-local: starting {labels}", file=sys.stderr, flush=True)
                procs = _start_models(runtime, endpoints, state, startup_timeout)
                if state.signum is not None:
                    raise _SignalExit(state.signum)
                print("distill-local: local models ready", file=sys.stderr, flush=True)

            app = runtime / "lib" / "distill"
            try:
                state.app = subprocess.Popen([str(app), *app_args])
            except OSError as error:
                raise LaunchError(f"cannot start runtime CLI {app}: {error}") from error
            try:
                status = _wait_for_app(state.app, state)
            finally:
                state.app = None
            return _child_status(status)
        finally:
            if procs:
                _stop_models(list(procs.values()))


@contextlib.contextmanager
def _session_lock(runtime: Path):
    path = runtime / ".distill-local-lifecycle.lock"
    try:
        fd = os.open(path, os.O_RDWR | os.O_CREAT, 0o600)
    except OSError as error:
        raise LaunchError(f"cannot open session lock {path}: {error.strerror}") from error
    locked = False
    try:
        try:
            fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
            locked = True
        except OSError as error:
            if error.errno in (errno.EACCES, errno.EAGAIN):
                raise LaunchError(
                    f"another managed distill-local session is running ({path}); "
                    "close it before starting another one. No processes were touched."
                ) from error
            raise LaunchError(f"cannot lock {path}: {error.strerror}") from error
        yield
    finally:
        if locked:
            try:
                fcntl.flock(fd, fcntl.LOCK_UN)
            except OSError:
                pass
        os.close(fd)


def _exec_gemini(args: tuple[str, ...]) -> None:
    agy = shutil.which("agy")
    if agy is None:
        # The installer puts agy here without adding it to every interactive PATH.
        local = Path.home() / ".local" / "bin" / "agy"
        if os.access(local, os.X_OK):
            agy = str(local)
    if agy is None:
        raise LaunchError(
            'missing Gemini CLI "agy"; install it and put agy on PATH '
            "or in ~/.local/bin, then retry"
        )
    os.execv(agy, ["agy", *args])


def main(argv: list[str] | None = None) -> int:
    argv = sys.argv if argv is None else argv
    if len(argv) < 2:
        print(
            "usage: distill-local-launch.py RUNTIME_ROOT [DISTILL_ARG ...]",
            file=sys.stderr,
        )
        return 2

    runtime = Path(argv[1])
    app_args = argv[2:]
    try:
        if not runtime.is_absolute():
            raise LaunchError(f"runtime path must be absolute: {runtime}")
        if not runtime.is_dir():
            raise LaunchError(f"runtime directory does not exist: {runtime}")

        if app_args:
            gemini_args = GEMINI_ROUTES.get(tuple(app_args))
            if gemini_args is not None:
                _exec_gemini(gemini_args)
            app = runtime / "lib" / "distill"
            _require_executable(app, "runtime CLI")
            os.execv(str(app), [str(app), *app_args])

        endpoints = selected_endpoints(_load_config(runtime))
        with _session_lock(runtime):
            return run_managed(runtime, [], endpoints)
    except LaunchError as error:
        print(f"distill-local: {error}", file=sys.stderr)
        return 1
    except _SignalExit as error:
        print(
            f"distill-local: startup interrupted by signal {error.signum}; "
            "local models stopped",
            file=sys.stderr,
        )
        return 128 + error.signum
    except KeyboardInterrupt:
        print("distill-local: interrupted; local models stopped", file=sys.stderr)
        return 130


if __name__ == "__main__":
    sys.exit(main())
