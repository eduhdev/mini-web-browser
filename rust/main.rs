use native_tls::TlsConnector;
use std::collections::HashMap;
use std::env;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;

fn show(body: &str) {
    let mut in_tag = false;

    for c in body.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            print!("{c}");
        }
    }
}

fn load(url: &Url) {
    let body = url.request();
    show(&body);
}

struct Url {
    scheme: String,
    host: String,
    path: String,
}

impl Url {
    fn new(url: &str) -> Self {
        let (scheme, rest) = url.split_once("://").expect("URL must contain ://");
        assert!(matches!(scheme, "http" | "https"));

        let normalized = if rest.contains('/') {
            rest.to_string()
        } else {
            format!("{rest}/")
        };

        let (host, path) = normalized
            .split_once('/')
            .expect("URL must contain a host");

        Self {
            scheme: scheme.to_string(),
            host: host.to_string(),
            path: format!("/{path}"),
        }
    }

    fn request(&self) -> String {
        let (host, port) = if let Some((parsed_host, parsed_port)) = self.host.split_once(':') {
            (
                parsed_host.to_string(),
                parsed_port.parse::<u16>().expect("invalid port"),
            )
        } else if self.scheme == "http" {
            (self.host.clone(), 80)
        } else {
            (self.host.clone(), 443)
        };

        let tcp_stream = TcpStream::connect((host.as_str(), port)).expect("failed to connect");

        let mut request = format!("GET {} HTTP/1.0\r\n", self.path);
        request.push_str(&format!("Host: {}\r\n", host));
        request.push_str("\r\n");

        let mut response = if self.scheme == "https" {
            let connector = TlsConnector::new().expect("failed to create TLS connector");
            let mut stream = connector
                .connect(&host, tcp_stream)
                .expect("failed to establish TLS connection");
            stream
                .write_all(request.as_bytes())
                .expect("failed to send request");
            BufReader::new(Box::new(stream) as Box<dyn ReadWrite>)
        } else {
            let mut stream = tcp_stream;
            stream
                .write_all(request.as_bytes())
                .expect("failed to send request");
            BufReader::new(Box::new(stream) as Box<dyn ReadWrite>)
        };

        let mut statusline = String::new();
        response
            .read_line(&mut statusline)
            .expect("failed to read status line");

        let mut parts = statusline.trim_end().splitn(3, ' ');
        let _version = parts.next().expect("missing HTTP version");
        let _status = parts.next().expect("missing status code");
        let _explanation = parts.next().expect("missing status explanation");

        let mut response_headers = HashMap::new();
        loop {
            let mut line = String::new();
            response.read_line(&mut line).expect("failed to read header");

            if line == "\r\n" {
                break;
            }

            let (header, value) = line
                .split_once(':')
                .expect("header line must contain ':'");
            response_headers.insert(header.to_ascii_lowercase(), value.trim().to_string());
        }

        assert!(!response_headers.contains_key("transfer-encoding"));
        assert!(!response_headers.contains_key("content-encoding"));

        let mut content = String::new();
        response
            .read_to_string(&mut content)
            .expect("failed to read response body");

        content
    }
}

trait ReadWrite: Read + Write {}

impl<T: Read + Write> ReadWrite for T {}

fn main() {
    let url = env::args().nth(1).expect("usage: main <url>");
    load(&Url::new(&url));
}
