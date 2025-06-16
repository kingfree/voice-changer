# server-rs

This directory contains a minimal Rust reimplementation of `MMVCServerSIO.py`.
It uses the `tokio` ecosystem and `axum` to provide HTTP endpoints.  The server
mirrors the Python layout by exposing a REST API (`MMVC_Rest`) and a simple
WebSocket endpoint through `MMVC_SocketIOApp`.

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

Log output can be controlled with the `--log-level` flag (e.g. `info`, `debug`).

After starting, open `http://127.0.0.1:18888/api/hello` in your browser to check
that the server is running. The `/test` endpoint accepts a JSON payload
containing `timestamp` and `buffer` fields and echoes them back.

In addition a WebSocket echo service is available at `/ws`.

Static frontend files from `client/demo/dist` are served at `/`, `/front`,
`/trainer`, and `/recorder` along with auxiliary directories used by the
Python implementation.

The server initializes a `VoiceChangerManager` which will manage model
loading and settings updates. During startup required weight files and
initial sample models are downloaded automatically if they are missing,
mirroring the behaviour of the Python implementation.

