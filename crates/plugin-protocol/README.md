# zerolaunch-plugin-protocol

JSON-RPC 2.0 protocol definitions for ZeroLaunch third-party plugins.

## What this crate provides

- **Message types** — all `plugin/*` and `host/*` RPC params/result structs (`messages.rs`)
- **Method name constants** — strongly-typed method name strings (`methods.rs`)
- **Manifest schema** — `manifest.toml` deserialization (`manifest.rs`)
- **Error codes** — JSON-RPC 2.0 standard + custom error codes (`error.rs`)
- **Envelope types** — `Request`, `Response`, `Notification` (`jsonrpc.rs`)

## Transport

The protocol uses LSP-style framed stdio:

```
Content-Length: 123\r\n
\r\n
{"jsonrpc":"2.0","id":1,"method":"plugin/query","params":{...}}
```

## Dependencies

- `zerolaunch-plugin-api` (data types)
- `serde` / `serde_json`
- `thiserror`
- `toml` / `semver` (manifest parsing)
