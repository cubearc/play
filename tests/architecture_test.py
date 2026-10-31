"""Enforce the reviewed native-only product boundary, alongside real native journeys."""
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
RETIRED = ["crates/play-local", "crates/play-launcher", "crates/play-runner", "frontend/src", "frontend/tests", "frontend/index.html", "frontend/playwright.config.ts", "frontend/vite.launcher.config.ts", "Dockerfile", "compose.yaml", ".dockerignore", "tests/runtime_smoke.py", "crates/play-engine/src/workspace/upgrade.rs"]

class NativeArchitecture(unittest.TestCase):
    def test_retired_entry_points_are_absent(self):
        self.assertEqual([p for p in RETIRED if (ROOT/p).exists()], [])

    def test_frontend_has_one_native_build_and_no_browser_server(self):
        package=json.loads((ROOT/"frontend/package.json").read_text())
        self.assertEqual(package["scripts"]["build"], "npm run desktop:build")
        self.assertFalse({"dev", "test:runner"} & package["scripts"].keys())
        self.assertFalse({"echarts", "tailwindcss", "@tailwindcss/vite"} & (package["dependencies"] | package["devDependencies"]).keys())

    def test_storage_has_one_baseline_and_no_migration_modules(self):
        storage=ROOT/"crates/play-engine/src/workspace"
        self.assertEqual(sorted(p.name for p in storage.glob("*.sql")), ["schema.sql"])
        self.assertTrue((ROOT/"frontend/desktop/main.ts").is_file())
        self.assertTrue((ROOT/"desktop/protocol.json").is_file())

if __name__ == "__main__":
    unittest.main()
