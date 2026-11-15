# mufus

## Project status and architecture

- Before working, read `.main-plan-docs.md` if available: it contains the detailed roadmap and security model. It is globally Git-ignored and may be absent in other clones; do not force-add it.
- The repository contains an initial Cargo workspace and Tauri/React shell with a preliminary read-only macOS disk check; device selection, privileged operations, and disk writing are not implemented. See `README.md` for setup and checks; later stages in the plan remain targets.
- The planned app is macOS-first, using Tauri 2, React, and TypeScript, with a UI-independent Rust core and a short-lived privileged helper. Work on the stage relevant to the request; do not present future features as implemented.
- Raw image writing copies an existing disk layout; it does not make an image bootable. Partition and filesystem options belong to a separate formatting mode, not the raw-write UI.

## Disk and privilege safety

- The app will erase disks: refuse writes when device identity or system topology is ambiguous. The helper must independently validate the target and the opened device; the GUI is not a trust boundary.
- A device path or number can be reused after unplugging; external transport or a removable flag alone is not proof of safety. Exclude system and boot devices and their dependencies; revalidate identity and topology immediately before writing, including the opened handle. Fail closed if this cannot be done reliably.
- Reject an image stored on the target device or its volumes, or when its location cannot be determined reliably. Check the actual image size against device capacity, use checked `u64` arithmetic, and enforce the boundary during writes.
- For compressed images, compressed size is not the write size. If the exact decompressed size is unknown, preflight by streaming a full decompression pass; ensure the source is unchanged between passes or refuse the operation.
- Do not enable physical writes before validating privilege elevation and a secure channel for the target OS. Tauri `externalBin` alone provides neither privilege elevation nor secure IPC; elevated launchers do not necessarily preserve stdin/stdout.
- Keep helper commands narrow and IPC versioned, bounded, and validated. Authenticate and restrict the local channel; do not pass secrets through arguments or logs, or allow arbitrary shell commands and destination paths.
- Cancellation, disconnect, short writes, flush errors, or failed verification must never be reported as success. After flushing, verify the exact written byte range with SHA-256; document platform cache limitations. Readback verifies the write, not image authenticity.
- Run automated and integration write tests only on temporary files or verified isolated virtual devices created by the test. A real USB drive is permitted only for separate, controlled manual acceptance; test-only paths must not bypass production target checks.

## Implementation and licensing

- Keep Tauri capabilities minimal and validate IPC data at runtime. Represent `u64` values crossing into JavaScript without loss of precision; TypeScript types alone do not validate external input.
- Isolate necessary platform `unsafe` code and document its invariants with `SAFETY` comments. Do not use crate-level `forbid(unsafe_code)` where platform modules must selectively permit it: `forbid` cannot be overridden.
- Do not read or copy Rufus, Ventoy, or proprietary UltraISO source code; use specifications and OS documentation. Verify licenses of dependencies, transitive packages, and separately distributed tools before including them; launching a GPL tool separately does not automatically resolve distribution obligations.
- Write code comments strictly in English; keep them concise, clear, and professional.

## Commits

- Commit subjects must be a single line in the format `[TYPE] - description;`, in imperative English, at most 130 characters, with no scope, quotes, Markdown, or text after `;`. Allowed types: `ADD`, `UPD`, `FIX`, `BUGFIX`, `FEATURE`, `REFACTOR`, `STYLE`, `LINT`, `RULE`, `CI`, `INIT`, `DOCS`, `TEST`.
- Describe the substantive change rather than accompanying versions, lockfiles, generated files, changelogs, README changes, or formatting; choose the type for code changes with tests based on the code. For version-only, lockfile-only, changelog-only, or `.gitignore`-only changes, use `[UPD] - bump version to X.Y.Z;`, `[UPD] - update lockfile;`, `[UPD] - bump CHANGELOG;`, or `[UPD] - bump gitignore;`, respectively.
