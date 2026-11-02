#!/usr/bin/env python3
"""Deterministic stand-in for download.geofabrik.de used by the CLI e2e.

Every region path gets its own synthetic snapshot (64 KiB derived from the
path), so each region stores a distinct object and gets a distinct CID:

  GET /<path>-latest.osm.pbf      -> 307 to /<path>-261024.osm.pbf
  GET /<path>-latest.osm.pbf.md5  -> "<md5>  <name>-latest.osm.pbf"
  GET /<path>-261024.osm.pbf      -> the snapshot bytes

Usage: e2e_geofabrik_fixture.py PORT_FILE
Binds 127.0.0.1 on a free port and writes the port to PORT_FILE.
"""
import hashlib
import http.server
import sys

VERSION = "261024"
SIZE = 64 * 1024


def snapshot(path: str) -> bytes:
    """Bytes for a region path, independent of the requested alias/version."""
    seed = path.rsplit("-", 1)[0].encode()
    out, counter = bytearray(), 0
    while len(out) < SIZE:
        out += hashlib.sha256(seed + counter.to_bytes(4, "big")).digest()
        counter += 1
    return bytes(out[:SIZE])


class Handler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):  # noqa: N802 (http.server API)
        path = self.path.split("?", 1)[0]
        if path.endswith("-latest.osm.pbf.md5"):
            pbf = path[: -len(".md5")]
            name = pbf.rsplit("/", 1)[-1]
            body = f"{hashlib.md5(snapshot(pbf)).hexdigest()}  {name}\n".encode()
            return self.reply(200, body, "text/plain")
        if path.endswith("-latest.osm.pbf"):
            self.send_response(307)
            self.send_header("Location", path.replace("-latest.osm.pbf", f"-{VERSION}.osm.pbf"))
            self.send_header("Content-Length", "0")
            self.end_headers()
            return None
        if path.endswith(f"-{VERSION}.osm.pbf"):
            return self.reply(200, snapshot(path), "application/octet-stream")
        return self.reply(404, b"not found\n", "text/plain")

    def reply(self, status, body, content_type):
        self.send_response(status)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Last-Modified", "Sat, 24 Oct 2026 00:51:09 GMT")
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, fmt, *args):
        sys.stderr.write("fixture: " + fmt % args + "\n")


def main():
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    with open(sys.argv[1], "w", encoding="utf-8") as f:
        f.write(str(server.server_address[1]))
    server.serve_forever()


if __name__ == "__main__":
    main()
