# server-rs

This directory contains a minimal Rust reimplementation of `MMVCServerSIO.py`.
It uses the `tokio` ecosystem and `axum` to provide HTTP endpoints similar to
the original Python server.

## Requirements

- Rust toolchain (edition 2021)

## Usage

```bash
cd server-rs
cargo run -- --port 18888 --host 127.0.0.1
```

After starting, open `http://127.0.0.1:18888/api/hello` in your browser to check
that the server is running. The `/test` endpoint accepts a JSON payload
containing `timestamp` and `buffer` fields and echoes them back.

