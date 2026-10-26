"""Exercise the real Compose image without mounting or changing host investigations."""

import http.cookiejar
import json
import os
from pathlib import Path
import re
import socket
import subprocess
import unittest
import urllib.error
import urllib.request
import uuid


ROOT = Path(__file__).resolve().parents[1]


class RuntimeSmoke(unittest.TestCase):
    @classmethod
    def compose(cls, *args, timeout=120):
        result = subprocess.run(
            ["docker", "compose", "--project-name", cls.project, *args],
            cwd=ROOT,
            env=cls.environment,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
        if result.returncode:
            output = re.sub(r"#session=[a-f0-9-]+", "#session=[redacted]", result.stdout + result.stderr)
            raise RuntimeError(output[-12000:])
        return result.stdout

    @classmethod
    def setUpClass(cls):
        cls.project = "cube-play-smoke-" + uuid.uuid4().hex[:12]
        with socket.socket() as listener:
            listener.bind(("127.0.0.1", 0))
            cls.port = listener.getsockname()[1]
        cls.environment = {**os.environ, "CUBE_PLAY_PORT": str(cls.port)}
        cls.origin = f"http://127.0.0.1:{cls.port}"
        cls.addClassCleanup(cls.compose, "down", "--remove-orphans")
        cls.compose("up", "--build", "--wait", "--wait-timeout", "60", timeout=1800)

    def request(self, path, headers=None, method="GET", opener=None):
        request = urllib.request.Request(self.origin + path, method=method, headers=headers or {})
        try:
            local_opener = opener or urllib.request.build_opener(urllib.request.ProxyHandler({}))
            response = local_opener.open(request, timeout=5)
        except urllib.error.HTTPError as error:
            response = error
        with response:
            return response.status, response.headers, response.read(1024 * 1024)

    def test_real_frontend_readiness_and_local_session(self):
        status, _, content = self.request("/")
        self.assertEqual(status, 200)
        self.assertIn(b"Cube Play", content)
        status, _, content = self.request("/healthz")
        self.assertEqual(status, 200)
        self.assertEqual(json.loads(content)["status"], "ok")
        self.assertEqual(self.request("/api/health")[0], 401)
        logs = self.compose("logs", "--no-color", "play-local")
        match = re.search(r"#session=([a-f0-9-]+)", logs)
        self.assertIsNotNone(match, "Local session URL was not printed")
        opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar()))
        status, headers, _ = self.request("/api/session", {"Origin": self.origin, "Authorization": "Bearer " + match[1]}, "POST", opener)
        self.assertEqual(status, 204)
        self.assertIn("HttpOnly", headers["Set-Cookie"])
        self.assertIn("SameSite=Strict", headers["Set-Cookie"])
        self.assertEqual(self.request("/api/health", opener=opener)[0], 200)

    def test_browser_request_boundaries(self):
        for headers in [{"Host": "attacker.example"}, {"Origin": "https://attacker.example"}, {"Sec-Fetch-Site": "cross-site"}]:
            self.assertEqual(self.request("/healthz", headers)[0], 403)

    def test_non_root_bounded_runtime_without_host_mounts(self):
        container = self.compose("ps", "--quiet", "play-local").strip()
        data = json.loads(subprocess.check_output(["docker", "inspect", container], text=True))[0]
        config, host = data["Config"], data["HostConfig"]
        self.assertEqual(config["User"], "65532:65532")
        self.assertTrue(host["ReadonlyRootfs"])
        self.assertFalse(host["Privileged"])
        self.assertIn("ALL", host["CapDrop"])
        self.assertIn("no-new-privileges:true", host["SecurityOpt"])
        self.assertEqual(host["PidsLimit"], 128)
        self.assertEqual(host["Memory"], 256 * 1024 * 1024)
        self.assertEqual(host["NanoCpus"], 1000000000)
        self.assertFalse(host["Binds"])
        self.assertFalse(any(mount["Type"] == "bind" for mount in data["Mounts"]))
        self.assertEqual(host["PortBindings"]["7331/tcp"][0]["HostIp"], "127.0.0.1")
        self.assertEqual(data["State"]["Health"]["Status"], "healthy")


if __name__ == "__main__":
    unittest.main(verbosity=2)
