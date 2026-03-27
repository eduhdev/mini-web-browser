use native_tls::TlsConnector;
use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;

const DEFAULT_FILE: &str = "test.html";

thread_local! {
    static CONNECTIONS: RefCell<HashMap<String, Connection>> = RefCell::new(HashMap::new());
}

fn show(body: &str) {
    let mut in_tag = false;
    let mut entity = String::new();
    let mut in_entity = false;

    for c in body.chars() {
        if in_entity {
            entity.push(c);

            if entity == "&lt;" {
                print!("<");
                entity.clear();
                in_entity = false;
            } else if entity == "&gt;" {
                print!(">");
                entity.clear();
                in_entity = false;
            } else if c == ';' {
                print!("{entity}");
                entity.clear();
                in_entity = false;
            }
        } else if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if c == '&' && !in_tag {
            entity.push(c);
            in_entity = true;
        } else if !in_tag {
            print!("{c}");
        }
    }
}

fn load(url: &Url) {
    let body = url.request();
    if url.view_source {
        print!("{body}");
    } else {
        show(&body);
    }
    println!();
}

struct Url {
    view_source: bool,
    inner: Option<Box<Url>>,
    scheme: String,
    host: String,
    path: String,
}

struct Connection {
    response: BufReader<Box<dyn ReadWrite>>,
}

impl Url {
    fn new(url: &str) -> Self {
        let (scheme, rest) = url.split_once(':').expect("URL must contain :");
        assert!(matches!(scheme, "http" | "https" | "file" | "data" | "view-source"));

        if scheme == "view-source" {
            return Self {
                view_source: true,
                inner: Some(Box::new(Self::new(rest))),
                scheme: scheme.to_string(),
                host: String::new(),
                path: String::new(),
            };
        }

        if scheme == "data" {
            let (_media_type, data) = rest.split_once(',').expect("data URL must contain ','");
            return Self {
                view_source: false,
                inner: None,
                scheme: scheme.to_string(),
                host: String::new(),
                path: data.to_string(),
            };
        }

        let rest = if let Some(stripped) = rest.strip_prefix("//") {
            stripped
        } else {
            rest
        };

        if scheme == "file" {
            let path = if rest.starts_with('/') {
                rest.to_string()
            } else {
                format!("/{rest}")
            };

            return Self {
                view_source: false,
                inner: None,
                scheme: scheme.to_string(),
                host: String::new(),
                path,
            };
        }

        let normalized = if rest.contains('/') {
            rest.to_string()
        } else {
            format!("{rest}/")
        };

        let (host, path) = normalized
            .split_once('/')
            .expect("URL must contain a host");

        Self {
            view_source: false,
            inner: None,
            scheme: scheme.to_string(),
            host: host.to_string(),
            path: format!("/{path}"),
        }
    }

    fn request(&self) -> String {
        if self.view_source {
            return self
                .inner
                .as_ref()
                .expect("view-source URL missing inner URL")
                .request();
        }

        if self.scheme == "data" {
            return self.path.clone();
        }

        if self.scheme == "file" {
            return fs::read_to_string(&self.path).expect("failed to read local file");
        }

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

        let headers = vec![
            ("Host", host.as_str()),
            ("Connection", "keep-alive"),
            ("User-Agent", "eduhdev-browser/0.1"),
        ];

        let mut request = format!("GET {} HTTP/1.1\r\n", self.path);
        for (header, value) in headers {
            request.push_str(&format!("{header}: {value}\r\n"));
        }
        request.push_str("\r\n");

        let key = format!("{}://{}:{}", self.scheme, host, port);

        CONNECTIONS.with(|connections| {
            let mut connections = connections.borrow_mut();
            let connection = connections
                .entry(key)
                .or_insert_with(|| Connection::new(&self.scheme, &host, port));

            connection
                .response
                .get_mut()
                .write_all(request.as_bytes())
                .expect("failed to send request");

            let mut statusline = String::new();
            connection
                .response
                .read_line(&mut statusline)
                .expect("failed to read status line");

            let mut parts = statusline.trim_end().splitn(3, ' ');
            let _version = parts.next().expect("missing HTTP version");
            let _status = parts.next().expect("missing status code");
            let _explanation = parts.next().expect("missing status explanation");

            let mut response_headers = HashMap::new();
            loop {
                let mut line = String::new();
                connection
                    .response
                    .read_line(&mut line)
                    .expect("failed to read header");

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

            let content_length = response_headers
                .get("content-length")
                .expect("missing content-length")
                .parse::<usize>()
                .expect("invalid content-length");

            let mut content = vec![0; content_length];
            connection
                .response
                .read_exact(&mut content)
                .expect("failed to read response body");

            String::from_utf8(content).expect("response body was not utf-8")
        })
    }
}

impl Connection {
    fn new(scheme: &str, host: &str, port: u16) -> Self {
        let tcp_stream = TcpStream::connect((host, port)).expect("failed to connect");

        let stream: Box<dyn ReadWrite> = if scheme == "https" {
            let connector = TlsConnector::new().expect("failed to create TLS connector");
            let stream = connector
                .connect(host, tcp_stream)
                .expect("failed to establish TLS connection");
            Box::new(stream)
        } else {
            Box::new(tcp_stream)
        };

        Self {
            response: BufReader::new(stream),
        }
    }
}

trait ReadWrite: Read + Write {}

impl<T: Read + Write> ReadWrite for T {}

fn main() {
    let url = env::args().nth(1).unwrap_or_else(default_file_url);
    load(&Url::new(&url));
}

fn default_file_url() -> String {
    let base = option_env!("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().expect("failed to get current directory"));
    let path = base.join(DEFAULT_FILE);
    format!("file://{}", path.display())
}
