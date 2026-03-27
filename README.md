# Browser

This project is a learning-focused browser implementation inspired by the book *[Web Browser Engineering](https://browser.engineering/)* by Pavel Panchekha and Chris Harrelson.

The book primarily builds the browser in Python, and this repository follows that approach while also mirroring the same ideas in Rust.

## Goals

- Build the browser step by step in Python
- Recreate the same concepts in Rust
- Compare both implementations while learning how browsers work internally

## Structure

- `python/`
  Python implementation
- `rust/`
  Rust implementation

Inside each implementation, the code is now split into separate graphics and network layers so UI work can continue independently from fetching and parsing.

## Current Status

The project currently contains a small text-based browser prototype in both languages.

Implemented so far in the network layer:

- `http`, `https`, `file`, `data`, and `view-source` URL support
- HTTP/1.1 requests with reusable request headers
- `Host`, `Connection`, `User-Agent`, and `Accept-Encoding` headers
- keep-alive connection reuse
- redirect handling with a redirect limit
- basic response caching with `Cache-Control: no-store` and `max-age`
- gzip-compressed response support
- chunked transfer decoding
- simple HTML text extraction
- support for `&lt;` and `&gt;` entities

The project also now includes a minimal standalone graphics shell in both Python and Rust. The GUI is intentionally not connected to the network layer yet.

This is still intentionally minimal. It does not yet implement full HTML parsing, layout, CSS, or JavaScript.

## Run

Python CLI:

```bash
python3 python/index.py
python3 python/index.py https://example.org
python3 python/index.py 'data:text/html,<h1>Hello</h1>'
python3 python/index.py 'view-source:https://example.org'
```

Python GUI:

```bash
python3 python/gui.py
```

Rust GUI:

```bash
cd rust
cargo run
```

## Test Files

Both implementations include a local HTML file used when no URL is provided:

- `python/test.html`
- `rust/test.html`
