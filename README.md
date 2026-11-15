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
npm audit --prefix frontend --audit-level=moderate
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace
```

The frontend temporarily overrides `typescript-estree`'s `minimatch` dependency to version 11 to address GHSA-g3hv-75wg-m249. Remove the override when upstream supports a patched version; the override crosses the upstream major-version range and must be rechecked when dependencies change.

See [CONTRIBUTING.md](CONTRIBUTING.md) for the development and disk-testing rules. Licensed under [MIT](LICENSE).
