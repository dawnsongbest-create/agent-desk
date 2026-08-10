# Agent Desk

Agent Desk is a local-first Tauri desktop surface for lightweight capture and Agent deliveries.

The repository is currently at **M1-A Foundation**. It contains the native shell, persistence and preference foundations, and a minimal Sticky surface. Note/Todo product UX, Inbox, Reader, Agent Gateway, notifications and Progressive Reading integration are intentionally deferred to later gates.

## Prerequisites

- Node.js 22.23.2 (see `.node-version`)
- pnpm 11
- Rust stable with `rustfmt` and `clippy`
- Tauri 2 platform prerequisites for Windows or macOS

## Checks

```text
pnpm install --frozen-lockfile
pnpm format
pnpm typecheck
pnpm lint
pnpm test
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --all-features
pnpm tauri build --no-bundle
```

Architecture decisions and Gate reports live in [`docs/`](docs/).
