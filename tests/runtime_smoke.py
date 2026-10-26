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
        cls.addClassCleanup(cls.compose, "down", "--volumes", "--remove-orphans")
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

            def files_api(origin, path):
                with opener.open(origin + path, timeout=10) as response:
                    return json.load(response)

            def save_record(origin, endpoint, payload):
                request = urllib.request.Request(origin + endpoint, data=json.dumps(payload).encode(), method="PUT", headers={"Origin": origin, "Content-Type": "application/json"})
                with opener.open(request, timeout=10) as response:
                    return json.load(response)

            try:
                url = launch()
                origin, health = authenticate(url)
                self.assertEqual(health["workspace"]["name"], source.name)
                self.assertNotIn(str(base), json.dumps(health))
                self.assertEqual(list(source.iterdir()), [], "Launcher must not create source files")
                self.assertEqual(files_api(origin, "/api/workspace/files")["entries"], [])
                imported = b"Private notes imported explicitly; source remains unchanged."
                request = urllib.request.Request(origin + "/api/workspace/import?name=private-notes.txt", data=imported, method="POST", headers={"Origin": origin, "Content-Type": "application/octet-stream"})
                with opener.open(request, timeout=10) as response:
                    self.assertEqual(response.status, 201)
                self.assertFalse((source / "private-notes.txt").exists())
                self.assertEqual(files_api(origin, "/api/workspace/inspect?store=imports&path=private-notes.txt")["text"]["value"], imported.decode())
                operation_id = str(uuid.uuid4())
                request = urllib.request.Request(origin + "/api/lab/execute", data=json.dumps({"id": operation_id, "operation": "encoding.base64.encode", "version": 1, "input": {"type": "bytes", "base64": "AP+A"}}).encode(), method="POST", headers={"Origin": origin, "Content-Type": "application/json"})
                with opener.open(request, timeout=10) as response:
                    lab_result = json.load(response)
                    self.assertEqual(lab_result["inline"]["data"], "AP+A")
                self.assertEqual(files_api(origin, "/api/lab/results/" + operation_id)["result"], lab_result["result"])
                investigation = files_api(origin, "/api/investigation")
                saved_view = {"store": "imports", "path": "", "offset": 0, "selected": {"store": "imports", "path": "private-notes.txt"}}
                self.assertEqual(save_record(origin, "/api/investigation", {"title": "Persistent case", "revision": investigation["revision"]})["revision"], 2)
                save_record(origin, "/api/investigation/view", {"revision": investigation["view_revision"], "view": saved_view})
                note_id = str(uuid.uuid4())
                save_record(origin, "/api/investigation/notes", {"id": note_id, "title": "Private hypothesis", "body": "Retained across real container recreation", "revision": 0})
                save_record(origin, "/api/investigation/progress", {"challenge_key": "local-fixture", "stage_key": "stage-one", "local_status": "in_progress", "revision": 0})
                artifact = files_api(origin, "/api/investigation/artifacts")[0]
                self.assertEqual(artifact["status"], "ready")
                if (ROOT / "frontend/node_modules/.bin/playwright").exists():
                    subprocess.run(["npm", "run", "test:runner"], cwd=ROOT / "frontend", env={**os.environ, "CUBE_PLAY_DOCKER_TEST_URL": url}, timeout=180, check=True)
                registry = json.loads((home / "workspaces.json").read_text())
                workspace_id = registry["workspaces"][0]["id"]
                name = f"cube-play-{workspace_id.replace('-', '')}-local"
                first = inspect(name)
                runner_name = name.removesuffix("-local") + "-runner"
                runner = inspect(runner_name)
                self.assertEqual(runner["HostConfig"]["NetworkMode"], "none")
                self.assertFalse(runner["HostConfig"]["Privileged"])
                self.assertTrue(runner["HostConfig"]["ReadonlyRootfs"])
                self.assertEqual(runner["HostConfig"]["Memory"], 512 * 1024 * 1024)
                self.assertEqual(runner["HostConfig"]["PidsLimit"], 64)
                self.assertFalse(runner["HostConfig"].get("PortBindings"))
                self.assertFalse(any("docker.sock" in mount["Source"] for mount in runner["Mounts"]))
                self.assertEqual(runner["Config"]["User"], self.runtime_user)
                for destination in ["/workspace/source", "/workspace/data"]:
                    self.assertTrue(any(mount["Destination"] == destination and not mount["RW"] for mount in runner["Mounts"]))

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
                image = (ROOT / "tests/fixtures/sample.png").read_bytes()
                (source / "sample.png").write_bytes(image)
                info = files_api(origin, "/api/workspace/inspect?path=sample.png")
                self.assertEqual(info["signatures"][0]["format"], "png")
                self.assertEqual(info["image"], {"width": 2, "height": 2})
                with opener.open(urllib.request.Request(origin + "/api/workspace/media?path=sample.png", headers={"Range": "bytes=0-11"}), timeout=10) as response:
                    self.assertEqual(response.status, 206)
                    self.assertEqual(response.read(), image[:12])
                self.assertEqual((source / "sample.png").read_bytes(), image)
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
                self.assertFalse(inspect(runner_name)["State"]["Running"])
                self.assertTrue(inspect(f"cube-play-{second_health['workspace']['id'].replace('-', '')}-local")["State"]["Running"])
                subprocess.run(["docker", "container", "rm", name], check=True, capture_output=True)
                restarted = launch()
                self.assertEqual(authenticate(restarted)[1]["workspace"]["id"], workspace_id)
                self.assertEqual(files_api(urllib.parse.urlsplit(restarted)._replace(path="", query="", fragment="").geturl(), "/api/workspace/inspect?store=imports&path=private-notes.txt")["text"]["value"], imported.decode())
                recovered_origin, _ = authenticate(restarted)
                recovered = files_api(recovered_origin, "/api/investigation")
                self.assertEqual(files_api(recovered_origin, "/api/lab/results/" + operation_id)["result"], lab_result["result"])
                self.assertEqual(recovered["id"], investigation["id"])
                self.assertEqual(recovered["title"], "Persistent case")
                self.assertEqual(recovered["view"], saved_view)
                self.assertNotEqual(recovered["session_id"], investigation["session_id"])
                self.assertEqual(recovered["interrupted_sessions"], 0)
                self.assertEqual(files_api(recovered_origin, "/api/investigation/notes")[0]["id"], note_id)
                self.assertEqual(files_api(recovered_origin, "/api/investigation/progress")[0]["local_status"], "in_progress")
                self.assertEqual(files_api(recovered_origin, "/api/investigation/artifacts")[0]["id"], artifact["id"])
                self.assertTrue(any(session["status"] == "closed" for session in files_api(recovered_origin, "/api/investigation/sessions")))
                self.assertNotEqual(inspect(name)["Id"], first["Id"])
                self.assertEqual(subprocess.check_output(["docker", "exec", name, "cat", "/var/lib/cube-play/test-marker"]), b"retained")
                self.assertEqual(subprocess.check_output(["docker", "exec", name, "cat", "/var/lib/cube-play/artifacts/result.bin"]), b"derived")
                self.assertEqual((source / "rabbit.cube").read_bytes(), b"original local fixture; never execute")
                # Exercise actual ungraceful termination and SQLite WAL recovery.
                subprocess.run(["docker", "kill", "--signal", "KILL", name], check=True, capture_output=True)
                crash_url = launch()
                crash_origin, _ = authenticate(crash_url)
                after_crash = files_api(crash_origin, "/api/investigation")
                self.assertEqual(after_crash["id"], investigation["id"])
                self.assertEqual(after_crash["interrupted_sessions"], 1)
                self.assertEqual(after_crash["view"], saved_view)
                self.assertEqual(files_api(crash_origin, "/api/investigation/notes")[0]["body"], "Retained across real container recreation")
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
