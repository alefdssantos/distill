#!/usr/bin/env python3
"""Focused tests for scripts/distill-local-launch.py."""

from __future__ import annotations

import importlib.util
import os
import signal
import socket
import stat
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from pathlib import Path
from unittest import mock


SCRIPT = Path(__file__).with_name("distill-local-launch.py")
_spec = importlib.util.spec_from_file_location("distill_local_launch", SCRIPT)
launch = importlib.util.module_from_spec(_spec)
sys.modules[_spec.name] = launch
_spec.loader.exec_module(launch)


FAKE_SERVE_MODEL = """\
#!/usr/bin/env python3
import json
import os
import signal
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

name = sys.argv[1]
root = os.path.dirname(os.path.abspath(__file__))
with open(os.path.join(root, "serve-model.calls"), "a", encoding="utf-8") as calls:
    calls.write(name + "\\n")
with open(os.path.join(root, "pid-" + name), "w", encoding="utf-8") as pid_file:
    pid_file.write(str(os.getpid()))

if name == "qwen" and os.environ.get("FAKE_QWEN_FAIL"):
    sys.exit(5)

class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == "/v1/models":
            body = json.dumps({"data": []}).encode("utf-8")
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
        else:
            self.send_error(404)

    def log_message(self, *args):
        pass

port = int(os.environ["FAKE_PORT_" + name.upper()])
server = ThreadingHTTPServer(("127.0.0.1", port), Handler)
signal.signal(signal.SIGTERM, lambda *_: sys.exit(0))
server.serve_forever()
"""

FAKE_APP = """\
#!/usr/bin/env python3
import os
import signal
import sys
import time

with open(__file__ + ".called", "w", encoding="utf-8") as called:
    called.write(" ".join(sys.argv[1:]))
if os.environ.get("FAKE_APP_IGNORE_SIGNALS"):
    for signum in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        signal.signal(signum, lambda *_: None)
time.sleep(float(os.environ.get("FAKE_APP_SLEEP", "0")))
sys.exit(int(os.environ.get("FAKE_APP_EXIT", "0")))
"""


def free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def write_executable(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")
    path.chmod(path.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)


def make_runtime(root: Path) -> Path:
    runtime = root / "runtime"
    (runtime / "lib").mkdir(parents=True)
    (runtime / "mlx-env" / "bin").mkdir(parents=True)
    (runtime / "models" / launch.QWEN.model_dir).mkdir(parents=True)
    (runtime / "models" / launch.LFM.model_dir).mkdir(parents=True)
    (runtime / "profile").mkdir()
    write_executable(runtime / "lib" / "distill", FAKE_APP)
    write_executable(runtime / "serve-model", FAKE_SERVE_MODEL)
    write_executable(runtime / "mlx-env" / "bin" / "python", "#!/bin/sh\nexit 0\n")
    write_executable(runtime / "mlx-env" / "bin" / "mlx_lm.server", "#!/bin/sh\nexit 0\n")
    return runtime


def pid_alive(pid: int) -> bool:
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    return True


def fixture_env(runtime: Path, **overrides: str) -> dict[str, str]:
    env = dict(
        os.environ,
        DISTILL_HOME=str(runtime / "profile"),
    )
    env.update(overrides)
    return env


def endpoints_for(ports: dict[str, int]) -> list[launch.Endpoint]:
    return [
        launch.Endpoint(
            "qwen",
            ports["QWEN"],
            f"http://127.0.0.1:{ports['QWEN']}/v1",
            launch.QWEN.model_dir,
        ),
        launch.Endpoint(
            "lfm",
            ports["LFM"],
            f"http://127.0.0.1:{ports['LFM']}/v1",
            launch.LFM.model_dir,
        ),
    ]


def wait_until_dead(pid: int, timeout: float = 3.0) -> bool:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if not pid_alive(pid):
            return True
        time.sleep(0.02)
    return not pid_alive(pid)


class SelectionTests(unittest.TestCase):
    def test_local_decision_and_local_model_select_qwen_and_lfm(self):
        config = {
            "jev": {
                "local_decision": {
                    "base_url": "http://127.0.0.1:8081/v1",
                    "model": "default_model",
                },
                "local": {"model": "lfm-local"},
            },
            "model": {"lfm-local": {"base_url": "http://127.0.0.1:8082/v1/"}},
        }
        self.assertEqual(
            [endpoint.name for endpoint in launch.selected_endpoints(config)],
            ["qwen", "lfm"],
        )

    def test_local_chat_provider_selects_qwen(self):
        config = {
            "jev": {
                "provider": "local-chat",
                "base_url": "http://127.0.0.1:8081/v1",
            }
        }
        self.assertEqual(
            [endpoint.name for endpoint in launch.selected_endpoints(config)],
            ["qwen"],
        )

    def test_no_selection_or_disabled_jev_starts_nothing(self):
        self.assertEqual(launch.selected_endpoints({}), [])
        disabled = {
            "jev": {
                "enabled": False,
                "local_decision": {
                    "base_url": "http://127.0.0.1:8081/v1",
                    "model": "default_model",
                },
                "local": {"model": "lfm-local"},
            },
            "model": {"lfm-local": {"base_url": "http://127.0.0.1:8082/v1"}},
        }
        self.assertEqual(launch.selected_endpoints(disabled), [])

    def test_unknown_endpoint_is_not_started(self):
        config = {
            "jev": {
                "local_decision": {
                    "base_url": "http://127.0.0.1:8080/v1",
                    "model": "default_model",
                }
            }
        }
        self.assertEqual(launch.selected_endpoints(config), [])


class ProcessTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.runtime = make_runtime(self.root)

    def tearDown(self):
        self.temp.cleanup()

    def test_pass_through_arg_execs_cli_without_models(self):
        env = fixture_env(self.runtime, FAKE_APP_EXIT="7")
        result = subprocess.run(
            [sys.executable, str(SCRIPT), str(self.runtime), "version"],
            capture_output=True,
            text=True,
            env=env,
            timeout=20,
        )
        self.assertEqual(result.returncode, 7, result.stderr)
        self.assertEqual((self.runtime / "lib" / "distill.called").read_text(), "version")
        self.assertFalse((self.runtime / "serve-model.calls").exists())

    def test_bare_invocation_without_selection_runs_cli(self):
        env = fixture_env(self.runtime, FAKE_APP_EXIT="3")
        result = subprocess.run(
            [sys.executable, str(SCRIPT), str(self.runtime)],
            capture_output=True,
            text=True,
            env=env,
            timeout=20,
        )
        self.assertEqual(result.returncode, 3, result.stderr)
        self.assertTrue((self.runtime / "lib" / "distill.called").exists())
        self.assertFalse((self.runtime / "serve-model.calls").exists())

    def test_disabled_jev_runs_controls_without_models(self):
        config = (
            "[jev]\nenabled = false\n"
            "[jev.local_decision]\n"
            'base_url = "http://127.0.0.1:8081/v1"\n'
            'model = "default_model"\n'
        )
        (self.runtime / "profile" / "config.toml").write_text(config, encoding="utf-8")
        result = subprocess.run(
            [sys.executable, str(SCRIPT), str(self.runtime)],
            capture_output=True,
            text=True,
            env=fixture_env(self.runtime),
            timeout=20,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((self.runtime / "lib" / "distill.called").exists())
        self.assertFalse((self.runtime / "serve-model.calls").exists())

    def test_second_session_lock_refuses_without_touching_first(self):
        env = fixture_env(self.runtime)
        with launch._session_lock(self.runtime):
            result = subprocess.run(
                [sys.executable, str(SCRIPT), str(self.runtime)],
                capture_output=True,
                text=True,
                env=env,
                timeout=20,
            )
        self.assertEqual(result.returncode, 1)
        self.assertIn("another managed distill-local session", result.stderr)
        self.assertFalse((self.runtime / "lib" / "distill.called").exists())

    def test_bound_port_fails_before_spawning_models(self):
        port = free_port()
        endpoint = launch.Endpoint(
            "qwen", port, f"http://127.0.0.1:{port}/v1", launch.QWEN.model_dir
        )
        listener = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        listener.bind(("127.0.0.1", endpoint.port))
        listener.listen()
        try:
            with self.assertRaises(launch.LaunchError) as caught:
                launch.run_managed(self.runtime, [], [endpoint], startup_timeout=5)
        finally:
            listener.close()
        self.assertIn("already in use", str(caught.exception))
        self.assertFalse((self.runtime / "serve-model.calls").exists())

    def test_readiness_listener_must_belong_to_expected_live_pid(self):
        port = free_port()
        binder = subprocess.Popen(
            [
                sys.executable,
                "-c",
                (
                    "import socket,sys,time;"
                    "s=socket.socket();"
                    "s.bind(('127.0.0.1',int(sys.argv[1])));"
                    "s.listen();"
                    "time.sleep(30)"
                ),
                str(port),
            ]
        )
        unrelated = subprocess.Popen(["sleep", "30"])
        try:
            deadline = time.monotonic() + 3.0
            while not launch._listener_owned_by(binder, port):
                if time.monotonic() >= deadline:
                    self.fail("fixture listener did not appear")
                time.sleep(0.05)
            self.assertTrue(launch._listener_owned_by(binder, port))
            self.assertFalse(launch._listener_owned_by(unrelated, port))
        finally:
            for proc in (binder, unrelated):
                if proc.poll() is None:
                    proc.terminate()
                    proc.wait(timeout=5)

    def test_startup_failure_cleans_partial_start(self):
        ports = {"QWEN": free_port(), "LFM": free_port()}
        endpoints = endpoints_for(ports)
        env = fixture_env(
            self.runtime,
            FAKE_PORT_QWEN=str(ports["QWEN"]),
            FAKE_PORT_LFM=str(ports["LFM"]),
            FAKE_QWEN_FAIL="1",
        )
        with mock.patch.dict(os.environ, env):
            with self.assertRaises(launch.LaunchError) as caught:
                launch.run_managed(self.runtime, [], endpoints, startup_timeout=10)

        self.assertIn("qwen model server exited during startup", str(caught.exception))
        self.assertFalse((self.runtime / "lib" / "distill.called").exists())
        lfm_pid = int((self.runtime / "pid-lfm").read_text())
        self.assertTrue(wait_until_dead(lfm_pid), "partial lfm start was not cleaned up")

    def test_sigterm_stops_models_and_app(self):
        ports = {"QWEN": free_port(), "LFM": free_port()}
        endpoints = endpoints_for(ports)
        env = fixture_env(
            self.runtime,
            FAKE_PORT_QWEN=str(ports["QWEN"]),
            FAKE_PORT_LFM=str(ports["LFM"]),
            FAKE_APP_SLEEP="30",
        )
        timer = threading.Timer(1.0, lambda: os.kill(os.getpid(), signal.SIGTERM))
        with (
            mock.patch.dict(os.environ, env),
            mock.patch.object(launch, "APP_SIGNAL_GRACE_SECONDS", 1.0),
        ):
            timer.start()
            try:
                status = launch.run_managed(self.runtime, [], endpoints, startup_timeout=10)
            finally:
                timer.cancel()

        self.assertEqual(status, 143)
        for name in ("qwen", "lfm"):
            pid = int((self.runtime / f"pid-{name}").read_text())
            self.assertTrue(wait_until_dead(pid), f"{name} survived SIGTERM cleanup")

    def test_signal_grace_kills_app_that_ignores_it(self):
        ports = {"QWEN": free_port(), "LFM": free_port()}
        endpoints = endpoints_for(ports)
        env = fixture_env(
            self.runtime,
            FAKE_PORT_QWEN=str(ports["QWEN"]),
            FAKE_PORT_LFM=str(ports["LFM"]),
            FAKE_APP_SLEEP="30",
            FAKE_APP_IGNORE_SIGNALS="1",
        )
        timer = threading.Timer(1.0, lambda: os.kill(os.getpid(), signal.SIGTERM))
        with (
            mock.patch.dict(os.environ, env),
            mock.patch.object(launch, "APP_SIGNAL_GRACE_SECONDS", 0.5),
        ):
            timer.start()
            try:
                status = launch.run_managed(self.runtime, [], endpoints, startup_timeout=10)
            finally:
                timer.cancel()

        self.assertEqual(status, 137)
        for name in ("qwen", "lfm"):
            pid = int((self.runtime / f"pid-{name}").read_text())
            self.assertTrue(wait_until_dead(pid), f"{name} survived SIGTERM cleanup")

    def test_shutdown_stops_only_owned_models_and_preserves_app_status(self):
        ports = {"QWEN": free_port(), "LFM": free_port()}
        endpoints = endpoints_for(ports)
        unrelated = subprocess.Popen(["sleep", "30"])
        env = fixture_env(
            self.runtime,
            FAKE_PORT_QWEN=str(ports["QWEN"]),
            FAKE_PORT_LFM=str(ports["LFM"]),
            FAKE_APP_EXIT="9",
            FAKE_APP_SLEEP="0.2",
        )
        try:
            with mock.patch.dict(os.environ, env):
                status = launch.run_managed(self.runtime, [], endpoints, startup_timeout=10)

            self.assertEqual(status, 9)
            for name in ("qwen", "lfm"):
                pid = int((self.runtime / f"pid-{name}").read_text())
                self.assertTrue(wait_until_dead(pid), f"{name} survived app exit")
            self.assertIsNone(unrelated.poll(), "unrelated process was killed")
        finally:
            if unrelated.poll() is None:
                unrelated.terminate()
                unrelated.wait(timeout=5)


if __name__ == "__main__":
    unittest.main()
