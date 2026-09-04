# ssh-mcp-client

Personal desktop SSH client for managing your own VPS servers, built on Tauri (Rust + TypeScript). Runs an embedded MCP server so an AI assistant (Claude) can act on the same SSH sessions the GUI has open — with an audit log and a confirmation step before destructive commands.

**Status:** early development, not yet usable.

## Stack

- [Tauri v2](https://tauri.app/) — Rust backend, native webview frontend
- [russh](https://github.com/Eugeny/russh) — SSH client
- [rmcp](https://github.com/modelcontextprotocol/rust-sdk) — MCP server (official Rust SDK)
- [xterm.js](https://xtermjs.org/) — terminal emulator
- Vanilla TypeScript + Vite frontend, no framework

## Development

```bash
npm install
npm run tauri dev
```

More details (architecture, security model, build instructions) will land here as the project takes shape.
