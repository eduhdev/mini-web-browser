import socket
import ssl
from pathlib import Path


DEFAULT_FILE = Path(__file__).with_name("test.html")

def show(body):
    in_tag = False
    entity = ""
    in_entity = False
    for c in body:
        if in_entity:
            entity += c
            if entity == "&lt;":
                print("<", end="")
                entity = ""
                in_entity = False
            elif entity == "&gt;":
                print(">", end="")
                entity = ""
                in_entity = False
            elif c == ";":
                print(entity, end="")
                entity = ""
                in_entity = False
            continue

        if c == "<":
            in_tag = True
        elif c == ">":
            in_tag = False
        elif c == "&" and not in_tag:
            entity = c
            in_entity = True
        elif not in_tag:
            print(c, end="")


def load(url):
    body = url.request()
    if url.view_source:
        print(body, end="")
    else:
        show(body)
    print()


class URL:
    connections = {}

    def __init__(self, url):
        self.view_source = False
        self.inner = None
        self.scheme, url = url.split(":", 1)
        assert self.scheme in ["http", "https", "file", "data", "view-source"]

        if self.scheme == "view-source":
            self.view_source = True
            self.inner = URL(url)
            return

        if self.scheme == "data":
            self.host = ""
            self.path = ""
            self.media_type, self.data = url.split(",", 1)
            return

        if url.startswith("//"):
            url = url[2:]

        if self.scheme == "file":
            if url.startswith("/"):
                self.host = ""
                self.path = url
            else:
                self.host = ""
                self.path = "/" + url
            return

        if "/" not in url:
            url = url + "/"
        self.host, url = url.split("/", 1)
        self.path = "/" + url

    def request(self):
        if self.view_source:
            return self.inner.request()

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
        }

        request = "GET {} HTTP/1.1\r\n".format(self.path)
        for header, value in headers.items():
            request += "{}: {}\r\n".format(header, value)
        request += "\r\n"

        s.sendall(request.encode("utf8"))

        statusline = response.readline().decode("utf8")
        version, status, explanation = statusline.split(" ", 2)

        response_headers = {}
        while True:
            line = response.readline().decode("utf8")
            if line == "\r\n":
                break
            header, value = line.split(":", 1)
            response_headers[header.casefold()] = value.strip()

        assert "transfer-encoding" not in response_headers
        assert "content-encoding" not in response_headers
        assert "content-length" in response_headers

        content_length = int(response_headers["content-length"])
        content = response.read(content_length).decode("utf8")

        return content


if __name__ == "__main__":
    import sys
    if len(sys.argv) > 1:
        url = sys.argv[1]
    else:
        url = "file://" + str(DEFAULT_FILE)
    load(URL(url))
