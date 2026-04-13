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

Inside each implementation, the code is split into separate loading, parsing, styling, layout, painting, and graphics responsibilities.

Current Python modules:

- `python/browser/network.py`
  URL handling, HTTP/file/data/about loading, redirects, caching, compression, and response decoding
- `python/browser/parser.py`
  HTML tree construction, implicit tag insertion, debug serialization, and visible-text extraction
- `python/browser/css.py`
  CSS parsing, selector matching, cascade priority, inheritance, and inline style handling
- `python/browser/layout.py`
  document/block layout objects plus CSS-driven paint command generation
- `python/browser/graphics.py`
  Tk window, stylesheet loading, paint tree traversal, drawing, scrolling, resizing, and input handling
- `python/browser/fonts.py`
  shared font caching
- `python/browser/emoji.py`
  emoji asset loading from the root `openmoji/` folder
- `browser.css`
  default browser stylesheet shared by both implementations

Current Rust modules:

- `rust/src/network.rs`
  URL handling, HTTP/file/data/about loading, redirects, caching, compression, and response decoding
- `rust/src/parser.rs`
  HTML tree construction, implicit tag insertion, debug serialization, and visible-text extraction
- `rust/src/css.rs`
  CSS parsing, selector matching, cascade priority, inheritance, and inline style handling
- `rust/src/layout.rs`
  document/block layout objects, font measurement, and CSS-driven paint command generation
- `rust/src/graphics.rs`
  egui window, stylesheet loading, paint tree traversal, drawing, scrolling, resizing, and input handling
- `rust/src/emoji.rs`
  emoji asset loading from the root `openmoji/` folder
- `rust/src/constants.rs`
  shared UI/layout constants

## Current Status

The project currently contains a small browser prototype, with a text-mode CLI and a graphical text renderer built on top of an HTML tree and a layout tree.

Implemented so far in the loading layer:

- `http`, `https`, `file`, `data`, `view-source`, and `about:blank` URL support
- HTTP/1.1 requests with reusable request headers
- `Host`, `Connection`, `User-Agent`, and `Accept-Encoding` headers
- keep-alive connection reuse
- redirect handling with a redirect limit
- basic response caching with `Cache-Control: no-store` and `max-age`
- gzip-compressed response support
- chunked transfer decoding

Implemented so far in the parser/document layer:

- HTML parsing into `Element` and `Text` nodes
- implicit `html`, `head`, and `body` insertion
- self-closing tag handling
- visible-text extraction from the parsed tree
- support for `&lt;` and `&gt;` entities
- debug printing of the reconstructed HTML tree

Implemented so far in the styling layer:

- default browser stylesheet loaded from `browser.css`
- inline `style` attribute parsing
- external stylesheet loading through `<link rel="stylesheet" href="...">`
- relative stylesheet URL resolution
- tag selectors and descendant selectors
- cascade sorting by selector priority
- inherited properties for:
  - `font-size`
  - `font-style`
  - `font-weight`
  - `color`
- percentage font-size resolution
- CSS-driven text color, font size, font style, font weight, and background color

Implemented so far in the layout/paint layer:

- document-level and block-level layout objects
- word-based layout with measured text widths
- block vs inline layout mode selection
- scrolling, resizing, and a proportional scrollbar
- line breaks for `<br>` and paragraph/block boundaries
- optional `--rtl` layout mode
- paint-command based rendering (`DrawText`, `DrawEmoji`, `DrawRect`)
- emoji rendering from the root `openmoji/` folder
- shared font/style caching

This is still intentionally minimal. It does not yet implement full browser-grade HTML parsing, complete CSS, or JavaScript.

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
