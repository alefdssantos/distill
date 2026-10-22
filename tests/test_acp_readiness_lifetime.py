"""Keep a real ACP agent alive through its deferred initialization callbacks."""
import json
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import os
from pathlib import Path
import queue
import subprocess
import tempfile
import threading
import time
import unittest


class AcpReadinessLifetimeTest(unittest.TestCase):
    def test_deferred_callbacks_survive_readiness(self):
        binary = Path(os.environ.get('DISTILL_TEST_BINARY', 'target/debug/distill')).resolve()
        class SettingsHandler(BaseHTTPRequestHandler):
            def do_GET(self):
                if self.path.split('?')[0] != '/v1/settings':
                    self.send_error(404)
                    return
                body = json.dumps({'announcements': [
                    {'id': 'lifetime-check', 'message': 'Deferred callback is alive'}]}).encode()
                self.send_response(200)
                self.send_header('Content-Type', 'application/json')
                self.send_header('Content-Length', str(len(body)))
                self.end_headers()
                self.wfile.write(body)

            def log_message(self, *args):
                pass

        server = ThreadingHTTPServer(('127.0.0.1', 0), SettingsHandler)
        server_thread = threading.Thread(target=server.serve_forever, daemon=True)
        server_thread.start()
        def close_server():
            server.shutdown()
            server.server_close()
            server_thread.join(timeout=2)
        self.addCleanup(close_server)
        origin = f'http://127.0.0.1:{server.server_port}'
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            home = root / 'home'
            home.mkdir()
            env = {'PATH': os.environ['PATH'], 'HOME': str(home),
                   'DISTILL_HOME': str(home / '.distill'), 'TMPDIR': str(root),
                   'LANG': 'en_US.UTF-8', 'RC_BOT_RUN': '1',
                   'GROK_OAUTH2_ISSUER': origin, 'GROK_OAUTH2_CLIENT_ID': 'lifetime-fixture'}
            distill_home = home / '.distill'
            distill_home.mkdir()
            (distill_home / 'config.toml').write_text(
                f'[endpoints]\ncli_chat_proxy_base_url = "{origin}"\n')
            (distill_home / 'auth.json').write_text(json.dumps({f'{origin}::lifetime-fixture': {
                'key': 'isolated-test-token', 'auth_mode': 'oidc', 'oidc_issuer': origin,
                'create_time': '2026-01-01T00:00:00Z', 'expires_at': '2099-01-01T00:00:00Z',
                'user_id': 'lifetime-fixture'}}))
            with (root / 'stderr.log').open('w+') as stderr:
                process = subprocess.Popen(
                    [str(binary), '--no-auto-update', 'agent', '--no-leader', 'stdio'],
                    stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=stderr,
                    text=True, env=env, cwd=root)
                messages = queue.Queue()
                observed = []

                def read_output():
                    for line in process.stdout:
                        messages.put(json.loads(line))
                    messages.put(None)

                reader = threading.Thread(target=read_output, daemon=True)
                reader.start()

                def request(identifier, method, params):
                    process.stdin.write(json.dumps({'jsonrpc': '2.0', 'id': identifier,
                                                    'method': method, 'params': params}) + '\n')
                    process.stdin.flush()
                    deadline = time.monotonic() + 30
                    while time.monotonic() < deadline:
                        message = messages.get(timeout=max(.01, deadline - time.monotonic()))
                        self.assertIsNotNone(message, 'ACP process exited during initialization')
                        observed.append(message)
                        if message.get('id') == identifier:
                            self.assertIn('result', message)
                            return message['result']
                    self.fail('ACP response timed out')

                try:
                    response = request(1, 'initialize', {
                        'protocolVersion': 1,
                        'clientCapabilities': {'_meta': {'remotecode/operationStatusV1': True}}})
                    self.assertTrue(response['_meta']['remotecode/readyRequired'])
                    request(2, '_remotecode/ready', {'_meta': {'remotecode/operationId': 'boot'}})
                    # initialize schedules announcements two seconds later, after readiness is stored.
                    time.sleep(3)
                    self.assertIsNone(process.poll(), 'agent crashed after readiness completed')
                    request(3, '_remotecode/status', {'operationId': 'boot'})
                    self.assertTrue(any(message.get('method') == '_x.ai/announcements/update'
                                        for message in observed),
                                    f'deferred initialization must still reach the client: '
                                    f'{[message.get("method") for message in observed]}')
                finally:
                    process.stdin.close()
                    try:
                        process.wait(timeout=10)
                    except subprocess.TimeoutExpired:
                        process.kill()
                        process.wait(timeout=5)
                    reader.join(timeout=1)
                    process.stdout.close()
                    stderr.seek(0)
                    self.assertEqual(process.returncode, 0, stderr.read())


if __name__ == '__main__':
    unittest.main()
