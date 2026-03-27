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

## Current Status

The project currently contains a very small text-based browser prototype in both languages. It:

- parses a URL
- opens a network connection
- sends a basic HTTP request
- reads the response
- prints visible text while skipping HTML tags

## Run

Python:

```bash
python3 python/index.py http://example.org
```

Rust:

```bash
cd rust
cargo run -- http://example.org
```
