use flate2::read::GzDecoder;
use native_tls::TlsConnector;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::thread_local;
use std::time::{Duration, Instant};

const MAX_REDIRECTS: usize = 10;

pub fn default_file_url() -> String {
    format!("file://{}/test.html", env!("CARGO_MANIFEST_DIR"))
}

thread_local! {
    static CONNECTIONS: RefCell<HashMap<String, Connection>> = RefCell::new(HashMap::new());
    static CACHE: RefCell<HashMap<String, CacheEntry>> = RefCell::new(HashMap::new());
}

pub struct Url {
    pub view_source: bool,
    inner: Option<Box<Url>>,
    scheme: String,
    host: String,
    path: String,
}

struct Connection {
    response: BufReader<Box<dyn ReadWrite>>,
}

struct CacheEntry {
    status: u16,
    headers: HashMap<String, String>,
    body: String,
    expires_at: Option<Instant>,
}

impl Url {
    pub fn new(url: &str) -> Self {
        let mut blank = Self::blank();
        let Some((scheme, rest)) = url.split_once(':') else {
            return blank;
        };

        if !matches!(scheme, "http" | "https" | "file" | "data" | "view-source" | "about") {
            return blank;
        }

        if scheme == "view-source" {
            return Self {
                view_source: true,
                inner: Some(Box::new(Self::new(rest))),
                scheme: scheme.to_string(),
                host: String::new(),
                path: String::new(),
            };
        }

        if scheme == "about" {
            if rest == "blank" {
                return blank;
            }
            return Self::blank();
        }

        if scheme == "data" {
            let Some((_media_type, data)) = rest.split_once(',') else {
                return Self::blank();
            };
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

        let Some((host, path)) = normalized.split_once('/') else {
            return Self::blank();
        };
        if host.is_empty() {
            return Self::blank();
        }

        blank.scheme = scheme.to_string();
        blank.host = host.to_string();
        blank.path = format!("/{path}");
        blank
    }

    pub fn request(&self) -> String {
        self.request_with_redirects(0)
    }

    fn request_with_redirects(&self, redirects: usize) -> String {
        if self.view_source {
            return self
                .inner
                .as_ref()
                .expect("view-source URL missing inner URL")
                .request_with_redirects(redirects);
        }

        if self.scheme == "about" {
            return String::new();
        }

        if self.scheme == "data" {
            return self.path.clone();
        }

        if self.scheme == "file" {
            return fs::read_to_string(&self.path).expect("failed to read local file");
        }

        if let Some((status, headers, body)) = self.get_cached_response() {
            return self.handle_response(status, &headers, body, redirects);
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
            ("Accept-Encoding", "gzip"),
        ];

        let mut request = format!("GET {} HTTP/1.1\r\n", self.path);
        for (header, value) in headers {
            request.push_str(&format!("{header}: {value}\r\n"));
        }
        request.push_str("\r\n");

        let key = format!("{}://{}:{}", self.scheme, host, port);

        let (status, response_headers, body) = CONNECTIONS.with(|connections| {
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
            let status = parts
                .next()
                .expect("missing status code")
                .parse::<u16>()
                .expect("invalid status code");
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

            let content = match response_headers.get("transfer-encoding").map(String::as_str) {
                None => {
                    let content_length = response_headers
                        .get("content-length")
                        .map(|value| value.parse::<usize>().expect("invalid content-length"))
                        .unwrap_or(0);

                    let mut content = vec![0; content_length];
                    connection
                        .response
                        .read_exact(&mut content)
                        .expect("failed to read response body");
                    content
                }
                Some("chunked") => read_chunked(&mut connection.response),
                Some(_) => panic!("unsupported transfer-encoding"),
            };

            let content = if response_headers.get("content-encoding").map(String::as_str) == Some("gzip") {
                let mut decoder = GzDecoder::new(&content[..]);
                let mut decoded = Vec::new();
                decoder
                    .read_to_end(&mut decoded)
                    .expect("failed to decompress gzip response");
                decoded
            } else {
                content
            };

            let body = String::from_utf8(content).expect("response body was not utf-8");
            (status, response_headers, body)
        });

        self.cache_response(status, &response_headers, &body);
        self.handle_response(status, &response_headers, body, redirects)
    }

    fn resolve(&self, location: &str) -> String {
        if location.starts_with("//") {
            format!("{}:{}", self.scheme, location)
        } else if location.starts_with('/') {
            format!("{}://{}{}", self.scheme, self.authority(), location)
        } else if location.split('/').next().unwrap_or("").contains(':') {
            location.to_string()
        } else {
            "about:blank".to_string()
        }
    }

    fn blank() -> Self {
        Self {
            view_source: false,
            inner: None,
            scheme: "about".to_string(),
            host: String::new(),
            path: "blank".to_string(),
        }
    }

    fn authority(&self) -> String {
        let default_port = if self.scheme == "http" { 80 } else { 443 };
        if self.port() == default_port {
            self.host_without_port().to_string()
        } else {
            format!("{}:{}", self.host_without_port(), self.port())
        }
    }

    fn host_without_port(&self) -> &str {
        self.host
            .split_once(':')
            .map(|(host, _)| host)
            .unwrap_or(&self.host)
    }

    fn port(&self) -> u16 {
        if let Some((_, port)) = self.host.split_once(':') {
            port.parse::<u16>().expect("invalid port")
        } else if self.scheme == "http" {
            80
        } else {
            443
        }
    }

    fn handle_response(
        &self,
        status: u16,
        response_headers: &HashMap<String, String>,
        body: String,
        redirects: usize,
    ) -> String {
        if (300..400).contains(&status) {
            let location = response_headers
                .get("location")
                .expect("redirect response missing location");
            assert!(redirects < MAX_REDIRECTS, "too many redirects");
            Url::new(&self.resolve(location)).request_with_redirects(redirects + 1)
        } else {
            body
        }
    }

    fn cache_key(&self) -> String {
        format!("{}://{}{}", self.scheme, self.authority(), self.path)
    }

    fn get_cached_response(&self) -> Option<(u16, HashMap<String, String>, String)> {
        CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            let key = self.cache_key();

            match cache.get(&key) {
                Some(entry)
                    if entry
                        .expires_at
                        .is_none_or(|expires_at| Instant::now() <= expires_at) =>
                {
                    Some((entry.status, entry.headers.clone(), entry.body.clone()))
                }
                Some(_) => {
                    cache.remove(&key);
                    None
                }
                None => None,
            }
        })
    }

    fn cache_response(&self, status: u16, headers: &HashMap<String, String>, body: &str) {
        if !matches!(status, 200 | 301 | 404) {
            return;
        }

        let expires_at = match headers.get("cache-control") {
            Some(value) => match parse_cache_control(value) {
                Some(expires_at) => expires_at,
                None => return,
            },
            None => None,
        };

        CACHE.with(|cache| {
            cache.borrow_mut().insert(
                self.cache_key(),
                CacheEntry {
                    status,
                    headers: headers.clone(),
                    body: body.to_string(),
                    expires_at,
                },
            );
        });
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

fn parse_cache_control(cache_control: &str) -> Option<Option<Instant>> {
    let mut expires_at = None;

    for value in cache_control.split(',') {
        let value = value.trim();
        if value == "no-store" {
            return None;
        }
        if let Some(seconds) = value.strip_prefix("max-age=") {
            let seconds = seconds.parse::<u64>().expect("invalid max-age");
            expires_at = Some(Instant::now() + Duration::from_secs(seconds));
            continue;
        }
        return None;
    }

    Some(expires_at)
}

fn read_chunked(response: &mut BufReader<Box<dyn ReadWrite>>) -> Vec<u8> {
    let mut body = Vec::new();

    loop {
        let mut line = String::new();
        response
            .read_line(&mut line)
            .expect("failed to read chunk size");

        let chunk_size = usize::from_str_radix(
            line.trim_end().split(';').next().expect("missing chunk size"),
            16,
        )
        .expect("invalid chunk size");

        if chunk_size == 0 {
            loop {
                let mut trailer = String::new();
                response
                    .read_line(&mut trailer)
                    .expect("failed to read chunk trailer");
                if trailer == "\r\n" {
                    break;
                }
            }
            break;
        }

        let start = body.len();
        body.resize(start + chunk_size, 0);
        response
            .read_exact(&mut body[start..])
            .expect("failed to read chunk");

        let mut crlf = [0; 2];
        response
            .read_exact(&mut crlf)
            .expect("failed to read chunk terminator");
        assert_eq!(crlf, *b"\r\n");
    }

    body
}
