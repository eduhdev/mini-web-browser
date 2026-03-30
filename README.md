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

Inside each implementation, the code is split into separate network, layout, graphics, and emoji/font helpers so rendering work can evolve independently from fetching and parsing.

Current Python modules:

- `python/browser/network.py`
  URL handling, HTTP/file/data/about loading, redirects, caching, compression, and HTML token extraction
- `python/browser/layout.py`
  text layout and tag-driven styling
- `python/browser/graphics.py`
  Tk window, drawing, scrolling, resizing, and input handling
- `python/browser/fonts.py`
  shared font caching
- `python/browser/emoji.py`
  emoji asset loading from the root `openmoji/` folder

Current Rust modules:

- `rust/src/network.rs`
- `rust/src/layout.rs`
- `rust/src/graphics.rs`
- `rust/src/emoji.rs`
- `rust/src/constants.rs`

## Current Status

The project currently contains a small browser prototype in both languages, with a text-mode CLI and a graphical text renderer.

Implemented so far in the network layer:

- `http`, `https`, `file`, `data`, `view-source`, and `about:blank` URL support
- HTTP/1.1 requests with reusable request headers
- `Host`, `Connection`, `User-Agent`, and `Accept-Encoding` headers
- keep-alive connection reuse
- redirect handling with a redirect limit
- basic response caching with `Cache-Control: no-store` and `max-age`
- gzip-compressed response support
- chunked transfer decoding
- HTML lexing into text/tag tokens
- support for `&lt;` and `&gt;` entities

Implemented so far in the graphics/layout layer:

- word-based layout with measured text widths
- scrolling, resizing, and a proportional scrollbar
- basic tag handling for:
  - `<b>`
  - `<i>`
  - `<small>`
  - `<big>`
  - `<br>`
  - `</div>`
  - `</p>`
- optional `--rtl` layout mode
- emoji rendering from the root `openmoji/` folder
- font caching on Python and cached styled text layouts on Rust

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
./run-python-gui.sh
./run-python-gui.sh "https://browser.engineering/text.html"
./run-python-gui.sh --rtl "data:text/html,<div><b>Hello</b> <i>world</i></div>"
```

Python GUI profiling:

```bash
./run-python-gui.sh --profile "https://browser.engineering/text.html"
```

Rust GUI:

```bash
cd rust
cargo run
cargo run -- "https://browser.engineering/text.html"
cargo run -- --rtl "data:text/html,<div><b>Hello</b> <i>world</i></div>"
```

## Test Files

Both implementations include a local HTML file used when no URL is provided:

- `python/test.html`
- `rust/test.html`
