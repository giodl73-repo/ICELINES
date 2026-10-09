"""Exercise the deployment verifier's network boundary with a local HTTP server."""
import hashlib
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import importlib.util
from pathlib import Path
import threading
import unittest

spec = importlib.util.spec_from_file_location("deployment", Path(__file__).parents[1] / "verify-browser-deployment.py")
deployment = importlib.util.module_from_spec(spec)
spec.loader.exec_module(deployment)


class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == "/redirect":
            self.send_response(302)
            self.send_header("Location", "/valid")
            self.end_headers()
            return
        payload = b"older build" if self.path == "/old" else b"\0asm\x01\0\0\0"
        self.send_response(200)
        self.send_header("Content-Type", "text/html" if self.path == "/html" else "application/wasm")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

    def log_message(self, *_):
        pass


class DeploymentTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        cls.thread = threading.Thread(target=cls.server.serve_forever, daemon=True)
        cls.thread.start()
        cls.base = f"http://127.0.0.1:{cls.server.server_port}"
        cls.sha = hashlib.sha256(b"\0asm\x01\0\0\0").hexdigest()

    @classmethod
    def tearDownClass(cls):
        cls.server.shutdown()
        cls.server.server_close()
        cls.thread.join()

    def test_accepts_exact_wasm_bytes_and_mime(self):
        result = deployment.verify_file(self.base + "/valid", self.sha, 8, {"application/wasm"})
        self.assertEqual(result["bytes"], 8)

    def test_refuses_html_fallback_with_matching_bytes(self):
        with self.assertRaisesRegex(ValueError, "MIME"):
            deployment.verify_file(self.base + "/html", self.sha, 8, {"application/wasm"})

    def test_refuses_stale_build(self):
        with self.assertRaisesRegex(ValueError, "digest"):
            deployment.verify_file(self.base + "/old", self.sha)

    def test_refuses_oversized_body(self):
        with self.assertRaisesRegex(ValueError, "size"):
            deployment.verify_file(self.base + "/valid", self.sha, 7)

    def test_refuses_redirect_even_to_matching_bytes(self):
        with self.assertRaisesRegex(ValueError, "redirected"):
            deployment.verify_file(self.base + "/redirect", self.sha, 8)

    def test_remote_deployment_cannot_use_plain_http(self):
        with self.assertRaisesRegex(ValueError, "require HTTPS"):
            deployment.verify(Path("unused"), "http://example.test/workbench/", "a" * 40, "b" * 64)


if __name__ == "__main__":
    unittest.main()
