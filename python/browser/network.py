import gzip
import socket
import ssl
import time
from pathlib import Path


DEFAULT_FILE = Path(__file__).resolve().parent.parent / "test.html"
MAX_REDIRECTS = 10


def lex(body):
    in_tag = False
    tag = ""
    entity = ""
    in_entity = False
    text = ""
    in_whitespace = False

    for c in body:
        if in_entity:
            entity += c
            if entity == "&lt;":
                text += "<"
                entity = ""
                in_entity = False
                in_whitespace = False
            elif entity == "&gt;":
                text += ">"
                entity = ""
                in_entity = False
                in_whitespace = False
            elif c == ";":
                text += entity
                entity = ""
                in_entity = False
                in_whitespace = False
            continue

        if c == "<":
            in_tag = True
            tag = ""
        elif c == ">":
            normalized_tag = tag.strip().casefold()
            if normalized_tag in ["br", "br/", "/div"]:
                text = text.rstrip(" ")
                text += "\n"
                in_whitespace = False
            in_tag = False
            tag = ""
        elif c == "&" and not in_tag:
            entity = c
            in_entity = True
        elif in_tag:
            tag += c
        elif not in_tag:
            if c.isspace():
                if text and not in_whitespace:
                    text += " "
                in_whitespace = True
            else:
                text += c
                in_whitespace = False

    return text.strip()


def fetch(url):
    return URL(url).request()


class URL:
    connections = {}
    cache = {}

    def __init__(self, url):
        self.scheme = "about"
        self.host = ""
        self.path = "blank"
        self.view_source = False
        self.inner = None
        self.port = None

        try:
            self.scheme, url = url.split(":", 1)
        except ValueError:
            return

        if self.scheme not in ["http", "https", "file", "data", "view-source", "about"]:
            self.make_blank()
            return

        if self.scheme == "view-source":
            try:
                self.view_source = True
                self.inner = URL(url)
            except Exception:
                self.make_blank()
            return

        if self.scheme == "about":
            if url == "blank":
                return
            self.make_blank()
            return

        if self.scheme == "data":
            try:
                self.path = ""
                self.media_type, self.data = url.split(",", 1)
            except ValueError:
                self.make_blank()
            return

        if url.startswith("//"):
            url = url[2:]

        if self.scheme == "file":
            if url.startswith("/"):
                self.path = url
            else:
                self.path = "/" + url
            return

        if "/" not in url:
            url = url + "/"
        if "/" not in url:
            self.make_blank()
            return

        self.host, url = url.split("/", 1)
        if not self.host:
            self.make_blank()
            return
        self.path = "/" + url

    def make_blank(self):
        self.scheme = "about"
        self.host = ""
        self.path = "blank"
        self.port = None
        self.view_source = False
        self.inner = None

    def request(self, redirects=0):
        if self.view_source:
            return self.inner.request(redirects)

        if self.scheme == "about":
            return ""

        if self.scheme == "data":
            return self.data

        if self.scheme == "file":
            with open(self.path, "r", encoding="utf8") as f:
                return f.read()

        host = self.host

        if ":" in host:
            host, port = host.split(":", 1)
            self.port = int(port)
        elif self.scheme == "http":
            self.port = 80
        elif self.scheme == "https":
            self.port = 443

        self.host = host

        cached_response = self.get_cached_response()
        if cached_response is not None:
            status, response_headers, content = cached_response
            return self.handle_response(status, response_headers, content, redirects)

        key = (self.scheme, self.host, self.port)

        if key in self.connections:
            s, response = self.connections[key]
        else:
            s = socket.socket(
                socket.AF_INET,
                socket.SOCK_STREAM,
                proto=socket.IPPROTO_TCP
            )
            if self.scheme == "https":
                ctx = ssl.create_default_context()
                s = ctx.wrap_socket(s, server_hostname=self.host)
            s.connect((self.host, self.port))
            response = s.makefile("rb")
            self.connections[key] = (s, response)

        headers = {
            "Host": self.host,
            "Connection": "keep-alive",
            "User-Agent": "eduhdev-browser/0.1",
            "Accept-Encoding": "gzip",
        }

        request = "GET {} HTTP/1.1\r\n".format(self.path)
        for header, value in headers.items():
            request += "{}: {}\r\n".format(header, value)
        request += "\r\n"

        s.sendall(request.encode("utf8"))

        statusline = response.readline().decode("utf8")
        version, status, explanation = statusline.split(" ", 2)
        status = int(status)

        response_headers = {}
        while True:
            line = response.readline().decode("utf8")
            if line == "\r\n":
                break
            header, value = line.split(":", 1)
            response_headers[header.casefold()] = value.strip()

        transfer_encoding = response_headers.get("transfer-encoding")
        if transfer_encoding is None:
            content_length = int(response_headers.get("content-length", 0))
            content = response.read(content_length)
        else:
            assert transfer_encoding == "chunked"
            content = self.read_chunked(response)

        if response_headers.get("content-encoding") == "gzip":
            content = gzip.decompress(content)
        content = content.decode("utf8")

        self.cache_response(status, response_headers, content)

        return self.handle_response(status, response_headers, content, redirects)

    def resolve(self, location):
        if location.startswith("//"):
            return "{}:{}".format(self.scheme, location)
        if location.startswith("/"):
            return "{}://{}{}".format(self.scheme, self.authority(), location)
        if ":" in location.split("/", 1)[0]:
            return location
        raise AssertionError("unsupported redirect location")

    def authority(self):
        default_port = 80 if self.scheme == "http" else 443
        if self.port == default_port:
            return self.host
        return "{}:{}".format(self.host, self.port)

    def handle_response(self, status, response_headers, content, redirects):
        if 300 <= status < 400:
            assert "location" in response_headers
            assert redirects < MAX_REDIRECTS
            return URL(self.resolve(response_headers["location"])).request(redirects + 1)
        return content

    def cache_key(self):
        return "{}://{}{}".format(self.scheme, self.authority(), self.path)

    def get_cached_response(self):
        key = self.cache_key()
        if key not in self.cache:
            return None

        status, response_headers, content, expires_at = self.cache[key]
        if expires_at is not None and time.time() > expires_at:
            del self.cache[key]
            return None
        return status, response_headers, content

    def cache_response(self, status, response_headers, content):
        if status not in [200, 301, 404]:
            return

        cache_control = response_headers.get("cache-control")
        if cache_control is None:
            expires_at = None
        else:
            expires_at = self.parse_cache_control(cache_control)
            if expires_at is False:
                return

        self.cache[self.cache_key()] = (
            status,
            response_headers.copy(),
            content,
            expires_at,
        )

    def parse_cache_control(self, cache_control):
        expires_at = None
        for value in cache_control.split(","):
            value = value.strip()
            if value == "no-store":
                return False
            if value.startswith("max-age="):
                seconds = int(value.split("=", 1)[1])
                expires_at = time.time() + seconds
                continue
            return False
        return expires_at

    def read_chunked(self, response):
        body = b""
        while True:
            line = response.readline().decode("utf8").strip()
            chunk_size = int(line.split(";", 1)[0], 16)
            if chunk_size == 0:
                while True:
                    trailer = response.readline()
                    if trailer == b"\r\n":
                        break
                break
            body += response.read(chunk_size)
            assert response.read(2) == b"\r\n"
        return body
