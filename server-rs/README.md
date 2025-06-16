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

To enable HTTPS specify `--https`. If `ssl.key` and `ssl.cert` don't exist they
will be generated automatically using a self–signed certificate:

```bash
cargo run -- --https
```

After starting, open `http://127.0.0.1:18888/api/hello` in your browser to check
that the server is running. The `/test` endpoint accepts a JSON payload
containing `timestamp` and `buffer` fields and echoes them back.

