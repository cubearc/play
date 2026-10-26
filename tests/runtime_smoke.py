"""Exercise the real Compose image without mounting or changing host investigations."""

import http.cookiejar
import json
import os
from pathlib import Path
import re
import socket
import subprocess
import tempfile
import time
from concurrent.futures import ThreadPoolExecutor
import unittest
import urllib.error
import urllib.request
import urllib.parse
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
        cls.runtime_user = f"{os.geteuid()}:{os.getegid()}" if hasattr(os, "geteuid") else "65532:65532"
        runtime_uid, runtime_gid = cls.runtime_user.split(":")
        cls.environment = {**os.environ, "CUBE_PLAY_PORT": str(cls.port), "CUBE_PLAY_UID": runtime_uid, "CUBE_PLAY_GID": runtime_gid}
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
        self.assertEqual(config["User"], self.runtime_user)
        self.assertNotEqual(config["User"].split(":")[0], "0")
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

    def test_launcher_workspace_reuse_read_only_and_persistent_volumes(self):
        launcher = ROOT / "target" / "debug" / ("cube-play.exe" if os.name == "nt" else "cube-play")
        self.assertTrue(launcher.is_file(), "Build play-launcher before running Docker smoke tests")
        image = self.compose("images", "--quiet", "play-local").strip()
        with tempfile.TemporaryDirectory(prefix="cube-play-launcher-") as temporary:
            base = Path(temporary).resolve()
            source = base / "investigation $private 空"
            source.mkdir()
            if os.name != "nt":
                source.chmod(0o700)
            home = base / "state $local"
            environment = {**os.environ, "CUBE_PLAY_HOME": str(home), "CUBE_PLAY_IMAGE": image}
            probe_source = base / "browser_probe.rs"
            probe = base / ("browser-probe.exe" if os.name == "nt" else "browser-probe")
            record = base / "browser-ready"
            probe_source.write_text(r'''
use std::{env,io::{Read,Write},net::TcpStream,time::Duration};
fn main(){
    let url=env::args().nth(1).unwrap();
    let authority=url.strip_prefix("http://").unwrap().split('/').next().unwrap();
    let mut stream=TcpStream::connect(authority).unwrap();
    stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    write!(stream,"GET /healthz HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",authority).unwrap();
    let mut text=String::new();stream.take(1024).read_to_string(&mut text).unwrap();
    assert!(text.starts_with("HTTP/1.1 200 "));
    std::fs::write(env::var_os("CUBE_PLAY_BROWSER_RECORD").unwrap(),b"ready").unwrap();
}
''')
            subprocess.run(["rustc", "--edition=2024", str(probe_source), "-o", str(probe)], check=True, capture_output=True)
            environment.update(CUBE_PLAY_BROWSER=str(probe), CUBE_PLAY_BROWSER_RECORD=str(record))
            cookie_jar = http.cookiejar.CookieJar()
            opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), urllib.request.HTTPCookieProcessor(cookie_jar))

            def launch(*args, cwd=source, browser=False):
                binary = [os.environ["CUBE_PLAY_CUBE"], "play"] if os.environ.get("CUBE_PLAY_CUBE") else [str(launcher)]
                result = subprocess.run([*binary, "--offline", *([] if browser else ["--no-browser"]), *map(str, args)], cwd=cwd, env=environment, capture_output=True, text=True, timeout=150)
                diagnostics = re.sub(r"#session=[a-f0-9-]+", "#session=[redacted]", result.stdout + result.stderr)
                if result.returncode and (home / "workspaces.json").is_file():
                    for workspace in json.loads((home / "workspaces.json").read_text())["workspaces"]:
                        container = "cube-play-" + workspace["id"].replace("-", "") + "-local"
                        health = subprocess.run(["docker", "inspect", "--format", "{{json .State}}", container], capture_output=True, text=True, timeout=15)
                        diagnostics += "\nRuntime state: " + health.stdout[-4000:]
                        mounts = subprocess.run(["docker", "inspect", "--format", "{{json .Mounts}}", container], capture_output=True, text=True, timeout=15)
                        diagnostics += "\nFixture mounts: " + mounts.stdout[-4000:]
                self.assertEqual(result.returncode, 0, diagnostics)
                match = re.search(r"Open Cube Play: (http://127\.0\.0\.1:\d+/#[^\s]+)", result.stdout)
                self.assertIsNotNone(match, "Launcher did not provide a ready local session")
                return match[1]

            def authenticate(url):
                parsed = urllib.parse.urlsplit(url)
                origin = f"{parsed.scheme}://{parsed.netloc}"
                values = urllib.parse.parse_qs(parsed.fragment)
                request = urllib.request.Request(origin + "/api/session", method="POST", headers={"Origin": origin, "Authorization": "Bearer " + values["session"][0]})
                with opener.open(request, timeout=5) as response:
                    self.assertEqual(response.status, 204)
                with opener.open(origin + "/api/health", timeout=5) as response:
                    return origin, json.load(response)

            def inspect(name):
                return json.loads(subprocess.check_output(["docker", "inspect", name], text=True))[0]

            try:
                url = launch()
                origin, health = authenticate(url)
                self.assertEqual(health["workspace"]["name"], source.name)
                self.assertNotIn(str(base), json.dumps(health))
                self.assertEqual(list(source.iterdir()), [], "Launcher must not create source files")
                registry = json.loads((home / "workspaces.json").read_text())
                workspace_id = registry["workspaces"][0]["id"]
                name = f"cube-play-{workspace_id.replace('-', '')}-local"
                first = inspect(name)
                bindings = [mount for mount in first["Mounts"] if mount["Type"] == "bind"]
                self.assertEqual(len(bindings), 2)
                self.assertTrue(all(not mount["RW"] for mount in bindings))
                self.assertTrue(any(mount["Destination"] == "/workspace/source" for mount in bindings))
                self.assertEqual(first["HostConfig"]["PortBindings"]["7331/tcp"][0]["HostIp"], "127.0.0.1")
                self.assertFalse(first["HostConfig"]["Privileged"])
                self.assertEqual(first["Config"]["User"], self.runtime_user)
                self.assertEqual(first["State"]["Health"]["Status"], "healthy")
                launch(browser=True)
                deadline = time.monotonic() + 5
                while not record.exists() and time.monotonic() < deadline:
                    time.sleep(0.05)
                self.assertEqual(record.read_bytes(), b"ready", "Browser opening must follow actual HTTP readiness")
                denied = subprocess.run(["docker", "exec", name, "touch", "/workspace/source/forbidden"], capture_output=True)
                self.assertNotEqual(denied.returncode, 0)
                self.assertFalse((source / "forbidden").exists())
                subprocess.run(["docker", "exec", name, "sh", "-c", "printf retained > /var/lib/cube-play/test-marker && printf derived > /var/lib/cube-play/artifacts/result.bin"], check=True)
                for filename, intent in [("sample 空.mp4", "file"), ("rabbit.cube", "challenge")]:
                    selected = source / filename
                    selected.write_bytes(b"original local fixture; never execute")
                    if os.name != "nt":
                        selected.chmod(0o600)
                    selected_url = launch(selected)
                    values = urllib.parse.parse_qs(urllib.parse.urlsplit(selected_url).fragment)
                    self.assertEqual(values["file"], [filename])
                    self.assertEqual(values["intent"], [intent])
                    self.assertEqual(inspect(name)["Id"], first["Id"])
                    self.assertEqual(selected.read_bytes(), b"original local fixture; never execute")
                    self.assertEqual(subprocess.check_output(["docker", "exec", name, "cat", f"/workspace/source/{filename}"]), b"original local fixture; never execute")
                    if os.name != "nt":
                        self.assertEqual(selected.stat().st_mode & 0o777, 0o600)
                        self.assertEqual(source.stat().st_mode & 0o777, 0o700)
                with ThreadPoolExecutor(max_workers=2) as executor:
                    urls = list(executor.map(lambda _: launch(), range(2)))
                self.assertTrue(all(authenticate(value)[1]["workspace"]["id"] == workspace_id for value in urls))
                self.assertEqual(inspect(name)["Id"], first["Id"])
                second_source = base / "second workspace"
                second_source.mkdir()
                second_url = launch(cwd=second_source)
                second_origin, second_health = authenticate(second_url)
                self.assertNotEqual(second_origin, origin)
                self.assertNotEqual(second_health["workspace"]["id"], workspace_id)
                with opener.open(origin + "/api/health", timeout=5) as response:
                    self.assertEqual(json.load(response)["workspace"]["id"], workspace_id)
                with opener.open(second_origin + "/api/health", timeout=5) as response:
                    self.assertEqual(json.load(response)["workspace"]["id"], second_health["workspace"]["id"])
                binary = [os.environ["CUBE_PLAY_CUBE"], "play"] if os.environ.get("CUBE_PLAY_CUBE") else [str(launcher)]
                stopped = subprocess.run([*binary, "--stop"], cwd=source, env=environment, capture_output=True, timeout=60)
                self.assertEqual(stopped.returncode, 0)
                self.assertFalse(inspect(name)["State"]["Running"])
                self.assertTrue(inspect(f"cube-play-{second_health['workspace']['id'].replace('-', '')}-local")["State"]["Running"])
                subprocess.run(["docker", "container", "rm", name], check=True, capture_output=True)
                restarted = launch()
                self.assertEqual(authenticate(restarted)[1]["workspace"]["id"], workspace_id)
                self.assertNotEqual(inspect(name)["Id"], first["Id"])
                self.assertEqual(subprocess.check_output(["docker", "exec", name, "cat", "/var/lib/cube-play/test-marker"]), b"retained")
                self.assertEqual(subprocess.check_output(["docker", "exec", name, "cat", "/var/lib/cube-play/artifacts/result.bin"]), b"derived")
                self.assertEqual((source / "rabbit.cube").read_bytes(), b"original local fixture; never execute")
            finally:
                registry_file = home / "workspaces.json"
                if registry_file.exists():
                    for workspace in json.loads(registry_file.read_text())["workspaces"]:
                        identifier = workspace["id"].replace("-", "")
                        folder = home / identifier
                        if (folder / "compose.json").exists():
                            # Only these fresh fixture-owned projects and test volumes are removed.
                            subprocess.run(["docker", "compose", "--project-name", f"cube-play-{identifier}", "--file", str(folder / "compose.json"), "--env-file", str(folder / "empty.env"), "down", "--volumes", "--remove-orphans"], env=environment, capture_output=True, timeout=60, check=True)


if __name__ == "__main__":
    unittest.main(verbosity=2)
