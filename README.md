# mufus

An experimental, unofficial desktop application for preparing boot media. The project is in its initial development stage: a preliminary read-only disk check is available on macOS; **device selection, privileged operations and disk writing are not implemented**.

## Development

Install Rust via rustup, Node.js 24 and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your platform. From the repository root:

```sh
npm ci --prefix frontend
npm run tauri --prefix frontend -- dev
```

The Rust toolchain is selected by `rust-toolchain.toml`. Checks:

```sh
npm run lint --prefix frontend
npm run format:check --prefix frontend
npm run test --prefix frontend
npm run build --prefix frontend
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for the development and disk-testing rules. Licensed under [MIT](LICENSE).
