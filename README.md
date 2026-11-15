# mufus

An experimental, unofficial desktop application for preparing boot media. The project is in its initial development stage: a preliminary read-only disk check is available on macOS; **device selection, privileged operations and disk writing are not implemented**.

The disk check uses `diskutil` and `bless --getBoot` to inspect the root and selected startup volumes. It requires their APFS stores to be identifiable; unavailable commands, unsupported layouts, inconsistent metadata, command output above 8 MiB, or a command that does not finish within 10 seconds cause the check to fail. A successful check does not establish complete boot topology or authorize disk writes.

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
cargo deny --locked --all-features check --hide-inclusion-graph
```

The frontend temporarily overrides `typescript-estree`'s `minimatch` dependency to version 11 to address GHSA-g3hv-75wg-m249. Remove the override when upstream supports a patched version; the override crosses the upstream major-version range and must be rechecked when dependencies change.

`cargo-deny` checks Rust advisories, dependency sources, licenses and duplicate versions; install it separately to run the check locally. The current Tauri dependency graph includes MPL-2.0 crates: distributing binaries requires reviewing and fulfilling their applicable license and source-availability obligations. The two documented GTK3 advisory exceptions in `deny.toml` must be revisited before Linux support and when upstream updates.

See [CONTRIBUTING.md](CONTRIBUTING.md) for the development and disk-testing rules. Licensed under [MIT](LICENSE).
