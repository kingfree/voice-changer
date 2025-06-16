# Rust Server

This is a minimal Rust backend using `axum` on the `tokio` runtime.

## Running

Install Rust and then run:

```bash
cargo run
```

The server exposes:
- `GET /version` returning `0.1`.
- WebSocket endpoint `/ws/audio` for streaming audio data.

Incoming binary data on the WebSocket is logged to the console.
